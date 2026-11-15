/* API C de basc-core (crates/basc-core/src/ffi.rs). */
#ifndef BASC_H
#define BASC_H

#include <stdint.h>

#define BASC_MUESTRAS 864
#define BASC_CANALES 2
#define BASC_TRAMA 484

typedef struct BascEncoder BascEncoder;

BascEncoder *basc_encoder_new(void);
/* pcm: BASC_MUESTRAS*BASC_CANALES floats entrelazados; salida: BASC_TRAMA
 * octetos. Devuelve BASC_TRAMA o -1. */
int32_t basc_encoder_encode(BascEncoder *enc, const float *pcm, uint8_t *salida);
void basc_encoder_free(BascEncoder *enc);

#endif
