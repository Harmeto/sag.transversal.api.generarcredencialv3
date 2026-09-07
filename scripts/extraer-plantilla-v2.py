#!/usr/bin/env python3
"""Extrae una plantilla HTML de v2 para migrarla a Typst.

Uso: scripts/extraer-plantilla-v2.py <ruta relativa en public/plantillas de v2>
  - escribe tmp/extraidas/<nombre>.html con las imágenes base64 reemplazadas por <IMG n>
  - guarda cada imagen en plantillas/<carpeta>/assets/<nombre>-imgN.<ext>
  - imprime las variables {x} que usa la plantilla
"""
import base64, os, re, sys, pathlib

V2 = pathlib.Path(os.environ.get("V2_DIR", "../sag.transversal.api.generarcredencialV2")) / "public/plantillas"
ruta = pathlib.Path(sys.argv[1])
html = (V2 / ruta).read_text(encoding="utf-8")
nombre = ruta.stem
assets = pathlib.Path("plantillas") / ruta.parent / "assets"
assets.mkdir(parents=True, exist_ok=True)

n = 0
def guardar(m):
    global n
    n += 1
    ext = {"jpeg": "jpg", "svg+xml": "svg"}.get(m.group(1), m.group(1))
    destino = assets / f"{nombre}-img{n}.{ext}"
    destino.write_bytes(base64.b64decode(m.group(2)))
    print(f"imagen {n}: {destino} ({destino.stat().st_size // 1024} KB)")
    return f"data:image/{m.group(1)};base64,<IMG {n}>"

limpio = re.sub(r"data:image/([\w+]+);base64,([A-Za-z0-9+/=\s]+)", guardar, html)
salida = pathlib.Path("tmp/extraidas") / f"{nombre}.html"
salida.parent.mkdir(parents=True, exist_ok=True)
salida.write_text(limpio, encoding="utf-8")
print(f"html sin base64: {salida} ({len(limpio)} chars, {limpio.count(chr(10))} líneas)")
print("variables:", " ".join(sorted(set(re.findall(r"\{([a-zA-Z_][a-zA-Z0-9_-]*)\}", limpio)))))
