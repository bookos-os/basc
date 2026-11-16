# BASC

**BASC** is a free Bluetooth audio encoder for Linux (BookOS) that lets a PC
send audio to Samsung Galaxy Buds earbuds using the same transmission format
Samsung phones use.

> **BASC is not a Samsung product.** It is not affiliated with, sponsored,
> endorsed or reviewed by Samsung Electronics Co., Ltd. or any of its
> subsidiaries.

## Trademark notice

"Samsung", "Galaxy", "Galaxy Buds", "Samsung Seamless Codec" and "SSC" are
trademarks or trade names of Samsung Electronics Co., Ltd. They are mentioned
here **only to describe what BASC is compatible with** (descriptive,
nominative use). The project name, the PipeWire codec name
(`api.bluez5.codec = "basc"`) and the visible profile name ("BASC (compatible
with Samsung SSC)") were chosen so that BASC cannot be mistaken for Samsung's
product.

## What it is

- An **independent encoder** built on CELT from
  [libopus](https://opus-codec.org/) 1.6 (3-clause BSD license, by Xiph.Org
  and the other Opus authors), with five changes guarded by `#ifdef BASC_SSC`
  and collected in `vendor/opus-celt/BASC.patch`.
- A Rust layer (`crates/basc-core`) that wraps that encoder, builds the frame
  header and exposes a C API.
- An SPA plugin for PipeWire's Bluetooth support (`spa-plugin/`), lab use only
  for now.
- Research and test tools (`crates/basc-snoop`, `crates/basc-wav`, `tools/`).

## What it is not

- **It contains no Samsung code**, neither copied nor rewritten, and no
  Samsung binaries. It does not link to or redistribute
  `libScalable_Encoder.so` or any other proprietary library.
- It is not an official or certified SSC implementation. It is not
  guaranteed to work with every pair of earbuds or with future firmware.
- It does not implement SSC‑UHQ (96 kHz) or bitrate adaptation.
- It is not RFC 6716–conformant Opus: the frame format deliberately differs
  in order to interoperate.

## What it is for

So that a Linux user can listen to audio from **their own** PC on **their
own** Galaxy Buds with the codec those earbuds already support, instead of
being limited to SBC or AAC. This is an **interoperability** project.

## How it was made

The format was worked out for interoperability purposes under Article 6 of
Directive 2009/24/EC (Article 100 of the Spanish Intellectual Property Law),
by observing Bluetooth traffic between devices owned by the author (phone and
earbuds) and studying the behaviour of the phone's encoder. Only
**interoperability data** made it into BASC from that study: two numeric
tables (band edges and bit allocation), the 24-bit precision of the range
coder, the absence of one field, and the 4-byte frame header layout. Details
are in [`docs/legal-review.md`](docs/legal-review.md) and
[`docs/protocol.md`](docs/protocol.md) (in Spanish).

## What it works with

Tested only with:

| Component | Version |
|---|---|
| Earbuds | Galaxy Buds3 Pro |
| Reference sender (for measurements) | Galaxy S22 Ultra, Android 16 |
| System | Fedora 44 (BookOS), kernel 7.x |
| PipeWire / WirePlumber | 1.6.11 / 0.5.19 |
| BlueZ | 5.87 |
| Rust | 1.99 |

Single mode: 48 kHz, stereo, 484-byte CBR frames every 18 ms
(≈ 215 kbit/s). Other Buds models may or may not work; they have not been
tested.

## Build and test

Requirements: Rust, a C compiler, `pipewire-devel` and `pkg-config`.

```sh
cargo build --release
cargo test

make -C spa-plugin                            # lab SPA plugin
tools/lab-pipewire.sh on                      # enable BASC for the session (off undoes it)
tools/basc-reproducir.sh [--eq] song.flac     # play a file directly through BlueZ
```

The lab plugin takes the `aptx` slot of PipeWire's codec loader through a
private plugin directory; it does not modify system files. Known limitations
are in [`docs/limitations.md`](docs/limitations.md) and results in
[`docs/test-results.md`](docs/test-results.md).

## Licenses

- BASC's own code: `LicenseRef-BookOS`, like the rest of BookOS.
- `vendor/opus-celt`: 3-clause BSD, text in
  [`vendor/opus-celt/COPYING`](vendor/opus-celt/COPYING). Binary
  distributions must reproduce that notice.
- `spa-plugin/pipewire-1.6.11`: PipeWire headers and sources, under their
  license in [`spa-plugin/pipewire-1.6.11/COPYING`](spa-plugin/pipewire-1.6.11/COPYING).

## Patents

SSC is not conformant Opus, so it is unclear whether Opus's royalty-free
patent grant covers it, and Samsung may hold its own patents on SSC.
**Have this reviewed by a professional before distributing binaries.**

## No warranty

Provided "as is", without warranty of any kind. This document is not legal
advice.
