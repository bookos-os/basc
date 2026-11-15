/* Sustituto mínimo de <bluetooth/bluetooth.h> (bluez-libs-devel).
 * media-codecs.c de PipeWire solo usa htobs/htobl; no hace falta instalar el
 * paquete de desarrollo para dos macros de orden de octetos. */
#ifndef BASC_COMPAT_BLUETOOTH_H
#define BASC_COMPAT_BLUETOOTH_H
#include <endian.h>
#define htobs(x) htole16(x)
#define htobl(x) htole32(x)
#endif
