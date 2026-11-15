/* Códec SSC de BASC para el soporte Bluetooth de PipeWire (SPA bluez5).
 *
 * Estructura copiada del plugin de LDAC de PipeWire 1.6.11. Toda la
 * codificación la hace basc-core (Rust), la misma que se probó con los Buds.
 *
 * Valores medidos en el S22 Ultra (basc/docs/protocol.md): códec de fabricante
 * Samsung 0x0075 / 0x0103, capacidad 0x3E, configuración 0x0C, 48 kHz
 * estéreo, una trama de 484 octetos (864 muestras) por paquete RTP sin octeto
 * de cuenta de tramas.
 */

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <arpa/inet.h>

#include <spa/param/audio/format.h>
#include <spa/pod/builder.h>

#include "rtp.h"
#include "media-codecs.h"
#include "basc.h"

#define SAMSUNG_VENDOR_ID 0x00000075
#define SSC_CODEC_ID      0x0103

/* Único octeto propio de la capacidad. Su significado es desconocido: se
 * anuncia lo que anuncian los Buds y se elige lo que eligió el S22. */
#define SSC_CAPS   0x3E
#define SSC_CONFIG 0x0C

typedef struct {
	a2dp_vendor_codec_t info;
	uint8_t ssc;
} __attribute__((packed)) a2dp_ssc_t;

struct impl {
	BascEncoder *enc;
	/* El S22 pone en la marca de tiempo RTP el número de paquete, no de
	 * muestra; se imita para no depender de cómo la interpreten los Buds. */
	uint32_t paquete;
};

static struct spa_log *log_;

static int codec_fill_caps(const struct media_codec *codec, uint32_t flags,
		const struct spa_dict *settings, uint8_t caps[A2DP_MAX_CAPS_SIZE])
{
	const a2dp_ssc_t a = {
		.info.vendor_id = SAMSUNG_VENDOR_ID,
		.info.codec_id = SSC_CODEC_ID,
		.ssc = SSC_CAPS,
	};
	memcpy(caps, &a, sizeof(a));
	return sizeof(a);
}

static int codec_select_config(const struct media_codec *codec, uint32_t flags,
		const void *caps, size_t caps_size,
		const struct media_codec_audio_info *info,
		const struct spa_dict *settings, uint8_t config[A2DP_MAX_CAPS_SIZE],
		void **config_data)
{
	a2dp_ssc_t c;

	if (caps_size < sizeof(c))
		return -EINVAL;
	memcpy(&c, caps, sizeof(c));
	if (c.info.vendor_id != SAMSUNG_VENDOR_ID || c.info.codec_id != SSC_CODEC_ID)
		return -ENOTSUP;
	/* Solo se conoce una configuración que los Buds aceptan y reproducen. */
	if ((c.ssc & SSC_CONFIG) != SSC_CONFIG)
		return -ENOTSUP;
	c.ssc = SSC_CONFIG;
	memcpy(config, &c, sizeof(c));
	return sizeof(c);
}

static int codec_validate_config(const struct media_codec *codec, uint32_t flags,
		const void *caps, size_t caps_size, struct spa_audio_info *info)
{
	a2dp_ssc_t c;

	if (caps_size < sizeof(c))
		return -EINVAL;
	memcpy(&c, caps, sizeof(c));
	if (c.info.vendor_id != SAMSUNG_VENDOR_ID || c.info.codec_id != SSC_CODEC_ID ||
	    c.ssc != SSC_CONFIG)
		return -ENOTSUP;

	spa_zero(*info);
	info->media_type = SPA_MEDIA_TYPE_audio;
	info->media_subtype = SPA_MEDIA_SUBTYPE_raw;
	info->info.raw.format = SPA_AUDIO_FORMAT_F32;
	info->info.raw.rate = 48000;
	info->info.raw.channels = 2;
	info->info.raw.position[0] = SPA_AUDIO_CHANNEL_FL;
	info->info.raw.position[1] = SPA_AUDIO_CHANNEL_FR;
	return 0;
}

static int codec_enum_config(const struct media_codec *codec, uint32_t flags,
		const void *caps, size_t caps_size, uint32_t id, uint32_t idx,
		struct spa_pod_builder *b, struct spa_pod **param)
{
	struct spa_audio_info info;
	uint32_t position[2] = { SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR };
	int res;

	if (idx > 0)
		return 0;
	if ((res = codec_validate_config(codec, flags, caps, caps_size, &info)) < 0)
		return res;

