# Protocolo SSC — especificación de trabajo

Cada dato lleva su estado: **V** verificado con hardware propio, **H** hipótesis
con fundamento, **D** desconocido. Nada marcado H o D se usa en código que
pretenda interoperar.

## Matriz

| # | Elemento | Estado | Valor / fuente | Experimento para cerrarlo |
|---|---|---|---|---|
| 1 | Tipo de códec A2DP | **V** | `0xFF` (no‑A2DP) | — |
| 2 | Vendor ID | **V** | `0x00000075` (Samsung) | — |
| 3 | Codec ID SSC | **V** | `0x0103` | — |
| 4 | Octetos propios de capacidad | **V** valor / **D** significado | `0x3E` (1 octeto) | E2: SET_CONFIGURATION del S22 |
| 5 | Codec ID SSC‑UHQ | **D** | Los Buds3 Pro anuncian un único SEP Samsung; UHQ podría ser un bit de `0x3E` o un SEP aparte que solo aparece tras alguna orden | E2 + E5 |
| 6 | Configuración elegida por el teléfono | **D** | | E2 |
| 7 | Cabecera RTP | **H** | A2DP manda RTP de 12 octetos para todos los códecs conocidos | E2 |
| 8 | Cabecera de carga tras RTP | **D** | | E2/E3 |
| 9 | Duración de trama | **D** | El salto de marca RTP la dará | E2 |
| 10 | Bitrate y adaptación | **D** | `basc-snoop` da kbit/s y tamaños | E2 con enlace bueno y degradado |
| 11 | Núcleo del códec | **H** | CELT modificado (research.md, H1) | E3 |
| 12 | Orden de octetos de la trama | **D** | | E3 |
| 13 | Requisitos 96 kHz / UHQ | **D** | Sin emisor UHQ disponible (S22 Ultra no lo soporta) | E5 |
| 14 | Mensajes fuera de AVDTP para habilitar UHQ | **D** | Posible canal RFCOMM/SPP del plugin Wearable | E5 |

Capacidades completas medidas el 2026‑11‑11 (D‑Bus,
`org.bluez.MediaEndpoint1.Capabilities` de los SEP remotos):

| SEP | Códec | Octetos |
|---|---|---|
| sep1 | SBC (0x00) | `3f ff 02 34` |
| sep2 | AAC (0x02) | `80 01 8c 83 e8 00` — MPEG‑2/4 AAC‑LC, 44,1/48 kHz, estéreo, VBR, 256 kbit/s |
| sep3 | Fabricante (0xFF) | `75 00 00 00 03 01 3e` |

`0x3E = 0b0011_1110`. No se interpreta: en otros códecs de fabricante el primer
octeto suele ser una máscara de frecuencias, pero suponerlo aquí sería inventar.

## Emisor de referencia: S22 Ultra (SM-S908B, Android 16, S908BXXSOGZH3)

Medido por `adb shell dumpsys bluetooth_manager` el 2026‑11‑11:

- **V** SSC es el códec tipo 7 de la pila de Samsung, con prioridad 8001, la
  más alta; aptX tiene 3001.
- **V** El historial registra la conexión con los Buds como `SSC HIFI`.
- **V** `A2dpOffloadEnabled: true`, y SSC figura en `codecConfigOffloading`.
  Con offload el DSP codifica y entrega las tramas directamente al
  controlador, así que **el registro HCI no las contendría**. Solo vería la
  señalización. Para E2 hay que desactivar la descarga por hardware.
- **V** `/system/lib64/libScalable_Encoder.so` existe en el teléfono, así que
  hay un codificador software. Que se use con el offload desactivado es
  **H**.

## Captura E2 (2026‑11‑11, S22 Ultra → Buds3 Pro, `capturas/btsnoop_hci.log`)

Pese a `A2dpOffloadEnabled: true`, el registro HCI de Samsung **sí incluye los
paquetes de medios**. No sé si la opción de desactivar el offload llegó a
aplicarse.

Negociación, vista con `basc-snoop`:

- DISCOVER: SEID 1, 2 y 3, todos libres y de tipo sink. SEID 3 es el de
  Samsung.
- **SET_CONFIGURATION** al SEID 3: `75 00 00 00 03 01 0C`. Es decir, el
  teléfono responde `0x0C` a una capacidad `0x3E`. Encaja con una máscara de
  bits de la que se eligen dos (`0x08 | 0x04`), pero es **H**.
- Categorías de servicio en la configuración: transporte de medios (`01 00`),
  códec (`07`) y delay reporting (`08 00`). Sin protección de contenido.

Medios, 2 783 paquetes (**V**):

