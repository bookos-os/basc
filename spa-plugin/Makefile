# Plugin de laboratorio: libspa-codec-bluez5-aptx.so con el códec SSC de BASC.
# Se carga con SPA_PLUGIN_DIR (tools/lab-pipewire.sh); no se instala en el
# sistema.
#
# Requiere pipewire-devel (cabeceras SPA) y `cargo build --release -p basc-core`.

RAIZ := ..
PW := pipewire-1.6.11
CFLAGS := -O2 -g -fPIC -Wall -Wextra -Wno-unused-parameter \
	$(shell pkg-config --cflags libspa-0.2) -I$(PW) -Icompat -I. -DCODEC_PLUGIN
BASC_A := $(RAIZ)/target/release/libbasc_core.a

libspa-codec-bluez5-aptx.so: basc-ssc.c $(PW)/media-codecs.c basc.h $(BASC_A)
	$(CC) $(CFLAGS) -shared -o $@ basc-ssc.c $(PW)/media-codecs.c \
		-Wl,--whole-archive $(BASC_A) -Wl,--no-whole-archive \
		-Wl,--version-script=exports.map -Wl,--strip-debug -lm -lpthread -ldl

$(BASC_A):
	cargo build --manifest-path $(RAIZ)/Cargo.toml --release --offline -p basc-core

clean:
	rm -f libspa-codec-bluez5-aptx.so
