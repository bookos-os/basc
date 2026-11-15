//! Codifica un WAV (PCM 16 o 24 bit, estéreo, 48 kHz) en tramas SSC y las
//! escribe con el formato de volcado de basc-snoop, para mandarlas a los Buds
//! con tools/e4-prueba.sh.
//!
//! Uso: basc-wav <entrada.wav> <salida.bin> [ganancia_dB]

use anyhow::{bail, ensure, Context, Result};
use basc_core::codec::{Encoder, CANALES, FS, MUESTRAS};
use std::io::Write;

/// Devuelve las muestras entrelazadas normalizadas a [-1, 1].
fn leer_wav(d: &[u8]) -> Result<Vec<f32>> {
    ensure!(
        d.len() >= 12 && &d[..4] == b"RIFF" && &d[8..12] == b"WAVE",
        "no es un WAV RIFF"
    );
    let (mut formato, mut datos) = (None, None);
    let mut resto = &d[12..];
    while let Some((cab, tras)) = resto.split_first_chunk::<8>() {
        let tam = u32::from_le_bytes([cab[4], cab[5], cab[6], cab[7]]) as usize;
        let cuerpo = tras.get(..tam).context("trozo RIFF truncado")?;
        match &cab[..4] {
            b"fmt " => formato = Some(cuerpo),
            b"data" => datos = Some(cuerpo),
            _ => {}
        }
        // Los trozos RIFF se alinean a 2 octetos.
        resto = tras.get(tam + (tam & 1)..).unwrap_or(&[]);
    }
    let fmt = formato.context("falta el trozo fmt")?;
    let datos = datos.context("falta el trozo data")?;
    ensure!(fmt.len() >= 16, "trozo fmt corto");
    let u16le = |i: usize| u16::from_le_bytes([fmt[i], fmt[i + 1]]);
    let tipo = u16le(0);
    let canales = u16le(2) as usize;
    let fs = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
    let bits = u16le(14);
    // 0xFFFE = WAVE_FORMAT_EXTENSIBLE: el formato real va en los dos primeros
    // octetos del SubFormat (fmt[24..26]); 1 = PCM entero, 3 = float.
    let pcm_entero = match tipo {
        1 => true,
        0xFFFE => fmt.get(24..26) == Some(&[1, 0]),
        _ => false,
    };
    ensure!(pcm_entero, "formato WAV {tipo:#x}: solo PCM entero");
    ensure!(canales == CANALES, "{canales} canales: SSC aquí es estéreo");
    ensure!(
        fs == FS as u32,
        "{fs} Hz: hay que remuestrear a 48 kHz antes"
    );
    let muestras = match bits {
        16 => datos
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&b| i16::from_le_bytes(b) as f32 / 32768.0)
            .collect(),
        24 => datos
            .as_chunks::<3>()
            .0
            .iter()
            .map(|b| (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0)
            .collect(),
        _ => bail!("{bits} bits por muestra: solo 16 o 24"),
    };
    Ok(muestras)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (entrada, salida, ganancia) = match args.as_slice() {
        [_, e, s] => (e, s, 0.0),
        [_, e, s, g] => (e, s, g.parse::<f32>().context("ganancia en dB")?),
        _ => bail!("uso: basc-wav <entrada.wav> <salida.bin> [ganancia_dB]"),
    };
    let pcm = leer_wav(&std::fs::read(entrada).with_context(|| format!("leyendo {entrada}"))?)?;
    let mut enc = Encoder::con_ganancia(ganancia)?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(salida)?);
    let mut n: u32 = 0;
    // La última trama incompleta se descarta; son menos de 18 ms.
    for bloque in pcm.as_chunks::<{ MUESTRAS * CANALES }>().0 {
        let trama = enc.codificar(bloque)?;
        // Formato de volcado de basc-snoop: µs, secuencia, marca RTP, longitud, trama.
        f.write_all(&(u64::from(n) * 18_000).to_le_bytes())?;
        f.write_all(&(n as u16).to_le_bytes())?;
        f.write_all(&n.to_le_bytes())?;
        f.write_all(&(trama.len() as u32).to_le_bytes())?;
        f.write_all(&trama)?;
        n += 1;
    }
    f.flush()?;
    println!("{n} tramas SSC ({:.2} s) en {salida}", f64::from(n) * 0.018);
    Ok(())
}
