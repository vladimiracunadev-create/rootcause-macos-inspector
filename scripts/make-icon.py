#!/usr/bin/env python3
"""Genera el icono de la app: un radar de círculos concéntricos en el azul de
marca (#1f6feb) sobre fondo oscuro (#0d1117), la misma identidad que dibuja la
interfaz en tiempo de ejecución.

Escribe un PNG por cada tamaño que `iconutil` espera en un `.iconset`. El PNG se
escribe a mano con `zlib` y `struct` para no depender de Pillow ni de ninguna
utilidad externa: así el empaquetado funciona igual en un Mac recién instalado
y en un runner de CI.
"""

import math
import pathlib
import struct
import sys
import zlib

FONDO = (13, 17, 23, 255)
MARCA = (31, 111, 235, 255)

# (nombre del archivo, lado en píxeles) que exige el formato .iconset
TAMANOS = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]


def dibujar(lado: int) -> bytes:
    """Devuelve las filas RGBA del icono al tamaño pedido, con antialiasing por
    supermuestreo de 3x3: sin él, los anillos finos quedan dentados."""
    centro = (lado - 1) / 2.0
    r_exterior = lado * 0.405
    r_interior = lado * 0.203
    grosor = max(lado * 0.047, 1.0)
    r_punto = lado * 0.062
    muestras = 3
    paso = 1.0 / (muestras + 1)

    filas = []
    for y in range(lado):
        fila = bytearray()
        for x in range(lado):
            cubierto = 0
            for sy in range(1, muestras + 1):
                for sx in range(1, muestras + 1):
                    dx = x + sx * paso - 0.5 - centro
                    dy = y + sy * paso - 0.5 - centro
                    distancia = math.sqrt(dx * dx + dy * dy)
                    en_anillo = (
                        abs(distancia - r_exterior) < grosor / 2
                        or abs(distancia - r_interior) < grosor / 2
                    )
                    if en_anillo or distancia < r_punto:
                        cubierto += 1
            mezcla = cubierto / (muestras * muestras)
            pixel = tuple(
                round(FONDO[i] + (MARCA[i] - FONDO[i]) * mezcla) for i in range(4)
            )
            fila.extend(pixel)
        filas.append(bytes(fila))
    return filas


def escribir_png(ruta: pathlib.Path, lado: int, filas) -> None:
    """Escribe un PNG RGBA de 8 bits sin dependencias externas."""

    def trozo(tipo: bytes, datos: bytes) -> bytes:
        return (
            struct.pack(">I", len(datos))
            + tipo
            + datos
            + struct.pack(">I", zlib.crc32(tipo + datos) & 0xFFFFFFFF)
        )

    cabecera = struct.pack(">IIBBBBB", lado, lado, 8, 6, 0, 0, 0)
    # Cada fila va precedida por su byte de filtro (0 = sin filtro).
    crudo = b"".join(b"\x00" + fila for fila in filas)

    ruta.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + trozo(b"IHDR", cabecera)
        + trozo(b"IDAT", zlib.compress(crudo, 9))
        + trozo(b"IEND", b"")
    )


def main() -> int:
    if len(sys.argv) != 2:
        print("uso: make-icon.py <carpeta.iconset>", file=sys.stderr)
        return 2

    destino = pathlib.Path(sys.argv[1])
    destino.mkdir(parents=True, exist_ok=True)

    # Los tamaños se repiten entre entradas del iconset (32, 256, 512): se
    # dibuja cada lado una sola vez y se reutiliza.
    cache = {}
    for nombre, lado in TAMANOS:
        if lado not in cache:
            cache[lado] = dibujar(lado)
        escribir_png(destino / nombre, lado, cache[lado])

    print(f"{len(TAMANOS)} PNG escritos en {destino}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
