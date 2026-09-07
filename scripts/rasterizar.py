#!/usr/bin/env python3
"""Rasteriza un PDF a PNG (una imagen por página). Uso: rasterizar.py <pdf> <prefijo-salida> [escala]"""
import sys, pypdfium2 as pdfium
pdf = pdfium.PdfDocument(sys.argv[1]); prefijo = sys.argv[2]; escala = float(sys.argv[3]) if len(sys.argv) > 3 else 1.4
for i in range(len(pdf)):
    pdf[i].render(scale=escala).to_pil().save(f"{prefijo}-p{i+1}.png")
print(f"{sys.argv[1]}: {len(pdf)} página(s) -> {prefijo}-p*.png")
