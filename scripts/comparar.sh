#!/usr/bin/env bash
# Renderiza la misma plantilla con Chromium (v2) y con Typst (v3) usando el mismo payload,
# y deja los PNG de cada página en tmp/comparacion/<nombre>/ para compararlos visualmente.
#
# Uso: scripts/comparar.sh <plantilla.html (ruta relativa v2)> <plantilla.typ (ruta relativa v3)> <payload.json>
# Requiere: target/release/bench-render, .venv (pypdfium2), node y el build de v2 con su Chromium.
set -euo pipefail
cd "$(dirname "$0")/.."
HTML="$1"; TYP="$2"; JSON="$3"
NOMBRE="$(basename "${TYP%.typ}")"
V2="${V2_DIR:-$(cd ../sag.transversal.api.generarcredencialV2 && pwd)}"
OUT="tmp/comparacion/$NOMBRE"; mkdir -p "$OUT"

echo "== Typst"
./target/release/bench-render "$TYP" "$JSON" "${N:-10}" 4
cp "tmp/bench-$(echo "${TYP%.typ}" | tr '/' '_').pdf" "$OUT/typst.pdf"

if [ "${SOLO_TYPST:-0}" != "1" ]; then
  echo "== Chromium (v2)"
  NODE_ENV=local PUPPETEER_CACHE_DIR="$V2/.cache/puppeteer" V2_DIR="$V2" PLANTILLA="$HTML" PAYLOAD="$JSON" \
    OUT="$OUT/chromium.pdf" N="${N:-5}" PAR=2 node scripts/bench-chromium-v2.mjs
  ./.venv/bin/python scripts/rasterizar.py "$OUT/chromium.pdf" "$OUT/chromium"
fi
./.venv/bin/python scripts/rasterizar.py "$OUT/typst.pdf" "$OUT/typst"
ls "$OUT"
