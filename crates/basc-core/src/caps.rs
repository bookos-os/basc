//! Capacidades A2DP del códec de fabricante de Samsung.
//!
//! Estructura de la cabecera (A2DP v1.3, §4.7.2, códec no-A2DP 0xFF):
//! Vendor ID (4 octetos, LE) + Codec ID (2 octetos, LE) + datos propios.
//!
//! Valores medidos el 2026-11-11 leyendo `org.bluez.MediaEndpoint1.Capabilities`
//! del SEP remoto de unos Buds3 Pro: `75 00 00 00 03 01 3E`.
//! El significado de los bits del octeto propio (0x3E) es DESCONOCIDO; por eso
//! se expone en bruto y no como banderas con nombre.

use thiserror::Error;

/// Tipo de códec A2DP "no-A2DP" (fabricante).
pub const A2DP_CODEC_VENDOR: u8 = 0xFF;
/// Samsung Electronics Co. Ltd. en el registro de Company Identifiers del SIG.
pub const SAMSUNG_VENDOR_ID: u32 = 0x0000_0075;
/// Codec ID anunciado por los Buds3 Pro (medido).
pub const SSC_CODEC_ID: u16 = 0x0103;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CapsError {
    #[error("capacidades de {0} octetos; la cabecera de fabricante necesita 6")]
    Corta(usize),
    #[error("fabricante 0x{vendor:08x} / códec 0x{codec:04x}, no es SSC de Samsung")]
    OtroCodec { vendor: u32, codec: u16 },
}

/// Capacidades SSC tal como llegan del SEP remoto.
#[derive(Debug, PartialEq, Eq)]
pub struct SscCaps<'a> {
    /// Octetos posteriores a la cabecera. Semántica sin determinar.
    pub especificos: &'a [u8],
}

pub fn parse_vendor_header(caps: &[u8]) -> Result<(u32, u16, &[u8]), CapsError> {
    let (cab, resto) = caps
        .split_first_chunk::<6>()
        .ok_or(CapsError::Corta(caps.len()))?;
    let vendor = u32::from_le_bytes([cab[0], cab[1], cab[2], cab[3]]);
    let codec = u16::from_le_bytes([cab[4], cab[5]]);
    Ok((vendor, codec, resto))
}

pub fn parse_ssc(caps: &[u8]) -> Result<SscCaps<'_>, CapsError> {
    match parse_vendor_header(caps)? {
        (SAMSUNG_VENDOR_ID, SSC_CODEC_ID, especificos) => Ok(SscCaps { especificos }),
        (vendor, codec, _) => Err(CapsError::OtroCodec { vendor, codec }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacidades_medidas_en_buds3_pro() {
        let caps = [0x75, 0x00, 0x00, 0x00, 0x03, 0x01, 0x3E];
        assert_eq!(
            parse_ssc(&caps),
            Ok(SscCaps {
                especificos: &[0x3E]
            })
        );
    }

    #[test]
    fn rechaza_otro_fabricante() {
        // Cabecera de aptX (APT Ltd 0x4F, códec 0x0001), para comprobar el orden LE.
        let caps = [0x4F, 0x00, 0x00, 0x00, 0x01, 0x00, 0xFF];
        assert_eq!(
            parse_ssc(&caps),
            Err(CapsError::OtroCodec {
                vendor: 0x4F,
                codec: 1
            })
        );
    }

    #[test]
    fn rechaza_cabecera_corta() {
        assert_eq!(parse_ssc(&[0x75, 0, 0]), Err(CapsError::Corta(3)));
    }
}
