# Investigación — fase A (2026-11-11)

## Entorno medido

| Componente | Versión | Cómo se midió |
|---|---|---|
| PipeWire | 1.6.11 (`pipewire-devel` instalado) | `pipewire --version`, `rpm -q` |
| WirePlumber | 0.5.19 | `wireplumber --version` |
| BlueZ | 5.87 (sin `bluez-libs-devel`) | `bluetoothctl --version` |
| Kernel | 7.2.15-200.fc44 | entorno |
| Rust | cargo 1.99.1 | `cargo --version` |
| Plugins de códec SPA | aac, faststream, g722, hfp-*, lc3, ldac, opus, opus-g, sbc. **No** aptx ni lc3plus | `ls /usr/lib64/spa-0.2/bluez5/` |
| Herramientas | `btmon` sí; `adb`, `tshark`, `qemu-aarch64` no | `which` |

Auriculares: Buds3 Pro `E8:C9:13:E0:E6:DA`, conectados al PC por AAC.
Teléfono de referencia: Galaxy S22 Ultra, con SSC normal y **sin** SSC‑UHQ
(según la usuaria, por hardware). No hay emisor UHQ disponible.

## Repositorio de referencia `Vinchethescript/samsung-ssc-reversed`

Clonado en el scratchpad (commit `9f186fa`, 2024‑09‑23, un solo commit). No se
copia nada de él a este repositorio.

- `libScalable_Encoder.so`: ELF aarch64, Android API 28, NDK r20, sin símbolos
  de depuración pero con la tabla dinámica. Procedencia no documentada por el
  autor; se asume extraída de un firmware Samsung. **Propietario: no se
  redistribuye ni se enlaza.**
- `encoder.c` (5 647 líneas): salida de un descompilador retocada. El README
  dice que no compila ni funciona. No se ha intentado compilar: no aporta nada
  que la tabla de símbolos no diga ya, y es código derivado de Samsung que no
  puede entrar en BASC.

### Lo que sí dice la tabla de símbolos (observación verificada)

Los nombres exportados coinciden uno a uno con funciones de CELT (libopus):
`ec_enc_*`, `ec_laplace_encode`, `alg_quant`, `quant_all_bands`,
`compute_allocation`, `spreading_decision`, `stereo_itheta`, `haar1`,
`pitch_search`, `comb_filter`, `eMeans`… con el prefijo `celt_`/`opus_`
cambiado por `ssc_` (`ssc_encoder_create`, `ssc_custom_mode_create`,
`ssc_mdct_forward`, `ssc_lcg_rand`).

Añadidos sin equivalente en CELT: `cutoff_start_index`,
`cutoff_start_uhq_index`, `previous_uhq_index`, `uhq_headroom`,
`compute_band_energies_for_lastband`, `last_band_info_calc`,
`patch_transient_decision*`, `SoundBooster_DRC_Init`, `headroom_process`,
`Biquad_Narrow_int2int_stereo`, `coeffs96andbelow`.

### Hipótesis que se derivan (NO verificadas)

- H1: SSC es un CELT en «modo personalizado» (`custom_mode_create`) con
  codificador de rango compatible con el de Opus.
- H2: UHQ añade bandas por encima del corte normal (`cutoff_start_uhq_index`,
  `lastband`) — de ahí 96 kHz.
- H3: DRC/headroom son preproceso del PCM, no parte del flujo de bits.

Si H1 se confirma, el trabajo del codificador se reduce a encontrar los
parámetros del modo (frecuencia, tamaño de trama, bandas, `eMeans`) y la
cabecera de paquete, y se puede escribir un CELT propio en Rust contra la
especificación pública RFC 6716 §4.3. Esto no se puede confirmar sin
tramas reales: la descompilación no prueba qué emite el teléfono.

## Canal de control SPP: qué manda el S22 y qué manda `bookos-budsd` (2026‑11‑12)

Mensajes con formato `FD tam id datos crc DD`, extraídos de la captura E3 y
con CRC comprobado. Los nombres son los de GalaxyBudsClient
(`SppMessageEnums.cs`).

- **`MANAGER_INFO` (136):** el S22 manda `01 01 24`, es decir tipo de cliente
  1 = Samsung y SDK 36. `bookos-budsd` manda `01 02 22` (tipo 2 = otro, SDK 34).
  **Hipótesis sin probar:** el firmware podría ajustar algo según el tipo de
  cliente.
- El S22 también envía: `EQUALIZER` 00 (apagado), `SET_SPATIAL_AUDIO` 0000,
  `SPATIAL_AUDIO_CONTROL` 06, `GAME_MODE` 01 y luego 00, `SET_TOUCHPAD_OPTION`,
  Bixby, postura del cuello, `UPDATE_TIME`, `MULTIPOINT_INFO` y varios de
  depuración o versión.
- **Ninguno trata del códec, del bitrate ni de la calidad.** SSC se negocia
  solo por AVDTP. Lo que el S22 hace distinto con el sonido está antes del
  códec (DRC/headroom, +14 dB) o en el tipo de cliente.
- Durante las pruebas E4, `budsd` reabre su canal 2–3 s después de la
  reconexión A2DP y vuelve a aplicar EQ y ANC: la comparación AAC/BASC se hizo
  con los mismos ajustes del DSP.

## Integración en PipeWire 1.6.11 (leído del código fuente de la etiqueta 1.6.11)

- `spa/plugins/bluez5/codec-loader.c` carga los códecs de una **lista fija**
  de bibliotecas `bluez5/libspa-codec-bluez5-<nombre>`. No hay descubrimiento
  dinámico: un plugin con nombre nuevo no se carga sin parchear PipeWire.
- La interfaz es `struct media_codec` (`media-codecs.h`), versión
  `SPA_VERSION_BLUEZ5_CODEC_MEDIA = 16`. El cabecero no se instala con
  `pipewire-devel`: un plugin externo tiene que compilarse contra el árbol
  fuente de la misma versión exacta.
- `media_codec.id` es un `enum spa_bluetooth_audio_codec` cerrado; SSC no
  tiene valor propio.
- Los códecs de fabricante existentes (aptX, LDAC, Opus‑05) usan
  `codec_id = A2DP_CODEC_VENDOR` + `vendor = {vendor_id, codec_id}`: es el
  mismo mecanismo que necesita SSC.

Conclusión: la vía menos invasiva es un plugin `libspa-codec-bluez5-basc.so`
con un parche pequeño a PipeWire (una línea en el cargador + un valor del
enum + nombre). Para experimentar sin tocar el sistema existe una vía sin
parche: un directorio de plugins privado (`SPA_PLUGIN_DIR`) con el plugin
instalado bajo el nombre de un hueco libre (`aptx`, que Fedora no trae) y
reutilizando ese id. Solo para laboratorio; ver docs/architecture.md.
