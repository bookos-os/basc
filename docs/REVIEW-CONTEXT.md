# BASC: contexto para revisión de código

Documento para quien revise el código, persona o modelo, **antes** de leerlo.
Fecha: 2026‑11‑12. Idioma del proyecto: castellano (código, comentarios,
commits).

## Qué es

BASC (*BookOS Adaptive Sound Codec*) es una implementación independiente del
códec Bluetooth **SSC de Samsung** para Linux, con la que un PC envía audio a
unos **Galaxy Buds3 Pro** como lo hace un móvil Samsung. Hardware de pruebas:
Galaxy Book5 Pro (Fedora 44, kernel 7.2, PipeWire 1.6.11, WirePlumber
0.5.19, BlueZ 5.87), Buds3 Pro `E8:C9:13:E0:E6:DA` y un Galaxy S22 Ultra como
emisor de referencia.

## Estado: qué funciona y está probado con hardware

- Los Buds anuncian SSC como códec de fabricante: Samsung `0x0075`, códec
  `0x0103`, capacidad `0x3E`. El S22 elige la configuración `0x0C`.
- **SSC es CELT (el núcleo de Opus) con 4 cambios.** Detalle y evidencia en
  `docs/protocol.md`:
  1. codificador de rango de **24 bits** (`EC_CODE_BITS 24`; Opus usa 32);
  2. **22 bandas** propias (`0…60 72 84 102`, MDCT corta de 108 bins);
  3. tabla de reparto de bits propia, de 11 × 22;
  4. postfiltro **sin** el campo `tapset`.

  Además, cada trama lleva una cabecera de 4 octetos (`ff ee` + contador +
  tamaño), y va una trama de 484 octetos por paquete RTP, cada 18 ms (864
  muestras a 48 kHz), en CBR.
- Validación del formato: con esos cambios, las tramas reales capturadas al
  S22 se decodifican bien. 1 261 de 1 276 terminan alineadas al bit, y los
  tonos de prueba salen en su canal y frecuencia.
- El encoder (libopus 1.6 modificada, llamada desde Rust) suena en los Buds:
  tonos y música. Evelyn lo ha escuchado.
- El plugin de PipeWire (`spa-plugin/`) carga en modo laboratorio. Los Buds
  muestran el perfil «SSC (BASC)», PipeWire lo elige y el audio del
  escritorio sale por él.

## Mapa del código

| Ruta | Qué es | Lenguaje |
|---|---|---|
| `crates/basc-core/src/codec.rs` | `Encoder` y `Decoder` (FFI a libopus), cabecera SSC, `MARGEN` de −2 dB, tests | Rust |
| `crates/basc-core/src/ffi.rs` | API C (`basc_encoder_new/encode/free`) para el plugin | Rust |
| `crates/basc-core/src/caps.rs` | Parseo de las capacidades A2DP de fabricante | Rust |
| `crates/basc-core/build.rs` | Compila `vendor/opus-celt` con `cc` y `-DBASC_SSC` | Rust |
| `crates/basc-snoop` | Lee btsnoop: AVDTP y RTP, y vuelca las tramas | Rust |
| `crates/basc-wav` | WAV → volcado de tramas SSC | Rust |
| `vendor/opus-celt/` | Subconjunto CELT de libopus 1.6 (BSD). Cambios en `#ifdef BASC_SSC`, recogidos en `BASC.patch` | C |
| `spa-plugin/basc-ssc.c` | Códec para el `media_codec` de PipeWire 1.6.11 (`SPA_VERSION_BLUEZ5_CODEC_MEDIA` 16) | C |
| `spa-plugin/pipewire-1.6.11/` | Cabeceras internas copiadas de PipeWire (MIT) | C |
| `spa-plugin/exports.map` | Solo exporta `spa_handle_factory_enum` y `spa_log_topic_enum` | ld |
| `tools/lab-pipewire.sh` | Activa o desactiva el plugin en WirePlumber (drop-in de systemd + `SPA_PLUGIN_DIR`) | sh |
| `tools/e4-reenvio.py`, `e4-prueba.sh`, `basc-reproducir.sh` | Envían un volcado a los Buds por D-Bus/BlueZ, sin PipeWire | py/sh |
| `tools/rangedec.py`, `senal-e3.py` | Decodificador de rango de investigación y generador de la señal de prueba | py |

## Cómo compilar y probar

