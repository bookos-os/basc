//! Extrae de un registro btsnoop (HCI snoop de Android o `btmon -w`) la
//! negociación AVDTP y la carga útil de los paquetes de medios.
//!
//! Es la herramienta de la fase B: con una captura del S22 Ultra enviando SSC
//! a los Buds3 Pro obtenemos la configuración elegida (SET_CONFIGURATION) y
//! las tramas reales que tendremos que saber generar.
//!
//! Uso: basc-snoop <captura.btsnoop> [volcado.bin]
//!
//! Formato del volcado, por paquete y todo LE:
//! u64 µs relativos · u16 secuencia RTP · u32 marca RTP · u32 longitud · carga.

use anyhow::{bail, ensure, Context, Result};
use std::collections::HashMap;
use std::io::Write;

const PSM_AVDTP: u16 = 0x0019;
const CATEGORIA_CODEC: u8 = 0x07;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Sentido {
    Enviado,
    Recibido,
}

#[derive(Clone, Copy)]
enum Canal {
    Senalizacion,
    Medios,
}

struct Registro<'a> {
    ts_us: i64,
    sentido: Sentido,
    datos: &'a [u8],
}

fn registros(buf: &[u8]) -> Result<Vec<Registro<'_>>> {
    ensure!(
        buf.len() >= 16 && &buf[..8] == b"btsnoop\0",
        "no es un archivo btsnoop"
    );
    let enlace = u32::from_be_bytes(buf[12..16].try_into().expect("4 octetos"));
    // 1002 = HCI UART (H4), el que usan Android y btmon. 1001 lleva el tipo en
    // las banderas; no lo hemos visto en la práctica y no se soporta.
    ensure!(
        enlace == 1002,
        "tipo de enlace btsnoop {enlace} no soportado (se espera 1002/H4)"
    );

    let mut out = Vec::new();
    let mut resto = &buf[16..];
    while let Some((cab, tras)) = resto.split_first_chunk::<24>() {
        let be32 = |i: usize| u32::from_be_bytes(cab[i..i + 4].try_into().expect("4 octetos"));
        let incluidos = be32(4) as usize;
        let banderas = be32(8);
        let ts_us = i64::from_be_bytes(cab[16..24].try_into().expect("8 octetos"));
        let Some(datos) = tras.get(..incluidos) else {
            break;
        };
        let sentido = if banderas & 1 == 0 {
            Sentido::Enviado
        } else {
            Sentido::Recibido
        };
        out.push(Registro {
            ts_us,
            sentido,
            datos,
        });
        resto = &tras[incluidos..];
    }
    Ok(out)
}

/// Estado del análisis: reensamblado ACL y canales L2CAP de AVDTP por conexión.
#[derive(Default)]
struct Analisis {
    fragmentos: HashMap<(u16, Sentido), (usize, Vec<u8>)>,
    peticiones: HashMap<(u16, u8), (u16, u16)>,
    canales: HashMap<(u16, u16), Canal>,
    medios: Vec<(i64, u16, u32, Vec<u8>)>,
}

impl Analisis {
    fn acl(&mut self, ts: i64, sentido: Sentido, acl: &[u8]) {
        let Some((cab, datos)) = acl.split_first_chunk::<4>() else {
            return;
        };
        let h = u16::from_le_bytes([cab[0], cab[1]]);
        let (handle, pb) = (h & 0x0fff, (h >> 12) & 3);
        let clave = (handle, sentido);
        if pb == 0b01 {
            let Some((esperado, b)) = self.fragmentos.get_mut(&clave) else {
                return;
            };
            b.extend_from_slice(datos);
            if b.len() < *esperado {
                return;
            }
        } else {
            let Some(len) = datos
                .get(..2)
                .map(|l| u16::from_le_bytes([l[0], l[1]]) as usize)
            else {
                return;
            };
            self.fragmentos.insert(clave, (len + 4, datos.to_vec()));
            if datos.len() < len + 4 {
                return;
            }
        }
        let (_, trama) = self
            .fragmentos
            .remove(&clave)
            .expect("fragmento recién completado");
        self.l2cap(ts, handle, &trama);
    }

