#!/usr/bin/env bash
# Prueba de humo de TODAS las plantillas registradas contra la API local:
# por cada plantilla busca scripts/ejemplo-<nombre-archivo>.json, ajusta PlantillaId,
# genera un preview y una credencial persistida, y rasteriza la primera página en tmp/smoke/.
set -euo pipefail
cd "$(dirname "$0")/.."
BASE="${BASE:-http://localhost:3345/api/v3/transversal/credencial}"
mkdir -p tmp/smoke
ok=0; fallas=0
while IFS=$'\t' read -r id ruta; do
  stem="$(basename "${ruta%.typ}")"
  json="scripts/ejemplo-$stem.json"
  if [ ! -f "$json" ]; then echo "SIN EJEMPLO  $ruta"; fallas=$((fallas+1)); continue; fi
  cuerpo="$(python3 -c "import json,sys; p=json.load(open('$json')); p['PlantillaId']=$id; print(json.dumps(p))")"
  t0=$(date +%s.%N)
  code=$(curl -sS -o "tmp/smoke/$stem.pdf" -w '%{http_code}' -H 'Content-Type: application/json' --data "$cuerpo" "$BASE/generarPreview")
  ms=$(python3 -c "print(int(($(date +%s.%N)-$t0)*1000))")
  if [ "$code" != "200" ]; then echo "FALLA $code   $ruta"; cat "tmp/smoke/$stem.pdf"; echo; fallas=$((fallas+1)); continue; fi
  paginas=$(./.venv/bin/python -c "import pypdfium2 as p; d=p.PdfDocument('tmp/smoke/$stem.pdf'); print(len(d)); d[0].render(scale=0.8).to_pil().save('tmp/smoke/$stem-p1.png')")
  guid=$(curl -sS -H 'Content-Type: application/json' --data "$cuerpo" "$BASE/generarCredencial" | python3 -c "import json,sys; print(json.load(sys.stdin).get('guid','SIN GUID'))")
  printf "OK   %4s ms  %2s pág  %-70s guid=%s\n" "$ms" "$paginas" "$ruta" "$guid"
  ok=$((ok+1))
done < <(curl -sS "$BASE/plantilla" | python3 -c "import json,sys; [print(f\"{p['id']}\t{p['ruta']}\") for p in json.load(sys.stdin)]")
echo "== $ok ok, $fallas con problemas"
[ "$fallas" -eq 0 ]