| Campo | Valor medido |
|---|---|
| RTP | Cabecera de 12 octetos, versión 2. La marca de tiempo sube **1 por paquete** (cuenta paquetes, no muestras) |
| Carga útil | **484 octetos exactos** en todos los paquetes (CBR) |
| Cadencia | 55,58 paquetes/s en 45,3 s de reproducción continua, que es 48 000/864 = 55,56. Una trama por paquete: `ff ee` solo aparece fijo en la posición 0 |
| Bitrate | 484 × 8 × 55,56 ≈ **215 kbit/s** |
| Octetos 0–1 | `ff ee` siempre. Sincronía (**H**) |
| Octeto 2 | Nibble alto: contador módulo 16. Nibble bajo: `1` constante |
| Octeto 3 | `f3` en todas las tramas |
| Octetos 4… | Dependen del contenido |
| Trama de silencio | `ff ee x1 f3 ff fe` seguido de 478 ceros |

Hipótesis derivadas:

- **H** Trama de 864 muestras a 48 kHz (18 ms). El 48 kHz no está
  verificado: 44,1 kHz daría 793,75 muestras, que no es un número entero,
  así que 48 kHz es lo más probable.
- **H** Tramas de señal consecutivas comparten prefijo, por ejemplo
  `a3 72 f6 4a f3 58 66 24 6…`. Es lo que se espera de un codificador de rango
  con energías gruesas estables, como CELT. Refuerza H1, pero no la prueba.
- **D** Qué tramo de la captura corresponde a `e3-senal.wav`. Hay 540
  paquetes (9,7 s) de silencio y luego 1 660 (29,9 s) de señal. No encaja
  limpio con los 20 s del archivo.

## H1, primera comprobación (2026‑11‑11)

`tools/rangedec.py` es un decodificador de rango escrito desde RFC 6716 §4.1.
Lo apliqué a las 1 276 tramas de E3 empezando en el octeto 4 y decodificando
`bit_logp(15)`, el indicador de silencio de CELT:

- Tramas de silencio: 1 en 441/441.
- Tramas de señal: 0 en 835/835.
- Empezando en el octeto 2 o en el 3: 0 en todas.

**V**: la cabecera propia de SSC son 4 octetos (`ff ee cN f3`). Detrás va un
flujo del codificador de rango de CELT/Opus, y su primer símbolo es el
indicador de silencio. Siguiente: decodificar post‑filtro, transitorio, intra
y energía gruesa, y deducir de ahí LM y el número de bandas.

## Formato CELT de SSC (2026‑11‑12)

Se ha obtenido así: tablas de datos y constantes leídas de
`libScalable_Encoder.so`, el orden de símbolos leído en la descompilación
(sin copiar código) y todo contrastado con las tramas de E3. Arnés: libopus
1.6 (BSD) compilada con trazas en el scratchpad (`traza/`).

| Elemento | Estado | Valor |
|---|---|---|
| Inicio del codificador de rango | **V** (código) | `ec_enc_init(buf+4, n−4)`, con n = 484 o 324 |
| `eMeans` | **V** | Idéntica a CELT en coma fija (25 valores) |
| Bandas | **V** | 22 bandas: `0 1 2 3 4 5 6 7 8 10 12 14 16 20 24 28 34 40 48 60 72 84 102` (×8 bins). CELT tiene 21 (…60 78 100) |
| `band_allocation` | **V** | 11 filas × 22 propias de SSC (en research de la sesión; se regenera desde el binario) |
| `e_prob_model` | **V** | Solo LM3. 23 parejas: las 21 de CELT más `130,17,125,19` (inter) y `68,45,62,49` (intra). Se indexa con `2·min(i,20)`, así que las parejas extra no se usan |
| Predicción de energía | **V** (asm) | coef. 16384 / beta 6554 (inter), 0 / 4915 (intra); límite −9; bucle banda→canal; Laplace estándar (`0x7fe0` = 32768 − 2·16) |
| Postfiltro | **V** | Igual que CELT pero **sin `tapset`** |
| Tamaño de trama | **V** | 864 = 8 × 108; LM = 3 |
| Cabecera, octeto 2 | **V** (código) | Nibble alto = contador; bits bajos = información de la última banda (energía por encima de un umbral, suma 0/4/8/12) |
| Cabecera, octeto 3 | **V** (código) | 3 bits altos = corte de ancho de banda (escala de 8 niveles); 5 bits bajos = tamaño (`f3` → 484, `f2` → 324) |
| `cutoff_start_index` | **V** | Bins 708 / 744 / 780 / 816 (19,7–22,7 kHz) dentro de la última banda |

| **Codificador de rango** | **V** (Ghidra + tramas) | **`EC_CODE_BITS = 24`**, no 32: renormaliza con `rng < 0x8001`, acarreo con `val >> 15 == 0xff`, máscara `0x7fff`. Era lo que desalineaba todo tras el primer símbolo |

### Primera decodificación correcta (2026‑11‑12)