    fn l2cap(&mut self, ts: i64, handle: u16, t: &[u8]) {
        let Some(cab) = t.first_chunk::<4>() else {
            return;
        };
        let len = u16::from_le_bytes([cab[0], cab[1]]) as usize;
        let cid = u16::from_le_bytes([cab[2], cab[3]]);
        let Some(carga) = t.get(4..4 + len) else {
            return;
        };
        if cid == 0x0001 {
            self.senalizacion_l2cap(handle, carga);
            return;
        }
        match self.canales.get(&(handle, cid)) {
            Some(Canal::Senalizacion) => avdtp(ts, carga),
            Some(Canal::Medios) => self.rtp(ts, carga),
            None => {}
        }
    }

    fn senalizacion_l2cap(&mut self, handle: u16, mut c: &[u8]) {
        while let Some((cab, resto)) = c.split_first_chunk::<4>() {
            let (codigo, id) = (cab[0], cab[1]);
            let len = u16::from_le_bytes([cab[2], cab[3]]) as usize;
            let Some(d) = resto.get(..len) else { return };
            let u16le = |i: usize| d.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
            match codigo {
                0x02 => {
                    if let (Some(psm), Some(scid)) = (u16le(0), u16le(2)) {
                        self.peticiones.insert((handle, id), (psm, scid));
                    }
                }
                0x03 => {
                    // Resultado 0 = éxito; 1 = pendiente, llegará otra respuesta.
                    if let (Some(dcid), Some(0)) = (u16le(0), u16le(4)) {
                        if let Some((PSM_AVDTP, scid)) = self.peticiones.remove(&(handle, id)) {
                            // AVDTP abre primero el canal de señalización y luego el de
                            // medios; si ya hay uno de señalización vivo, este es de medios.
                            let hay_senal = self.canales.iter().any(|(&(h, _), c)| {
                                h == handle && matches!(c, Canal::Senalizacion)
                            });
                            let canal = if hay_senal {
                                Canal::Medios
                            } else {
                                Canal::Senalizacion
                            };
                            // Cada extremo escribe al CID del otro: hay que reconocer ambos.
                            self.canales.insert((handle, dcid), canal);
                            self.canales.insert((handle, scid), canal);
                        }
                    }
                }
                0x06 => {
                    // Disconnection Request: los CID quedan libres y pueden reutilizarse
                    // en una reconexión AVDTP dentro de la misma captura.
                    for cid in [u16le(0), u16le(2)].into_iter().flatten() {
                        self.canales.remove(&(handle, cid));
                    }
                }
                _ => {}
            }
            c = &resto[len..];
        }
    }

    fn rtp(&mut self, ts: i64, p: &[u8]) {
        let Some(cab) = p.first_chunk::<12>() else {
            return;
        };
        if cab[0] >> 6 != 2 {
            return;
        }
        let csrc = (cab[0] & 0x0f) as usize * 4;
        let seq = u16::from_be_bytes([cab[2], cab[3]]);
        let marca = u32::from_be_bytes([cab[4], cab[5], cab[6], cab[7]]);
        if let Some(carga) = p.get(12 + csrc..) {
            self.medios.push((ts, seq, marca, carga.to_vec()));
        }
    }
}

fn avdtp(ts: i64, s: &[u8]) {
    let (Some(&b0), Some(&b1)) = (s.first(), s.get(1)) else {
        return;
    };
    if (b0 >> 2) & 3 != 0 {
        println!("{ts} AVDTP fragmentado, no soportado: {}", hex(s));
        return;
    }
    let tipo = match b0 & 3 {
        0 => "orden",
        2 => "aceptada",
        3 => "rechazada",
        _ => "general",
    };
    let senal = b1 & 0x3f;
    let nombre = match senal {
        0x01 => "DISCOVER",
        0x02 => "GET_CAPABILITIES",
        0x03 => "SET_CONFIGURATION",
        0x04 => "GET_CONFIGURATION",
        0x05 => "RECONFIGURE",
        0x06 => "OPEN",
        0x07 => "START",
        0x08 => "CLOSE",
        0x09 => "SUSPEND",
        0x0a => "ABORT",
        0x0c => "GET_ALL_CAPABILITIES",
        0x0d => "DELAYREPORT",
        _ => "?",
    };
    println!("{ts} AVDTP {nombre} {tipo}: {}", hex(&s[2..]));
    let caps = match (senal, b0 & 3) {
        (0x02 | 0x0c, 2) => s.get(2..),
        (0x03, 0) => s.get(4..),
        (0x05, 0) => s.get(3..),
        _ => None,
    };
    if let Some(caps) = caps {
        codecs(caps);
    }
}

