# Resultados de pruebas

| Nivel | Prueba | Estado | Fecha | Evidencia |
|---|---|---|---|---|
| 1 | `cargo build`, `cargo test` (3 tests de `caps`), `clippy`, `fmt` | Superada | 2026‑11‑11 | local |
| 2 | Detección de Buds3 Pro y lectura de capacidades A2DP | Superada | 2026‑11‑11 | protocol.md, E1 |
| 2 | Identificación de SSC (vendor 0x75, códec 0x0103) | Superada (solo el anuncio) | 2026‑11‑11 | protocol.md |
| 2 | Identificación de SSC‑UHQ | No ejecutada — sin referencia | | |
| 2 | Negociación SSC iniciada desde el PC (BlueZ + endpoint propio, config `0x0C`) | Superada | 2026‑11‑11 23:39 | `tools/e4-reenvio.py`: SelectConfiguration → SetConfiguration `sep3/fd0`, Acquire con MTU de escritura 1024 |
| 3 | E4: los Buds reproducen tramas SSC **del S22** reenviadas desde el PC | Superada (transporte y negociación; **no** el codificador) | 2026‑11‑11 23:41 | Evelyn oyó, sin cortes: silencio, seno a la izquierda, seno a la derecha y barrido. 1 276 tramas en 22,97 s |
| 2 | `basc-snoop` sobre una captura real | Superada | 2026‑11‑11 | E2: negociación completa y 2 783 tramas |
| 2 | Configuración SSC elegida por el emisor de referencia | Medida (`0x0C`) | 2026‑11‑11 | protocol.md |
| 3 | Decodificar las tramas reales del S22 con libopus modificada | Superada | 2026‑11‑12 | 1 261/1 276 tramas terminan alineadas; 1 kHz por canal, separación > 120 dB |
| 3 | **Tramas de un encoder independiente** (libopus BSD + formato SSC, prototipo del scratchpad) **reproducidas en los Buds3 Pro** | **Superada** | 2026‑11‑12 00:33 | Evelyn lo oyó bien: silencio, L, R y barrido. 1 111 tramas en 20,00 s. Verificación previa: −20,3 dBFS y 0 errores |
| 3 | El audio del escritorio vuelve tras la prueba | Fallo, corregido | 2026‑11‑12 | Quedaba solo HFP. Ahora `tools/e4-prueba.sh` reconecta A2DP al terminar |
| 3 | **Música real** (40 s de un FLAC, 48 kHz/24 bit) codificada por `basc-wav` y reproducida en los Buds | **Superada** | 2026‑11‑12 00:42 | Evelyn: «se escucha bien, tal vez no tan bien como en el teléfono». 2 222 tramas, 0 errores al decodificar |
| 3 | Restaurar A2DP tras la prueba | Falló una vez (quedó HFP); arreglado con `pactl set-card-profile` | 2026‑11‑12 | `e4-prueba.sh` |
| 5 | Coste del encoder (complejidad 10) | Medido | 2026‑11‑12 | 40 s de música en 0,80 s (≈ 2 % de un núcleo, ×50 tiempo real), 29 MB de pico con la lectura del WAV |
| — | Complejidad 5 → 10 | SNR segmental +0,12 dB (24,75 → 24,87, mediana) | 2026‑11‑12 | Se queda en 10 por la segunda MDCT en transitorios; la SNR no recoge la mejora perceptiva |
| 3 | «Menos graves» que AAC (Evelyn) | Explicado | 2026‑11‑12 | La prueba mandaba el WAV en bruto; AAC pasa por el EQ de auriculares de BookOS. Con el mismo EQ, BASC conserva los graves: ±0,01 dB entre 20 y 400 Hz (decodificado frente a entrada) |
| 3 | «Pip» en los fuertes (Evelyn) | Causa probable corregida; **pendiente de confirmar de oído** | 2026‑11‑12 | La salida de CELT llegaba a +1,15 dBFS (22 046 muestras > 0 dBFS). Con MARGEN de −2 dB: 0 muestras, pico −0,77 dBFS. Envío puntual: p99 0,3 ms |
| 3 | Cortes en vivo con el plugin (Evelyn) | **Causa fuera de BASC, corregida** | 2026‑11‑12 | xruns del filtro de EQ de auriculares de BookOS (`atmos`), con `node.latency` 240/48000: +13 a +19 en 18 s, **igual con AAC que con BASC**. Pasado a 1024/48000 para destinos Bluetooth (`atmos/GPUI/src-native/auriculares.rs`): **0 xruns en 18 s**; el filtro usa el 3 % del ciclo |
| — | Nombre visible | Cambiado | 2026‑11‑12 | «BASC (compatible con SSC de Samsung)», `api.bluez5.codec = "basc"` (marca: legal-review.md) |
| 3 | «AAC más alto y con otro EQ» (Evelyn: BASC al 70 % ≈ AAC al 50 %) | **Causa encontrada y corregida** | 2026‑11‑12 | E5: SSC no lleva pre-énfasis y BASC sí (libopus, 0,85). Las tramas del S22 sin des-énfasis reproducen la fuente plana (±0,7 dB). BASC sin pre-énfasis y con margen −1 dB: pico +0,09 dBFS en 1–2 muestras. **Evelyn: «suenan igual»** (AAC y BASC al 50 %) |
| 4–5 | Alta calidad, estabilidad | No ejecutadas | | |
