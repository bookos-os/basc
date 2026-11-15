//! Codificador SSC: CELT de libopus adaptado (vendor/opus-celt) más la
//! cabecera de 4 octetos de Samsung.
//!
//! Parámetros fijos, los medidos en el S22 Ultra (docs/protocol.md): 48 kHz,
//! estéreo, tramas de 864 muestras (18 ms) y 484 octetos en CBR (≈ 215 kbit/s).

use crate::limitador::Limitador;
use std::ffi::{c_int, c_uchar, c_void};
use thiserror::Error;

pub const FS: i32 = 48_000;
pub const MUESTRAS: usize = 864;
pub const CANALES: usize = 2;
pub const CABECERA: usize = 4;
pub const TRAMA: usize = 484;
const CARGA: usize = TRAMA - CABECERA;

// Peticiones de ctl de libopus (celt.h y opus_defines.h).
const CELT_SET_SIGNALLING_REQUEST: c_int = 10016;
const OPUS_SET_VBR_REQUEST: c_int = 4006;
const OPUS_SET_COMPLEXITY_REQUEST: c_int = 4010;
/// libopus pone 5 por omisión en modos personalizados; con 8 o más analiza
/// los transitorios con una segunda MDCT (celt_encoder.c, `secondMdct`).
const COMPLEJIDAD: c_int = 10;
/// −1 dB antes de codificar, para que la salida decodificada no pase de
/// 0 dBFS (el decodificador de los Buds es de coma fija y recorta). Medido el
/// 12-11 sin pre-énfasis: con −2 dB el pico de música masterizada a 0 dBFS
/// quedaba en −0,9/−1,0 dBFS. Samsung deja ≈ −0,6 dB (E5).
const MARGEN: f32 = 0.891_250_9; // 10^(−1/20)

#[repr(C)]
struct Modo(c_void);
#[repr(C)]
struct EncC(c_void);
#[repr(C)]
struct DecC(c_void);

extern "C" {
    fn opus_custom_mode_create(fs: i32, frame_size: c_int, error: *mut c_int) -> *mut Modo;
    fn opus_custom_mode_destroy(mode: *mut Modo);
    fn opus_custom_encoder_create(
        mode: *const Modo,
        channels: c_int,
        error: *mut c_int,
    ) -> *mut EncC;
    fn opus_custom_encoder_destroy(st: *mut EncC);
    fn opus_custom_encoder_ctl(st: *mut EncC, request: c_int, ...) -> c_int;
    fn opus_custom_encode_float(
        st: *mut EncC,
        pcm: *const f32,
        frame_size: c_int,
        compressed: *mut c_uchar,
        max_bytes: c_int,
    ) -> c_int;
    fn opus_custom_decoder_create(
        mode: *const Modo,
        channels: c_int,
        error: *mut c_int,
    ) -> *mut DecC;
    fn opus_custom_decoder_destroy(st: *mut DecC);
    fn opus_custom_decoder_ctl(st: *mut DecC, request: c_int, ...) -> c_int;
    fn opus_custom_decode_float(
        st: *mut DecC,
        data: *const c_uchar,
        len: c_int,
        pcm: *mut f32,
        frame_size: c_int,
    ) -> c_int;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("libopus no pudo crear el modo o el estado (código {0})")]
    Creacion(i32),
    #[error("libopus devolvió {0} al codificar; se esperaban {CARGA} octetos")]
    Codificacion(i32),
    #[error("trama inválida: {0}")]
    Trama(&'static str),
    #[error("libopus devolvió {0} al decodificar")]
    Decodificacion(i32),
}

/// Modo CELT de SSC. Lo comparten codificador y decodificador, y tiene que
/// sobrevivirles: libopus guarda un puntero a él.
struct ModoSsc(*mut Modo);

impl ModoSsc {
    fn nuevo() -> Result<Self, CodecError> {
        let mut err = 0;
        // SAFETY: argumentos por valor y un puntero de salida válido.
        let m = unsafe { opus_custom_mode_create(FS, MUESTRAS as c_int, &mut err) };
        if m.is_null() {
            return Err(CodecError::Creacion(err));
        }
        Ok(Self(m))
    }
}

