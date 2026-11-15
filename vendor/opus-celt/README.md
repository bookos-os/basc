# CELT de libopus 1.6 adaptado a SSC

Subconjunto de libopus 1.6 (`celt/`, tres cabeceras de `include/` y siete de
`silk/` que necesita `vq.c`). La licencia BSD está en `COPYING`. Es código de
Xiph y de los demás autores de Opus: **no contiene nada de Samsung**.

Todos los cambios están dentro de `#ifdef BASC_SSC` y recogidos en
`BASC.patch`, que se aplica sobre libopus 1.6 original. Hay cinco:

1. `mfrngcod.h`: el codificador de rango usa `EC_CODE_BITS` = 24.
2. `modes.c`: 22 bandas y la tabla de reparto de bits de SSC, que se usan
   siempre en lugar de las bandas Bark que libopus calcula para modos
   personalizados.
3. `celt_encoder.c`: el postfiltro no codifica el `tapset`.
4. `celt_decoder.c`: el mismo cambio en el decodificador. Se usa en los tests
   para comprobar las tramas.
5. `modes.c`: pre-énfasis 0 (SSC no la lleva; E5 en `docs/protocol.md`).
   Además, `opus_custom_mode_create` rechaza cualquier modo que no sea
   48 kHz/864.

De dónde sale cada valor y cómo se verificó: `docs/protocol.md`.
