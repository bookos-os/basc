# BASC

**BASC** es un codificador de audio Bluetooth libre para Linux (BookOS) que
permite enviar audio desde un PC a auriculares Samsung Galaxy Buds usando el
mismo formato de transmisión que emplean los teléfonos Samsung.

> **BASC no es un producto de Samsung**, no está afiliado, patrocinado,
> respaldado ni revisado por Samsung Electronics Co., Ltd. ni por ninguna de
> sus filiales.

## Aviso sobre marcas

«Samsung», «Galaxy», «Galaxy Buds», «Samsung Seamless Codec» y «SSC» son
marcas o nombres comerciales de Samsung Electronics Co., Ltd. Aquí se citan
**solo para describir con qué es compatible BASC** (uso descriptivo y
nominativo). El nombre del proyecto, del códec en PipeWire
(`api.bluez5.codec = "basc"`) y del perfil visible («BASC (compatible con SSC
de Samsung)») se eligieron para que no se confunda con el producto de Samsung.

## Qué es

- Un **codificador independiente**, construido sobre CELT de
  [libopus](https://opus-codec.org/) 1.6 (licencia BSD de 3 cláusulas, de
  Xiph.Org y los demás autores de Opus), con cinco cambios marcados con
  `#ifdef BASC_SSC` y recogidos en `vendor/opus-celt/BASC.patch`.
- Una capa en Rust (`crates/basc-core`) que envuelve ese codificador, genera
  la cabecera de trama y expone una API C.
- Un plugin SPA para el soporte Bluetooth de PipeWire (`spa-plugin/`), de
  momento solo para laboratorio.
- Herramientas de investigación y prueba (`crates/basc-snoop`,
  `crates/basc-wav`, `tools/`).

## Qué no es

- **No contiene código de Samsung**, ni copiado ni reescrito, ni binarios de
  Samsung. No enlaza ni redistribuye `libScalable_Encoder.so` ni ninguna otra
  biblioteca propietaria.
- No es una implementación oficial ni certificada de SSC. No garantiza
  funcionar con todos los auriculares ni con versiones futuras de firmware.
- No implementa SSC‑UHQ (96 kHz) ni la adaptación de bitrate.
- No es Opus conforme a la RFC 6716: el formato de trama difiere a propósito
  para poder interoperar.

## Para qué sirve

Para que un usuario de Linux pueda escuchar el audio de **su propio** PC en
**sus propios** Galaxy Buds con el códec que esos auriculares ya admiten, en
lugar de quedar limitado a SBC o AAC. Es un trabajo de **interoperabilidad**.

## Cómo se hizo

El formato se dedujo con fines de interoperabilidad, al amparo del art. 6 de
la Directiva 2009/24/CE (art. 100 de la Ley de Propiedad Intelectual en
España), observando el tráfico Bluetooth entre dispositivos propios
(teléfono y auriculares de la autora) y estudiando el comportamiento del
codificador del teléfono. De ese estudio solo pasan a BASC **datos de
interoperabilidad**: dos tablas numéricas (bordes de banda y reparto de
bits), una precisión de 24 bits del codificador de rango, la ausencia de un
campo y el formato de la cabecera de 4 octetos. El detalle está en
[`docs/legal-review.md`](docs/legal-review.md) y
[`docs/protocol.md`](docs/protocol.md).

## Con qué funciona

Probado únicamente con:

| Componente | Versión |
|---|---|
| Auriculares | Galaxy Buds3 Pro |
| Emisor de referencia (para medir) | Galaxy S22 Ultra, Android 16 |
| Sistema | Fedora 44 (BookOS), kernel 7.x |
| PipeWire / WirePlumber | 1.6.11 / 0.5.19 |
| BlueZ | 5.87 |
| Rust | 1.99 |

Modo único: 48 kHz, estéreo, tramas CBR de 484 octetos cada 18 ms
(≈ 215 kbit/s). Otros modelos de Buds pueden funcionar o no; no se han
probado.

## Compilar y probar

Requisitos: Rust, un compilador de C, `pipewire-devel` y `pkg-config`.

```sh
cargo build --release
cargo test

make -C spa-plugin                            # plugin SPA de laboratorio
tools/lab-pipewire.sh on                      # activa BASC en la sesión (off lo deshace)
tools/basc-reproducir.sh [--eq] canción.flac  # reproduce un archivo por BlueZ directo
```

El plugin de laboratorio ocupa el hueco `aptx` del cargador de códecs de
PipeWire mediante un directorio de plugins privado; no toca los archivos del
sistema. Las limitaciones conocidas están en
[`docs/limitations.md`](docs/limitations.md) y los resultados en
[`docs/test-results.md`](docs/test-results.md).

## Licencias

- Código propio de BASC: `LicenseRef-BookOS`, como el resto de BookOS.
- `vendor/opus-celt`: BSD de 3 cláusulas, texto en
  [`vendor/opus-celt/COPYING`](vendor/opus-celt/COPYING). Las distribuciones
  binarias deben reproducir ese aviso.
- `spa-plugin/pipewire-1.6.11`: cabeceras y fuentes de PipeWire, con su
  licencia en [`spa-plugin/pipewire-1.6.11/COPYING`](spa-plugin/pipewire-1.6.11/COPYING).

## Patentes

SSC no es Opus conforme, así que no está claro que la concesión de patentes
libre de regalías de Opus lo cubra, y Samsung puede tener patentes propias
sobre SSC. **Antes de distribuir binarios conviene revisarlo con un
profesional.**

## Sin garantía

Se ofrece «tal cual», sin garantía de ningún tipo. Este documento no es
asesoramiento jurídico.
