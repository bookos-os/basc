#!/bin/sh
# Prueba E4 completa: reenvía un volcado de tramas SSC a los Buds y deja el
# audio del escritorio como estaba.
# Uso: tools/e4-prueba.sh <volcado.bin> <primera> <cuántas> [MAC]
#
# Por qué cada paso:
# - Hay que parar WirePlumber porque BlueZ (a2dp_reconfig) no deja a otro
#   emisor D-Bus tocar un flujo que PipeWire tiene abierto (EPERM).
# - Al volver, WirePlumber solo encuentra el perfil HFP: el canal A2DP se
#   negoció con nuestro endpoint, que ya no existe. Desconectar y conectar el
#   perfil A2DP fuerza a PipeWire a negociar de nuevo (AAC). Sin esto, los
#   widgets se quedan en «esperando dispositivo».
set -u
mac="${4:-${BASC_MAC:?pasa la MAC o define BASC_MAC}}"
dev="/org/bluez/hci0/dev_$(echo "$mac" | tr : _)"
a2dp=0000110b-0000-1000-8000-00805f9b34fb

restaurar() {
    systemctl --user start wireplumber
    sleep 3
    busctl call org.bluez "$dev" org.bluez.Device1 DisconnectProfile s "$a2dp" || true
    sleep 2
    busctl call org.bluez "$dev" org.bluez.Device1 ConnectProfile s "$a2dp"
    # A veces WirePlumber elige igualmente HFP (visto el 12-11 tras 40 s de
    # prueba); se fuerza el perfil A2DP en la tarjeta.
    sleep 3
    pactl set-card-profile "bluez_card.$(echo "$mac" | tr : _)" a2dp-sink
}
trap restaurar EXIT

systemctl --user stop wireplumber
sleep 2
timeout 120 python3 -I "$(dirname "$0")/e4-reenvio.py" "$1" "$2" "$3" "$mac"
