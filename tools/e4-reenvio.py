#!/usr/bin/env python3
"""Experimento E4: reenvía a los Buds, sin tocarlas, tramas SSC capturadas al S22.

Valida negociación y transporte sin depender del codificador: si se oye la
señal de E3, BASC sabe abrir un flujo SSC que los Buds aceptan y decodifican.

Uso: tools/e4-reenvio.py <volcado basc-snoop .bin> <primera trama> <n tramas> [MAC]

Registra en BlueZ un endpoint fuente SSC propio y fuerza la configuración del
SEP remoto de Samsung. Esto le quita el transporte a PipeWire mientras dura;
al terminar, reconectar los Buds devuelve el AAC normal.
"""
import os
import struct
import sys
import threading
import time

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

# BlueZ lee "Vendor" en un struct {u16 cid; u16 vid} empaquetado (media.c,
# 5.87): el ID de empresa va en los 16 bits bajos.
VENDOR = (0x0103 << 16) | 0x0075
CAPS = bytes.fromhex("75000000 03013e")
# Lo que eligió el S22 en E2/E3.
CONFIG = bytes.fromhex("75000000 03010c")
TRAMA_S = 864 / 48000  # medido en E3: 278 tramas = 5,00 s
RUTA_EP = "/basc/ssc_source"
# 5 tramas = 90 ms de colchón. El S22 no se adelanta (ver docs/protocol.md),
# pero se retrasa hasta 36 ms; PipeWire también precarga su búfer.
PRECARGA = 5

volcado, primera, cuantas = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
mac = sys.argv[4] if len(sys.argv) > 4 else os.environ["BASC_MAC"]
disp = "/org/bluez/hci0/dev_" + mac.replace(":", "_")


def leer_tramas():
    d = open(volcado, "rb").read()
    out, i = [], 0
    while i < len(d):
        _, _, _, n = struct.unpack_from("<QHII", d, i)
        i += 18
        out.append(d[i : i + n])
        i += n
    return out[primera : primera + cuantas]


tramas = leer_tramas()
assert tramas and all(t[:2] == b"\xff\xee" for t in tramas), "tramas sin sincronía ff ee"

dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
bus = dbus.SystemBus()
bucle = GLib.MainLoop()


def enviar(transporte):
    tr = dbus.Interface(bus.get_object("org.bluez", transporte), "org.bluez.MediaTransport1")
    fd, _, mtu_w = tr.Acquire()
    fd = fd.take()
    print(f"transporte adquirido, MTU de escritura {mtu_w}")
    t0 = time.monotonic()
    retrasos = []
    for k, t in enumerate(tramas):
        # Misma forma que la del S22: v2, PT 96, marca de tiempo = nº de paquete.
        rtp = struct.pack(">BBHII", 0x80, 0x60, k & 0xFFFF, k, 1)
        # Las PRECARGA primeras salen de golpe: dan a los Buds un colchón
        # contra los retrasos del planificador. Sin él se oían cortes («pip»)
        # el 12-11.
        if k >= PRECARGA:
            retrasos.append(max(0.0, time.monotonic() - (t0 + (k - PRECARGA) * TRAMA_S)))
        os.write(fd, rtp + t)
        espera = t0 + (k + 1 - PRECARGA) * TRAMA_S - time.monotonic()
        if espera > 0:
            time.sleep(espera)
    # Con PRECARGA tramas o menos no hay ninguna enviada a su hora que medir.
    tarde = sorted(retrasos) or [0.0]
    print(
        f"{len(tramas)} tramas enviadas en {time.monotonic() - t0:.2f} s; "
        f"retraso sobre la hora: mediana {tarde[len(tarde) // 2] * 1000:.1f} ms, "
        f"p99 {tarde[int(len(tarde) * 0.99)] * 1000:.1f} ms, máx {tarde[-1] * 1000:.1f} ms, "
        f"más de {PRECARGA * 18} ms: {sum(r > PRECARGA * TRAMA_S for r in retrasos)}"
    )
    os.close(fd)
    tr.Release()
    GLib.idle_add(bucle.quit)


class Endpoint(dbus.service.Object):
    @dbus.service.method("org.bluez.MediaEndpoint1", in_signature="ay", out_signature="ay")
    def SelectConfiguration(self, caps):
        print("SelectConfiguration", bytes(caps).hex())
        return dbus.Array(CONFIG, signature="y")

    @dbus.service.method("org.bluez.MediaEndpoint1", in_signature="oa{sv}", out_signature="")
    def SetConfiguration(self, transporte, props):
        print("SetConfiguration", transporte, bytes(props.get("Configuration", b"")).hex())
        threading.Thread(target=enviar, args=(transporte,), daemon=True).start()

    @dbus.service.method("org.bluez.MediaEndpoint1", in_signature="o", out_signature="")
    def ClearConfiguration(self, transporte):
        print("ClearConfiguration", transporte)

    @dbus.service.method("org.bluez.MediaEndpoint1", in_signature="", out_signature="")
    def Release(self):
        print("Release")


ep = Endpoint(bus, RUTA_EP)
media = dbus.Interface(bus.get_object("org.bluez", "/org/bluez/hci0"), "org.bluez.Media1")
media.RegisterEndpoint(
    RUTA_EP,
    {
        "UUID": "0000110a-0000-1000-8000-00805f9b34fb",
        "Codec": dbus.Byte(0xFF),
        "Vendor": dbus.UInt32(VENDOR),
        "Capabilities": dbus.Array(CAPS, signature="y"),
    },
)

om = dbus.Interface(bus.get_object("org.bluez", "/"), "org.freedesktop.DBus.ObjectManager")
remoto = next(
    (
        p
        for p, ifs in om.GetManagedObjects().items()
        if p.startswith(disp + "/sep")
        and int(ifs.get("org.bluez.MediaEndpoint1", {}).get("Codec", -1)) == 0xFF
    ),
    None,
)
if remoto:
    # Solo funciona si nadie más tiene un flujo abierto: BlueZ (a2dp.c,
    # a2dp_reconfig) prohíbe cambiar de SEP a un emisor D-Bus distinto.
    print("SEP remoto SSC:", remoto)
    dbus.Interface(bus.get_object("org.bluez", remoto), "org.bluez.MediaEndpoint1").SetConfiguration(
        dbus.ObjectPath(RUTA_EP),
        {"Capabilities": dbus.Array(CONFIG, signature="y")},
        reply_handler=lambda: print("configuración aceptada por BlueZ"),
        error_handler=lambda e: (print("SetConfiguration falló:", e), bucle.quit()),
    )
else:
    # Sin canal A2DP abierto, BlueZ negocia al conectar el perfil y nos pide
    # SelectConfiguration, porque somos la única fuente registrada.
    print("sin SEP remoto; conectando el perfil A2DP sink")
    dbus.Interface(bus.get_object("org.bluez", disp), "org.bluez.Device1").ConnectProfile(
        "0000110b-0000-1000-8000-00805f9b34fb",
        reply_handler=lambda: print("perfil A2DP conectado"),
        error_handler=lambda e: (print("ConnectProfile falló:", e), bucle.quit()),
    )
GLib.timeout_add_seconds(120, bucle.quit)
try:
    bucle.run()
finally:
    media.UnregisterEndpoint(RUTA_EP)
