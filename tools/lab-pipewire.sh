#!/bin/sh
# Activa o desactiva el códec SSC de BASC en el PipeWire de la sesión.
#
#   tools/lab-pipewire.sh on    # WirePlumber carga el plugin de BASC
#   tools/lab-pipewire.sh off   # todo como estaba
#
# Por qué así:
# - El cargador de códecs de PipeWire 1.6.11 tiene una lista cerrada de
#   nombres. Sin parchear PipeWire, la única forma de cargar un códec propio
#   es ocupar un nombre de esa lista que el sistema no tenga: aptx en Fedora.
# - SPA_PLUGIN_DIR sustituye al directorio de plugins entero. Por eso se arma
#   un espejo con enlaces a todos los del sistema, más el de BASC.
# - Solo lo necesita WirePlumber: el monitor bluez5, y con él los códecs,
#   corre en su proceso. El demonio pipewire no se toca.
# - Es un drop-in de systemd en ~/.config; «off» lo borra.
set -eu
aqui="$(cd "$(dirname "$0")/.." && pwd)"
sistema=/usr/lib64/spa-0.2
espejo="${XDG_DATA_HOME:-$HOME/.local/share}/basc/spa-0.2"
dropin="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/wireplumber.service.d/basc-lab.conf"

case "${1:-}" in
on)
    [ ! -e "$sistema/bluez5/libspa-codec-bluez5-aptx.so" ] ||
        { echo "El sistema ya trae aptx: este método no vale aquí." >&2; exit 1; }
    make -C "$aqui/spa-plugin" -s
    rm -rf "$espejo"
    mkdir -p "$espejo/bluez5"
    for d in "$sistema"/*; do
        [ "$(basename "$d")" = bluez5 ] || ln -s "$d" "$espejo/"
    done
    for f in "$sistema"/bluez5/*; do ln -s "$f" "$espejo/bluez5/"; done
    ln -s "$aqui/spa-plugin/libspa-codec-bluez5-aptx.so" "$espejo/bluez5/"
    mkdir -p "$(dirname "$dropin")"
    # Entre comillas: systemd parte Environment= por espacios.
    printf '[Service]\nEnvironment="SPA_PLUGIN_DIR=%s"\n' "$espejo" > "$dropin"
    ;;
off)
    rm -f "$dropin"
    rm -rf "$espejo"
    ;;
*)
    echo "uso: $0 on|off" >&2
    exit 2
    ;;
esac
systemctl --user daemon-reload
systemctl --user restart wireplumber
# Tras reiniciar WirePlumber, los auriculares ya conectados se quedaban solo en
# HFP (visto dos veces el 12-11). Reabrir A2DP hace que se negocie de nuevo.
sleep 3
bluetoothctl devices Connected | while read -r _ mac _; do
    busctl call org.bluez "/org/bluez/hci0/dev_$(echo "$mac" | tr : _)" \
        org.bluez.Device1 ConnectProfile s 0000110b-0000-1000-8000-00805f9b34fb \
        2>/dev/null || true
done
echo "WirePlumber reiniciado ($1)."