```sh
cargo build --release && cargo test && cargo clippy --all-targets   # 9 tests en basc-core
make -C spa-plugin                                                     # plugin de laboratorio
tools/lab-pipewire.sh on    # activar en la sesión; «off» lo deshace
```

Las pruebas con hardware necesitan los Buds conectados. `capturas/` (las
capturas HCI y los WAV) no se versiona.

## Reglas del proyecto (no son fallos, son decisiones)

- **Ni una línea de código de Samsung**, tampoco reescrita. Del binario
  `libScalable_Encoder.so` solo se sacaron datos de interoperabilidad: tablas,
  una constante y el orden de los campos. Base legal: Directiva 2009/24/CE
  art. 6. Si ves algo que parezca derivado de código de Samsung, **avisa: es
  el fallo más grave posible.**
- El reparto C/Rust es intencionado (decidido con Evelyn): libopus probada
  ahora, Rust alrededor.
- Sin dependencias nuevas sin consultarlo. Errores con `thiserror` en
  bibliotecas y `anyhow` en binarios, sin `unwrap()`, y `expect()` solo con un
  invariante local.

## Dónde mirar con lupa (por orden de riesgo)

1. **`spa-plugin/basc-ssc.c` dentro de WirePlumber.** Un fallo aquí tumba el
   audio de toda la sesión.
   - `codec_encode`: ¿puede PipeWire llamar con `src_size` mayor o menor que
     un bloque? Ahora se exige ≥ 6912 octetos y se consume uno exacto.
   - `codec_start_encode`: la marca de tiempo RTP cuenta paquetes, como el
     S22, y no muestras. ¿Rompe algo de `media-sink.c` (cálculo de retardo,
     resincronización)?
   - Sin `abr_process`, `reduce_bitpool` ni `get_interval`: ¿qué pasa cuando
     el enlace se congestiona? Hoy no se baja de 484 octetos.
   - `.id = SPA_BLUETOOTH_AUDIO_CODEC_APTX` es un apaño de laboratorio. ¿Hay
     sitios de PipeWire o WirePlumber que deriven el nombre o el
     comportamiento del id y no de `.name`?
   - `codec_get_delay` devuelve 108 muestras, medidas con el decodificador
     propio, no con los Buds.
2. **`crates/basc-core/src/ffi.rs`**: `unsafe` con casts de puntero a array.
   Revisar alineación, tamaños, `Send`/`Sync` (el `Encoder` lleva punteros
   crudos y no debe cruzar hilos) y doble liberación.
3. **`codec.rs`**: orden de destrucción `st` → `_modo`, ya que libopus guarda
   un puntero al modo. `MARGEN` multiplica la entrada sin `clamp`: ¿llega
   alguna vez algo por encima de 1.0 desde PipeWire? Cabecera con valores
   fijos `x1` y `f3`.
4. **`vendor/opus-celt/BASC.patch`**: los `if (1)` de `modes.c` fuerzan las
   tablas de SSC en cualquier modo. Comprobar que no rompe nada fuera de
   48 kHz/864, aunque solo se usa ese.
5. **`tools/lab-pipewire.sh`**: borra y rehace
   `~/.local/share/basc/spa-0.2`; revisar que no pueda tocar nada fuera de
   ahí.
6. **`basc-snoop`**: parseo de entrada no confiable (capturas). Buscar
   desbordamientos o índices sin comprobar.

## Ya revisado (Gemini, 2026‑11‑12)

Aplicados y descartados en `TASKS.md`, sección «Revisión externa». No hace
falta volver a reportarlos.

## Problemas conocidos (no hace falta reportarlos)

- 7 de las 1 276 tramas reales terminan con `ec_get_error = 1` (sin
  investigar).
- Solo hay un tamaño de trama: falta el de 324 octetos (`f2`) que usa el S22
  con enlace malo.
- Con el mismo WAV, el S22 suena unos 14 dB más fuerte porque lleva su propio
  DRC.
- UHQ / 96 kHz: sin emisor de referencia; no se ha hecho nada.
- `basc-snoop` no reensambla la señalización AVDTP fragmentada.
- Pendiente de confirmar de oído que el `MARGEN` de −2 dB elimina los «pip».

## Documentación

`docs/protocol.md` (formato y evidencia), `docs/research.md`,
`docs/architecture.md`, `docs/test-results.md`, `docs/limitations.md`,
`docs/legal-review.md` y `TASKS.md`.
