# Arquitectura (2026‑11‑12)

```
aplicaciones → PipeWire (EQ de auriculares de BookOS) → WirePlumber/bluez5
   → libspa-codec-bluez5-aptx.so (spa-plugin/basc-ssc.c, C)
       → basc_encoder_* (crates/basc-core/src/ffi.rs, Rust)
           → Encoder (codec.rs): recorte + MARGEN −2 dB, cabecera SSC
               → CELT de libopus 1.6 adaptado (vendor/opus-celt, C, BSD)
   → BlueZ → A2DP (RTP + trama SSC de 484 octetos cada 18 ms) → Buds3 Pro
```

## Piezas y lenguaje

| Pieza | Lenguaje | Por qué |
|---|---|---|
| `vendor/opus-celt` | C | CELT de libopus probado en producción. Los 4 cambios de SSC van en `#ifdef BASC_SSC` (`BASC.patch`). Reescribirlo en Rust queda para después, comparando contra esta versión. |
| `crates/basc-core` | Rust | `Encoder`/`Decoder` seguros sobre libopus, cabecera SSC, capacidades y API C. Hay `unsafe` en la FFI con libopus y en `ffi.rs`, cada bloque con su `SAFETY`. |
| `spa-plugin/` | C | `struct media_codec` es una tabla de punteros a función de una cabecera interna de PipeWire, versionada (v16) y no instalada. En C contra las fuentes de 1.6.11 no hay que mantener a mano una réplica del ABI en Rust. El C solo empaqueta: RTP y llamadas a `basc_encoder_*`. |
| `crates/basc-snoop`, `crates/basc-wav`, `tools/` | Rust, sh, py | Investigación y pruebas: capturas, volcados, envío directo por BlueZ. |

## Integración

1. **Laboratorio (activo ahora): `tools/lab-pipewire.sh on|off`.** Un
   directorio privado de plugins con enlaces a los del sistema, más el de BASC
   con el nombre `aptx` (hueco libre en Fedora), y WirePlumber con
   `SPA_PLUGIN_DIR` apuntando ahí mediante un drop-in de systemd. Reutiliza
   `SPA_BLUETOOTH_AUDIO_CODEC_APTX` como id, pero PipeWire publica `.name`:
   el nodo sale con `api.bluez5.codec = "basc"` (antes "ssc"; renombrado por marca).
2. **Distribución (pendiente).** Parche a PipeWire: entrada en la lista de
   `codec-loader.c` y un valor `SPA_BLUETOOTH_AUDIO_CODEC_SSC` con su nombre.
   El plugin va aparte.

WirePlumber no necesita cambios. El perfil «BASC (compatible con SSC de Samsung)» aparece junto a AAC y
SBC con la prioridad más alta, y el respaldo AAC/SBC se mantiene: PipeWire
solo elige SSC si el SEP remoto lo anuncia y `select_config` lo acepta.

## Por qué en este orden

Primero E4 (reenviar tramas capturadas), luego el encoder y luego el plugin.
Cada paso se validó con hardware antes del siguiente, así que cuando algo
falla se sabe en qué capa está.
