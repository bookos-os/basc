//! API C del codificador, para el plugin SPA de PipeWire (spa-plugin/).
//!
//! El plugin no reimplementa nada: llama a este mismo `Encoder`, que es el que
//! se ha validado con los Buds. Cabecera C equivalente en spa-plugin/basc.h.

use crate::codec::{Encoder, CANALES, MUESTRAS, TRAMA};

/// Crea un codificador. Devuelve NULL si libopus no pudo inicializarse.
///
/// La ganancia sale de `BASC_GANANCIA_DB` (0 si no está o no es un número):
/// PipeWire no pasa parámetros a un códec, y así se ajusta desde el entorno de
/// WirePlumber (tools/lab-pipewire.sh). Se limita a 0–20 dB.
#[no_mangle]
pub extern "C" fn basc_encoder_new() -> *mut Encoder {
    let db = std::env::var("BASC_GANANCIA_DB")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .map_or(0.0, |v| v.clamp(0.0, 20.0));
    Encoder::con_ganancia(db).map_or(std::ptr::null_mut(), |e| Box::into_raw(Box::new(e)))
}

/// Codifica una trama: `pcm` apunta a 864×2 floats entrelazados y `salida` a
/// 484 octetos. Devuelve 484, o -1 si algún puntero es NULL o libopus falló.
///
/// Ninguno de los dos punteros necesita estar alineado: media-sink.c de
/// PipeWire pasa su búfer tras avanzar por octetos.
///
/// # Safety
/// `enc` viene de `basc_encoder_new` y no se ha liberado; `pcm` y `salida`
/// son válidos para los tamaños indicados y no se solapan.
#[no_mangle]
pub unsafe extern "C" fn basc_encoder_encode(
    enc: *mut Encoder,
    pcm: *const f32,
    salida: *mut u8,
) -> i32 {
    if enc.is_null() || pcm.is_null() || salida.is_null() {
        return -1;
    }
    // SAFETY: no nulos y con el tamaño garantizado por quien llama; la copia
    // no alineada evita crear una referencia a f32 posiblemente desalineada.
    let (enc, entrada) = unsafe {
        (
            &mut *enc,
            pcm.cast::<[f32; MUESTRAS * CANALES]>().read_unaligned(),
        )
    };
    match enc.codificar(&entrada) {
        Ok(trama) => {
            // SAFETY: `salida` tiene TRAMA octetos (contrato); u8 no exige alineación.
            unsafe { std::ptr::copy_nonoverlapping(trama.as_ptr(), salida, TRAMA) };
            TRAMA as i32
        }
        Err(_) => -1,
    }
}

/// Libera el codificador. Admite NULL.
///
/// # Safety
/// `enc` viene de `basc_encoder_new` y no se usa después.
#[no_mangle]
pub unsafe extern "C" fn basc_encoder_free(enc: *mut Encoder) {
    if !enc.is_null() {
        // SAFETY: creado con Box::into_raw en basc_encoder_new.
        drop(unsafe { Box::from_raw(enc) });
    }
}
