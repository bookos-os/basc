#!/bin/sh
# Reproduce un archivo de audio en los Buds por SSC, codificado por BASC.
#
#   tools/basc-reproducir.sh canción.flac            # señal tal cual
#   tools/basc-reproducir.sh --eq canción.flac       # con el EQ de BookOS
#   tools/basc-reproducir.sh --desde 30 --durante 40 canción.flac
#
# Mientras no exista el plugin de PipeWire, BASC no puede procesar en vivo.
# --eq toma prestado el filtro de auriculares de BookOS reproduciendo primero
# el fragmento por AAC y grabando lo que llega a los Buds: así las dos
# versiones llevan el mismo EQ y se pueden comparar.
set -eu
aqui="$(cd "$(dirname "$0")/.." && pwd)"
mac="${BASC_MAC:?define BASC_MAC con la MAC de los Buds}"
eq=0 desde=0 durante=60
while [ $# -gt 1 ]; do
    case "$1" in
        --eq) eq=1 ;;
        --desde) desde="$2"; shift ;;
        --durante) durante="$2"; shift ;;
        *) break ;;
    esac
    shift
done
[ $# -eq 1 ] || { echo "uso: $0 [--eq] [--desde s] [--durante s] archivo" >&2; exit 2; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
ffmpeg -v error -y -ss "$desde" -t "$durante" -i "$1" -ar 48000 -ac 2 -c:a pcm_s24le "$tmp/a.wav"

if [ "$eq" -eq 1 ]; then
    echo "Sonando por AAC para grabar con el EQ de BookOS ($durante s)…"
    sink="bluez_output.$(echo "$mac" | tr : _).1"
    pw-record --target "$sink" -P stream.capture.sink=true --rate 48000 --channels 2 \
        --format s24 "$tmp/eq.wav" &
    rec=$!
    sleep 1
    pw-play "$tmp/a.wav"
    sleep 1
    kill -INT "$rec"
    wait "$rec" || true
    mv "$tmp/eq.wav" "$tmp/a.wav"
fi

cargo run --manifest-path "$aqui/Cargo.toml" --release --offline -q -p basc-wav -- "$tmp/a.wav" "$tmp/a.bin"
tramas=$(( $(stat -c %s "$tmp/a.bin") / (18 + 484) ))
echo "Sonando por BASC ($tramas tramas)…"
"$aqui/tools/e4-prueba.sh" "$tmp/a.bin" 0 "$tramas" "$mac"