fn codecs(mut c: &[u8]) {
    while let [cat, len, resto @ ..] = c {
        let len = *len as usize;
        let Some(d) = resto.get(..len) else { return };
        if *cat == CATEGORIA_CODEC && d.len() >= 2 {
            let info = &d[2..];
            print!("    códec tipo 0x{:02x}: {}", d[1], hex(info));
            if d[1] == basc_core::caps::A2DP_CODEC_VENDOR {
                match basc_core::caps::parse_ssc(info) {
                    Ok(ssc) => print!("  ← SSC, octetos propios {}", hex(ssc.especificos)),
                    Err(e) => print!("  ({e})"),
                }
            }
            println!();
        }
        c = &resto[len..];
    }
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn resumen(medios: &[(i64, u16, u32, Vec<u8>)]) {
    let Some(primero) = medios.first() else {
        println!("Sin paquetes de medios. ¿Captura en modo «filtrado» o sin reproducción?");
        return;
    };
    let tamanos: Vec<usize> = medios.iter().map(|m| m.3.len()).collect();
    let media = tamanos.iter().sum::<usize>() as f64 / tamanos.len() as f64;
    // Las marcas vienen de un archivo externo: pueden ir desordenadas o estar corruptas.
    let dur_s = medios.last().expect("no vacío").0.saturating_sub(primero.0) as f64 / 1e6;
    println!(
        "\n{} paquetes de medios en {dur_s:.1} s; carga mín {} / media {media:.1} / máx {}; ~{:.0} kbit/s",
        medios.len(),
        tamanos.iter().min().expect("no vacío"),
        tamanos.iter().max().expect("no vacío"),
        tamanos.iter().sum::<usize>() as f64 * 8.0 / dur_s.max(1e-9) / 1000.0
    );
    // El salto de la marca RTP entre paquetes da muestras por paquete: es la
    // pista más directa de la duración de trama.
    let mut saltos: HashMap<u32, usize> = HashMap::new();
    for par in medios.windows(2) {
        *saltos.entry(par[1].2.wrapping_sub(par[0].2)).or_default() += 1;
    }
    let mut saltos: Vec<_> = saltos.into_iter().collect();
    saltos.sort_by_key(|s| std::cmp::Reverse(s.1));
    println!(
        "saltos de marca RTP más frecuentes (salto × veces): {:?}",
        &saltos[..saltos.len().min(6)]
    );
    println!("primeros octetos de carga de los 8 primeros paquetes:");
    for m in medios.iter().take(8) {
        println!(
            "    seq {} ts {} len {}: {}",
            m.1,
            m.2,
            m.3.len(),
            hex(&m.3[..m.3.len().min(16)])
        );
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let Some(ruta) = args.get(1) else {
        bail!("uso: basc-snoop <captura.btsnoop> [volcado.bin]")
    };
    let buf = std::fs::read(ruta).with_context(|| format!("leyendo {ruta}"))?;
    let mut a = Analisis::default();
    for r in registros(&buf)? {
        if let [0x02, acl @ ..] = r.datos {
            a.acl(r.ts_us, r.sentido, acl);
        }
    }
    resumen(&a.medios);
    if let Some(destino) = args.get(2) {
        let base = a.medios.first().map_or(0, |m| m.0);
        let mut f = std::io::BufWriter::new(std::fs::File::create(destino)?);
        for (ts, seq, marca, carga) in &a.medios {
            f.write_all(&(ts.saturating_sub(base).max(0) as u64).to_le_bytes())?;
            f.write_all(&seq.to_le_bytes())?;
            f.write_all(&marca.to_le_bytes())?;
            f.write_all(&(carga.len() as u32).to_le_bytes())?;
            f.write_all(carga)?;
        }
        f.flush()?;
        println!("volcado escrito en {destino}");
    }
    Ok(())
}
