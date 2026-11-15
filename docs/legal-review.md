# Revisión legal (preliminar; no es asesoramiento jurídico)

- **Binario de Samsung (`libScalable_Encoder.so`).** No está en este
  repositorio, no se enlaza ni se redistribuye. Se ha usado para estudiar el
  formato: la tabla de símbolos, tablas de datos (bandas, reparto de bits,
  probabilidades) y el comportamiento de algunas funciones (codificador de
  rango, energía, cabecera), leídas con llvm-objdump y Ghidra.
- **Qué entra en BASC de ese estudio:** solo **datos de interoperabilidad**:
  dos tablas numéricas (22 bordes de banda y 11 × 22 valores de reparto), una
  constante de precisión (24 bits), la ausencia de un campo y el formato de 4
  octetos de la cabecera. **Ni una línea de código de Samsung**, tampoco
  reescrita: la implementación es libopus.
- **`encoder.c` de terceros.** Es descompilación de código de Samsung y su
  README reserva los derechos a Samsung. Se leyó para conocer el orden de los
  campos del flujo. No se copia nada.
- **Base legal de la investigación.** Directiva 2009/24/CE art. 6
  (descompilación para interoperabilidad; en España, art. 100 LPI) y análisis
  de comportamiento con dispositivos propios: capturas HCI del propio teléfono
  y auriculares. Los límites del art. 6 aplican: la información obtenida solo
  sirve para interoperar, no se cede a terceros para otros fines y no se usa
  para un programa sustancialmente similar. BASC no se parece al encoder de
  Samsung: es CELT de libopus.
- **libopus (`vendor/opus-celt`).** Licencia BSD de 3 cláusulas, en `COPYING`.
  Exige conservar el aviso de copyright en el código y reproducirlo en la
  documentación de las distribuciones binarias. Los cambios están marcados
  (`BASC_SSC`, `BASC.patch`).
- **Código propio.** `LicenseRef-BookOS`, como el resto de BookOS.
- **Patentes. Hay que revisarlo antes de distribuir.** El núcleo es CELT,
  confirmado. Las patentes de Opus (Xiph, Broadcom, Microsoft y otros) se
  conceden libres de regalías para implementaciones conformes a la RFC 6716.
  **SSC no es Opus conforme** (rango de 24 bits, otras bandas), así que no está
  claro que esa concesión lo cubra. Samsung puede tener además patentes
  propias sobre SSC o UHQ.
- **Marca.** «Samsung Seamless Codec» y «SSC» son de Samsung. El proyecto se
  llama BASC y solo menciona SSC para describir la compatibilidad.
- **Nada se publica** sin decisión explícita.
