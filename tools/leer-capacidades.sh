#!/bin/sh
# Lee por D-Bus las capacidades A2DP que anuncia un dispositivo conectado.
# BlueZ expone los SEP remotos como org.bluez.MediaEndpoint1 legibles sin root,
# así que no hace falta btmon para el experimento E1.
# Uso: tools/leer-capacidades.sh AA:BB:CC:DD:EE:FF
set -eu
dev="/org/bluez/hci0/dev_$(echo "$1" | tr : _)"
for sep in $(busctl tree org.bluez --list | grep "^$dev/sep[0-9]*$"); do
    printf '%s  códec=%s  caps=%s\n' "${sep##*/}" \
        "$(busctl get-property org.bluez "$sep" org.bluez.MediaEndpoint1 Codec)" \
        "$(busctl get-property org.bluez "$sep" org.bluez.MediaEndpoint1 Capabilities)"
done