	*param = spa_pod_builder_add_object(b,
			SPA_TYPE_OBJECT_Format, id,
			SPA_FORMAT_mediaType,      SPA_POD_Id(SPA_MEDIA_TYPE_audio),
			SPA_FORMAT_mediaSubtype,   SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),
			SPA_FORMAT_AUDIO_format,   SPA_POD_Id(SPA_AUDIO_FORMAT_F32),
			SPA_FORMAT_AUDIO_rate,     SPA_POD_Int(48000),
			SPA_FORMAT_AUDIO_channels, SPA_POD_Int(2),
			SPA_FORMAT_AUDIO_position, SPA_POD_Array(sizeof(uint32_t),
					SPA_TYPE_Id, 2, position));
	return *param == NULL ? -EIO : 1;
}

static void *codec_init(const struct media_codec *codec, uint32_t flags,
		void *config, size_t config_len, const struct spa_audio_info *info,
		void *props, size_t mtu)
{
	struct impl *this;

	if (info->info.raw.format != SPA_AUDIO_FORMAT_F32 ||
	    info->info.raw.rate != 48000 || info->info.raw.channels != 2) {
		errno = EINVAL;
		return NULL;
	}
	if (mtu < sizeof(struct rtp_header) + BASC_TRAMA) {
		spa_log_error(log_, "BASC: MTU %zu menor que una trama SSC", mtu);
		errno = EINVAL;
		return NULL;
	}
	this = calloc(1, sizeof(*this));
	if (this == NULL)
		return NULL;
	this->enc = basc_encoder_new();
	if (this->enc == NULL) {
		free(this);
		errno = EIO;
		return NULL;
	}
	return this;
}

static void codec_deinit(void *data)
{
	struct impl *this = data;
	basc_encoder_free(this->enc);
	free(this);
}

static int codec_get_block_size(void *data)
{
	return BASC_MUESTRAS * BASC_CANALES * sizeof(float);
}

static int codec_start_encode(void *data,
		void *dst, size_t dst_size, uint16_t seqnum, uint32_t timestamp)
{
	struct impl *this = data;
	struct rtp_header *header = dst;

	if (dst_size < sizeof(*header))
		return -EINVAL;
	memset(header, 0, sizeof(*header));
	header->v = 2;
	header->pt = 96;
	header->sequence_number = htons(seqnum);
	header->timestamp = htonl(this->paquete++);
	header->ssrc = htonl(1);
	/* Sin octeto rtp_payload: en las tramas del S22 la cabecera SSC ff ee va
	 * justo detrás de la cabecera RTP. */
	return sizeof(struct rtp_header);
}

static int codec_encode(void *data,
		const void *src, size_t src_size,
		void *dst, size_t dst_size,
		size_t *dst_out, int *need_flush)
{
	struct impl *this = data;
	int bloque = codec_get_block_size(data);

	if (src_size < (size_t)bloque || dst_size < BASC_TRAMA)
		return -EINVAL;
	if (basc_encoder_encode(this->enc, src, dst) != BASC_TRAMA)
		return -EIO;
	*dst_out = BASC_TRAMA;
	*need_flush = NEED_FLUSH_ALL;
	return bloque;
}

static void codec_get_delay(void *data, uint32_t *encoder, uint32_t *decoder)
{
	/* CELT: 108 muestras medidas al decodificar, más una trama (864) de
	 * anticipación del limitador de basc-core. */
	if (encoder)
		*encoder = 108 + BASC_MUESTRAS;
	if (decoder)
		*decoder = 0;
}

static void codec_set_log(struct spa_log *global_log)
{
	log_ = global_log;
	spa_log_topic_init(log_, &codec_plugin_log_topic);
}

const struct media_codec a2dp_codec_basc_ssc = {
	/* Laboratorio: SSC no tiene valor propio en el enum cerrado de PipeWire,
	 * así que se ocupa el de aptX, que Fedora no trae
	 * (basc/docs/architecture.md). */
	.id = SPA_BLUETOOTH_AUDIO_CODEC_APTX,
	.kind = MEDIA_CODEC_A2DP,
	.codec_id = A2DP_CODEC_VENDOR,
	.vendor = { .vendor_id = SAMSUNG_VENDOR_ID, .codec_id = SSC_CODEC_ID },
	/* «SSC» y «Samsung Seamless Codec» son nombres de Samsung: el producto se
	 * llama BASC y SSC solo se menciona para decir con qué es compatible
	 * (docs/legal-review.md). */
	.name = "basc",
	.description = "BASC (compatible con SSC de Samsung)",
	.fill_caps = codec_fill_caps,
	.select_config = codec_select_config,
	.enum_config = codec_enum_config,
	.validate_config = codec_validate_config,
	.init = codec_init,
	.deinit = codec_deinit,
	.get_block_size = codec_get_block_size,
	.start_encode = codec_start_encode,
	.encode = codec_encode,
	.get_delay = codec_get_delay,
	.set_log = codec_set_log,
};

MEDIA_CODEC_EXPORT_DEF(
	"aptx",
	&a2dp_codec_basc_ssc
);
