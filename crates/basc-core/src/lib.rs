//! Núcleo de BASC: capacidades A2DP de SSC y codificador de tramas.
//!
//! Todo lo que hay aquí está verificado contra hardware real (docs/protocol.md
//! y docs/test-results.md).

pub mod caps;
pub mod codec;
pub mod ffi;
pub mod limitador;