impl Drop for ModoSsc {
    fn drop(&mut self) {
        // SAFETY: creado por opus_custom_mode_create; los estados que lo usan
        // se destruyen antes (orden de campos en Encoder/Decoder).
        unsafe { opus_custom_mode_destroy(self.0) }
    }
}

pub struct Encoder {
    st: *mut EncC,
    // Va detrás de `st` para que Drop lo libere después del estado.
    _modo: ModoSsc,
    contador: u8,
    limitador: Limitador,
}

impl Encoder {
    /// Sin ganancia: la señal sale al nivel de la fuente (menos MARGEN).
    pub fn nuevo() -> Result<Self, CodecError> {
        Self::con_ganancia(0.0)
    }

    /// Con `ganancia_db` de ganancia y limitador (ver limitador.rs). Añade una
    /// trama (18 ms) de retardo.
    pub fn con_ganancia(ganancia_db: f32) -> Result<Self, CodecError> {
        let modo = ModoSsc::nuevo()?;
        let mut err = 0;
        // SAFETY: el modo es válido y vive tanto como el Encoder.
        let st = unsafe { opus_custom_encoder_create(modo.0, CANALES as c_int, &mut err) };
        if st.is_null() {
            return Err(CodecError::Creacion(err));
        }
        // Sin el octeto de señalización de Opus (SSC no lo lleva) y en CBR:
        // el S22 manda siempre 484 octetos.
        // SAFETY: estado válido; las tres peticiones esperan un int.
        unsafe {
            opus_custom_encoder_ctl(st, CELT_SET_SIGNALLING_REQUEST, 0 as c_int);
            opus_custom_encoder_ctl(st, OPUS_SET_VBR_REQUEST, 0 as c_int);
            opus_custom_encoder_ctl(st, OPUS_SET_COMPLEXITY_REQUEST, COMPLEJIDAD);
        }
        Ok(Self {
            st,
            _modo: modo,
            contador: 0,
            limitador: Limitador::nuevo(ganancia_db),
        })
    }

    /// Codifica 18 ms de PCM entrelazado (L, R) en [-1, 1].
    pub fn codificar(
        &mut self,
        pcm: &[f32; MUESTRAS * CANALES],
    ) -> Result<[u8; TRAMA], CodecError> {
        // PipeWire trabaja en F32 sin recortar: con el volumen de una
        // aplicación por encima del 100 % o un EQ llegan muestras > 1.0, que
        // tras MARGEN volverían a pasar de 0 dBFS. libopus solo recorta en
        // ±2.0. Un NaN o un Inf envenenaría para siempre la memoria de los
        // filtros del encoder, así que se cambia por silencio.
        let limpia = pcm.map(|v| {
            if v.is_finite() {
                v.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        });
        let entrada = self.limitador.procesar(&limpia).map(|v| v * MARGEN);
        let mut trama = [0u8; TRAMA];
        // SAFETY: entrada tiene MUESTRAS*CANALES floats y la salida CARGA octetos.
        let n = unsafe {
            opus_custom_encode_float(
                self.st,
                entrada.as_ptr(),
                MUESTRAS as c_int,
                trama[CABECERA..].as_mut_ptr(),
                CARGA as c_int,
            )
        };
        if n != CARGA as c_int {
            return Err(CodecError::Codificacion(n));
        }
        trama[..CABECERA].copy_from_slice(&cabecera(self.contador));
        self.contador = (self.contador + 1) & 0x0f;
        Ok(trama)
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        // SAFETY: creado por opus_custom_encoder_create y no usado después.
        unsafe { opus_custom_encoder_destroy(self.st) }
    }
}

/// Cabecera de 4 octetos. Los valores fijos son los de las tramas normales de
/// E3 (`ff ee x1 f3`). Su semántica es parcial (docs/protocol.md): el nibble
/// bajo del octeto 2 lleva información de la última banda y los 3 bits altos
/// del octeto 3 el corte de ancho de banda; 0x13 en los 5 bajos = 484 octetos.
/// Con estos valores fijos los Buds3 Pro reproducen bien (prueba del 12-11).
fn cabecera(contador: u8) -> [u8; CABECERA] {
    [0xff, 0xee, contador << 4 | 0x1, 0xf3]
}

/// Decodificador de comprobación: el mismo CELT adaptado. No es el de los
/// Buds; sirve para verificar que las tramas son coherentes.
pub struct Decoder {
    st: *mut DecC,
    _modo: ModoSsc,
}

impl Decoder {
    pub fn nuevo() -> Result<Self, CodecError> {
        let modo = ModoSsc::nuevo()?;
        let mut err = 0;
        // SAFETY: como en Encoder::nuevo.
        let st = unsafe { opus_custom_decoder_create(modo.0, CANALES as c_int, &mut err) };
        if st.is_null() {
            return Err(CodecError::Creacion(err));
        }
        // SAFETY: estado válido; la petición espera un int.
        unsafe { opus_custom_decoder_ctl(st, CELT_SET_SIGNALLING_REQUEST, 0 as c_int) };
        Ok(Self { st, _modo: modo })
    }

