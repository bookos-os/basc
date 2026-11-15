#!/usr/bin/env python3
"""Genera la señal del experimento E3 (docs/protocol.md): 48 kHz, 24 bit, estéreo.

Tramos de 5 s: silencio digital, seno 1 kHz -20 dBFS solo izquierda, el mismo
solo derecha, barrido logarítmico 20 Hz-20 kHz en ambos. El silencio aísla los
campos fijos de la trama; los senos separan canales; el barrido recorre bandas.
"""
import math
import struct
import sys
import wave

FS = 48_000
TRAMO = 5 * FS
AMP = 10 ** (-20 / 20) * (2**23 - 1)


def muestras():
    for _ in range(TRAMO):
        yield 0, 0
    for n in range(TRAMO):
        yield round(AMP * math.sin(2 * math.pi * 1000 * n / FS)), 0
    for n in range(TRAMO):
        yield 0, round(AMP * math.sin(2 * math.pi * 1000 * n / FS))
    k = math.log(20_000 / 20)
    dur = TRAMO / FS
    for n in range(TRAMO):
        t = n / FS
        fase = 2 * math.pi * 20 * dur / k * (math.exp(t / dur * k) - 1)
        v = round(AMP * math.sin(fase))
        yield v, v


with wave.open(sys.argv[1] if len(sys.argv) > 1 else "e3-senal.wav", "wb") as w:
    w.setnchannels(2)
    w.setsampwidth(3)
    w.setframerate(FS)
    w.writeframes(b"".join(struct.pack("<i", l)[:3] + struct.pack("<i", r)[:3] for l, r in muestras()))
