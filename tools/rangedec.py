"""Decodificador de rango de CELT según RFC 6716 §4.1, para probar H1 sobre tramas SSC."""
import struct


class RangeDec:
    def __init__(self, buf):
        self.buf, self.pos = buf, 0
        self.rng = 1 << 7
        self.rem = self._byte()
        self.val = self.rng - 1 - (self.rem >> 1)
        self._norm()

    def _byte(self):
        b = self.buf[self.pos] if self.pos < len(self.buf) else 0
        self.pos += 1
        return b

    def _norm(self):
        while self.rng <= 1 << 23:
            self.rng <<= 8
            sym = self.rem
            self.rem = self._byte()
            sym = ((sym << 8) | self.rem) >> 1
            self.val = ((self.val << 8) + (255 & ~sym)) & 0x7FFFFFFF

    def bit_logp(self, logp):
        s = self.rng >> logp
        r = self.val < s
        if not r:
            self.val -= s
        self.rng = s if r else self.rng - s
        self._norm()
        return int(r)


def tramas(ruta):
    d = open(ruta, "rb").read()
    i = 0
    while i < len(d):
        _, _, _, n = struct.unpack_from("<QHII", d, i)
        i += 18
        yield d[i : i + n]
        i += n