    pub fn decodificar(&mut self, trama: &[u8]) -> Result<[f32; MUESTRAS * CANALES], CodecError> {
        let carga = match trama {
            [0xff, 0xee, _, _, carga @ ..] if !carga.is_empty() => carga,
            _ => return Err(CodecError::Trama("sin sincronía ff ee o vacía")),
        };
        let mut pcm = [0f32; MUESTRAS * CANALES];
        // SAFETY: carga válida durante la llamada; pcm tiene sitio para MUESTRAS.
        let n = unsafe {
            opus_custom_decode_float(
                self.st,
                carga.as_ptr(),
                carga.len() as c_int,
                pcm.as_mut_ptr(),
                MUESTRAS as c_int,
            )
        };
        if n != MUESTRAS as c_int {
            return Err(CodecError::Decodificacion(n));
        }
        Ok(pcm)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // SAFETY: creado por opus_custom_decoder_create y no usado después.
        unsafe { opus_custom_decoder_destroy(self.st) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Codifica y decodifica `tramas` de un seno de 1 kHz a -20 dBFS en un
    /// solo canal y devuelve la salida entrelazada.
    fn ida_y_vuelta(canal: usize, tramas: usize) -> (Vec<[u8; TRAMA]>, Vec<f32>) {
        let mut enc = Encoder::nuevo().expect("encoder");
        let mut dec = Decoder::nuevo().expect("decoder");
        let amp = 10f32.powf(-20.0 / 20.0);
        let (mut codificadas, mut salida) = (Vec::new(), Vec::new());
        for t in 0..tramas {
            let mut pcm = [0f32; MUESTRAS * CANALES];
            for (k, par) in pcm.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let n = (t * MUESTRAS + k) as f32;
                par[canal] = amp * (2.0 * PI * 1000.0 * n / FS as f32).sin();
            }
            let trama = enc.codificar(&pcm).expect("codificar");
            salida.extend_from_slice(&dec.decodificar(&trama).expect("decodificar"));
            codificadas.push(trama);
        }
        (codificadas, salida)
    }

    fn pico_db(x: impl Iterator<Item = f32>) -> f32 {
        20.0 * x.fold(1e-12f32, |m, v| m.max(v.abs())).log10()
    }

    #[test]
    fn cabecera_como_la_del_s22() {
        let (tramas, _) = ida_y_vuelta(0, 18);
        for (k, t) in tramas.iter().enumerate() {
            assert_eq!(t[..2], [0xff, 0xee]);
            assert_eq!(t[2], ((k as u8) & 15) << 4 | 1, "contador de 4 bits");
            assert_eq!(t[3], 0xf3);
        }
    }

    #[test]
    fn seno_izquierdo_conserva_nivel_y_canal() {
        let (_, pcm) = ida_y_vuelta(0, 40);
        // Se descartan las 10 primeras tramas: el decodificador arranca con retardo.
        let estable = &pcm[10 * MUESTRAS * CANALES..];
        let l = pico_db(estable.iter().step_by(2).copied());
        let r = pico_db(estable.iter().skip(1).step_by(2).copied());
        // -20 dBFS de entrada menos 1 dB de MARGEN.
        assert!(
            (l + 21.0).abs() < 0.5,
            "nivel izquierdo {l} dBFS, se esperaban -21"
        );
        assert!(r < -100.0, "fuga al derecho {r} dBFS");
    }

    #[test]
    fn graves_recortados_a_fondo_de_escala_no_pasan_de_0_dbfs() {
        // El peor caso medido: graves fuertes ya recortados (como la música
        // tras el EQ de BookOS). Sin MARGEN la salida llegaba a +1,15 dBFS.
        let mut enc = Encoder::nuevo().expect("encoder");
        let mut dec = Decoder::nuevo().expect("decoder");
        let mut pico = 0f32;
        for t in 0..60 {
            let mut pcm = [0f32; MUESTRAS * CANALES];
            for (k, par) in pcm.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                let n = (t * MUESTRAS + k) as f32 / FS as f32;
                let v = (2.0 * (2.0 * PI * 60.0 * n).sin() + 0.5 * (2.0 * PI * 3000.0 * n).sin())
                    .clamp(-1.0, 1.0);
                *par = [v, v];
            }
            let salida = dec
                .decodificar(&enc.codificar(&pcm).expect("codificar"))
                .expect("decodificar");
            pico = salida.iter().fold(pico, |m, v| m.max(v.abs()));
        }
        assert!(
            pico <= 1.0,
            "pico decodificado {:+.2} dBFS",
            20.0 * pico.log10()
        );
    }

