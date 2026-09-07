#!/usr/bin/env bash
# Prueba de humo contra la API local: health, catálogos, preview y generación persistida.
set -euo pipefail
BASE="${BASE:-http://localhost:3345/api/v3/transversal/credencial}"
OUT="${OUT:-./tmp}"; mkdir -p "$OUT"
cd "$(dirname "$0")/.."
echo "== health";      curl -sS "$BASE/health"; echo
echo "== categoria";   curl -sS "$BASE/categoria"; echo
echo "== plantilla";   curl -sS "$BASE/plantilla?paginate=true&limit=5"; echo
echo "== preview -> $OUT/preview-datos-cze.pdf"
curl -sS -o "$OUT/preview-datos-cze.pdf" -w 'HTTP %{http_code} en %{time_total}s, %{size_download} bytes\n' \
  -H 'Content-Type: application/json' --data @scripts/ejemplo-datos-cze.json "$BASE/generarPreview"
echo "== generarCredencial"
curl -sS -H 'Content-Type: application/json' --data @scripts/ejemplo-datos-cze.json "$BASE/generarCredencial"; echo
echo "== credencial (últimas)"; curl -sS "$BASE/credencial?sort=fecha_creacion_desc&paginate=true&limit=3"; echo
