# TASKS — BASC

## Estado (2026‑11‑12)

- **Formato SSC entendido de principio a fin.** Es CELT con 4 cambios. Las
  tramas reales del S22 se decodifican alineadas (docs/protocol.md).
- **Encoder BASC funcionando:** `basc-core::codec` (libopus BSD adaptada,
  llamada desde Rust) y `basc-wav`. Los Buds3 Pro reproducen bien sus tramas
  (docs/test-results.md).
- **Plugin de PipeWire funcionando en laboratorio:** con
  `tools/lab-pipewire.sh on`, los Buds muestran «SSC (BASC)» y todo el audio
  del escritorio sale por él. `off` lo deshace.
- Revisión externa (Gemini) aplicada el 12-11: ver «Revisión» más abajo.

## Cómo probar

```sh
tools/lab-pipewire.sh on                      # BASC en la sesión (off para quitarlo)
tools/basc-reproducir.sh [--eq] canción.flac  # un archivo, por BlueZ directo
```

`e4-prueba.sh` para WirePlumber, envía, lo arranca otra vez y reconecta A2DP.
`capturas/e3-senal.wav` se genera con `tools/senal-e3.py`. `capturas/` no se
versiona.

## Hecho

- [x] Auditoría de `samsung-ssc-reversed`: el núcleo es CELT, y `encoder.c`
      no sirve ni se puede reutilizar.
- [x] PipeWire 1.6.11 / BlueZ 5.87: el cargador de códecs es una lista cerrada.
- [x] E1: capacidades de los Buds (`75 00 00 00 03 01 3E`).
- [x] E2/E3: capturas del S22 por adb; la señal está alineada con las tramas.
- [x] E4: el PC negocia SSC y los Buds reproducen las tramas del S22.
- [x] Formato: rango de 24 bits, 22 bandas, tabla de reparto propia, sin
      tapset y cabecera de 4 octetos. Medido con Ghidra y validado: 1 261 de
      1 276 tramas reales acaban alineadas.
- [x] Encoder: el prototipo y luego `basc-core::codec`, idéntico octeto a
      octeto. Los Buds lo reproducen bien. La trama de silencio es idéntica a
      la del S22.

## Siguiente

1. [ ] Confirmar de oído BASC sin pre-énfasis frente a AAC (nivel, graves y
       «EQ»), y que no hay «pip» con el margen de −1 dB.
2. [ ] Pruebas largas con el plugin: horas de reproducción, reconexiones,
       cambio de perfil SSC↔AAC, enlace degradado. Mirar el CPU de
       WirePlumber.
2b. [ ] Distribución: parche a PipeWire (`codec-loader.c` + id propio) en
       vez del hueco `aptx`.
3. [ ] Las 7 tramas reales que acaban con `error = 1` (protocol.md).
4. [ ] Cabecera: semántica completa del nibble bajo del octeto 2 y de los
       bits altos del octeto 3. Ahora van fijos (`x1`, `f3`) y funcionan.
5. [ ] Tramas de 324 octetos (`f2`): el S22 baja de tamaño cuando el enlace
       empeora. Hace falta para la adaptación de bitrate.
6. [ ] Nivel: el S22 suena unos 14 dB más fuerte con el mismo WAV (DRC o
       headroom propios). Comparar de oído y decidir si BASC lo imita.
7. [ ] UHQ / 96 kHz: bloqueado sin un emisor UHQ de referencia.

## Revisión externa (Gemini, 2026‑11‑12)

Aplicado: recorte a [−1, 1] y NaN/Inf → 0 antes de codificar (con test);
FFI con comprobación de NULL y lectura no alineada; `opus_custom_mode_create`
rechaza todo lo que no sea 48 kHz/864 en compilación SSC y `modes.c` usa la
condición exacta; `basc-snoop` resta marcas con saturación y libera los CID
al desconectar L2CAP; `basc-wav` comprueba el SubFormat de
WAVE_FORMAT_EXTENSIBLE; `build.rs` vigila todo `vendor/opus-celt`; los
scripts entrecomillan, limpian y no se cuelgan con pocas tramas; documentación
al día.

Descartado tras comprobarlo: (4) `caps_size < sizeof` es lo que hacen todos
los plugins A2DP de PipeWire 1.6.11; (§1.4) el nodo se publica como
`api.bluez5.codec` con el `.name` del códec, no `"aptx"` (visto con pactl). Pánico a través de
`extern "C"`: desde Rust 1.81 aborta sin UB, y `codificar` no tiene caminos
con pánico.

## Problemas encontrados

- BlueZ (`a2dp_reconfig`) devuelve EPERM si otro emisor D-Bus (PipeWire) tiene
  el flujo abierto. Por eso se para WirePlumber durante las pruebas.
- Tras reiniciar WirePlumber o tras una prueba, los Buds se quedaban solo en
  HFP; `lab-pipewire.sh` y `e4-prueba.sh` reabren A2DP.
- (Antes) Tras la prueba, WirePlumber solo encontraba HFP y los widgets se quedaban en
  «esperando dispositivo». Lo arregla la reconexión A2DP de `e4-prueba.sh`.
- El clasificador de permisos bloquea `./configure` de tarballs descargados:
  se compila con `gcc`/`cc` directo.
- El registro HCI del S22 incluye el audio aunque el offload esté activo. Se
  saca con `adb bugreport` (`FS/data/log/bt/btsnoop_hci.log`).
