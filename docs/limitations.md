# Limitaciones (2026‑11‑12)

- **Plugin solo en laboratorio.** Funciona en la sesión con
  `tools/lab-pipewire.sh on`, ocupando el hueco `aptx` del cargador de
  PipeWire. Para distribuirlo hay que parchear PipeWire (lista cerrada de
  códecs).
- **Un solo modo:** 48 kHz, estéreo, 484 octetos CBR (≈ 215 kbit/s), la
  configuración `0x0C` que elige el S22. No adapta el bitrate: falta el modo de
  324 octetos (`f2`) que el S22 usa cuando el enlace empeora. Si el socket se
  llena, media-sink.c descarta el paquete (revisión de Gemini, 12-11).
- **Cabecera con valores fijos** (`x1`, `f3`). Los Buds los aceptan, pero no
  se informa de la energía de la última banda ni se varía el corte de ancho
  de banda como hace Samsung.
- **Nivel:** igual que Samsung desde que se quitó la pre-énfasis (E5). BASC
  deja −1 dB de margen y Samsung ≈ −0,6 dB. Hay un limitador con ganancia
  (`BASC_GANANCIA_DB`), apagado (0 dB): con música masterizada no gana nada.
- **Pruebas largas sin hacer:** estabilidad de horas, reconexiones, enlace
  degradado.
- **UHQ / 96 kHz sin referencia.** El S22 Ultra no emite UHQ y los Buds3 Pro
  anuncian un solo SEP Samsung. Cualquier trabajo de UHQ sería inventado.
- **`basc-snoop`** no reensambla la señalización AVDTP fragmentada (avisa) y
  solo lee btsnoop H4 (1002).