    #[test]
    fn seno_derecho_conserva_nivel_y_canal() {
        let (_, pcm) = ida_y_vuelta(1, 40);
        let estable = &pcm[10 * MUESTRAS * CANALES..];
        let l = pico_db(estable.iter().step_by(2).copied());
        let r = pico_db(estable.iter().skip(1).step_by(2).copied());
        assert!(
            (r + 21.0).abs() < 0.5,
            "nivel derecho {r} dBFS, se esperaban -21"
        );
        assert!(l < -100.0, "fuga al izquierdo {l} dBFS");
    }

    #[test]
    fn silencio_produce_trama_de_silencio_como_la_del_s22() {
        let mut enc = Encoder::nuevo().expect("encoder");
        let trama = enc
            .codificar(&[0f32; MUESTRAS * CANALES])
            .expect("codificar");
        // Trama de silencio capturada al S22: ff ee x1 f3 ff fe y 478 ceros.
        assert_eq!(trama[4..6], [0xff, 0xfe]);
        assert!(trama[6..].iter().all(|&b| b == 0));
    }

    #[test]
    fn nan_y_sobrecarga_no_estropean_el_encoder() {
        // Un NaN o muestras a ±3.0 (aplicación al 300 %) en una trama no
        // pueden dejar al encoder produciendo basura en las siguientes.
        let mut enc = Encoder::nuevo().expect("encoder");
        let mut dec = Decoder::nuevo().expect("decoder");
        let mut mala = [3.0f32; MUESTRAS * CANALES];
        mala[0] = f32::NAN;
        mala[1] = f32::INFINITY;
        let salida = dec
            .decodificar(&enc.codificar(&mala).expect("codificar"))
            .expect("decodificar");
        assert!(salida.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        // Después, silencio: tiene que volver a ser silencio.
        let mut ultima = [1f32; MUESTRAS * CANALES];
        for _ in 0..20 {
            ultima = dec
                .decodificar(
                    &enc.codificar(&[0f32; MUESTRAS * CANALES])
                        .expect("codificar"),
                )
                .expect("decodificar");
        }
        assert!(
            ultima.iter().all(|v| v.abs() < 1e-3),
            "el encoder quedó contaminado"
        );
    }

    #[test]
    fn rechaza_trama_sin_sincronia() {
        let mut dec = Decoder::nuevo().expect("decoder");
        assert_eq!(
            dec.decodificar(&[0u8; TRAMA]),
            Err(CodecError::Trama("sin sincronía ff ee o vacía"))
        );
    }
}