libopus 1.6 con cinco cambios: rango de 24 bits, 22 bandas, `band_allocation`
de SSC, `e_prob_model` de 23 parejas y postfiltro sin `tapset`. Con eso, las
1 276 tramas de E3 dan:

| Tramo | Resultado |
|---|---|
| Silencio | −240 dBFS |
| Seno izquierdo | 1000 Hz solo en L; R a −126 dBFS |
| Seno derecho | 1000 Hz solo en R; L a −126 dBFS |
| Nivel del seno | **−5,7 dBFS frente a −20 de la fuente: +14,3 dB sin explicar** |
| SNR del seno | **41 dB: bajo para 215 kbit/s, quedan diferencias en etapas finales** |

**Prueba de alineación completa (V):** en tramas CBR, un decodificador bien
alineado termina con `ec_tell` justo en el borde de la trama. Resultados:

- **1 261 de 1 276** tramas reales terminan con 0–1 bits sobrantes y
  `ec_get_error = 0`.
- Las que sobran mucho son de silencio, como corresponde.
- **7 tramas** terminan con `error = 1`. Pendiente de mirar.

Con esto, el flujo completo (energías, reparto, PVQ y estéreo) se lee igual
que lo escribió el S22.

Más comprobaciones con Ghidra: `quant_fine_energy`, `quant_energy_finalise`,
`ec_enc_bits` (ventana de 32) y `ec_enc_uint` (8 bits) son estándar de CELT en
coma fija. `amp2Log2` es equivalente a la de libopus (`E<<2` en vez de
`+2.0`).

Referencia: libopus modificada con este formato, codificando el seno a 480 B,
da −20,3 dBFS (nivel correcto) y una SNR de 54 dB. Con las tramas del S22,
42 dB. La diferencia es del encoder de Samsung, que es de coma fija y lleva
DRC, no del formato.

Pendiente: (1) el origen de los +14 dB. Puede ser la ganancia de
`headroom`/DRC del emisor o una escala del decodificador; hay que comparar con
`headroom_gain` y `margin`. (2) La SNR: revisar `quant_fine_energy`,
`quant_energy_finalise`, `compute_allocation` y `alg_quant` frente a
libopus, buscando el mismo tipo de cambio (constantes de precisión).

## Pre-énfasis: SSC no la lleva (E5, 2026‑11‑12)

E5: la misma canción (40 s, la de las pruebas de BASC) reproducida en el S22
por adb y capturada por HCI. Las tramas decodificadas con libopus (con su
des-énfasis 1/(1 − 0,85·z⁻¹)) frente a la fuente dan exactamente la curva de
esa des-énfasis: +16 dB en graves, +12,2 a 1–2 kHz, +8,0 a 2–4 kHz y −5,5 a
16–20 kHz, con la misma ganancia para cualquier nivel (sin compresión).
Decodificadas **sin** des-énfasis: plana entre −0,5 y −0,7 dB (−1,1 dB en
16–20 kHz).

**V:** SSC no aplica pre-énfasis y los Buds no deshacen ninguna. Samsung no
ecualiza ni comprime; solo deja ≈ 0,6 dB de margen. Los «+14 dB» del tono de
1 kHz en E3, que atribuí a un DRC, eran esto (des-énfasis a 1 kHz = +14,3
dB). BASC aplicaba la pre-énfasis de libopus (0,85), así que en los Buds
sonaba con 16 dB menos de graves, más agudos y más bajo. Es el quinto cambio
de `BASC.patch`.

## Experimentos

**E1 — capacidades del receptor.** Hecho (tabla de arriba). Repetible con
`tools/leer-capacidades.sh`.

**E2 — captura de referencia S22 Ultra → Buds3 Pro.** Procedimiento en
TASKS.md. Da: SET_CONFIGURATION (fila 4–6), cabecera RTP y de carga (7–8),
duración de trama (9), bitrate (10) y las tramas reales.

**E3 — tramas con señal conocida.** En la misma captura, reproducir en el
teléfono: 5 s de silencio digital, 5 s de seno de 1 kHz a −20 dBFS solo en
canal izquierdo, 5 s igual en el derecho. Silencio → aísla cabecera y campos
fijos. Seno → permite probar si el flujo se decodifica con un decodificador
CELT de modo personalizado (H1). Izquierda/derecha → codificación estéreo.

**E4 — interoperabilidad mínima.** Reenviar a los Buds desde el PC las tramas
capturadas en E2/E3, tal cual, sobre un transporte SSC negociado por BASC. Si
suena el seno, quedan validados transporte, empaquetado y negociación con
independencia del codificador. Es la prueba que separa los dos problemas.

**E5 — UHQ.** Sin emisor UHQ no hay referencia. Opciones: un teléfono Samsung
compatible prestado, o capturas publicadas por terceros con su procedencia.
Mientras tanto, UHQ queda fuera.
