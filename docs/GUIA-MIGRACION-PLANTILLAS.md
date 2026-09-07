# Guía para migrar una plantilla HTML de v2 a Typst (v3)

Convenciones y herramientas usadas en las plantillas ya migradas
(`plantillas/mascotas/datos-cze.typ`, `plantillas/emergencias-pecuarias/bioseguridad-traspatio.typ`).
Léelas antes de empezar: son la referencia de estilo.

## 0. Entorno

```bash
export PATH="$HOME/.cargo/bin:$PATH"     # cargo / rustc
# v2 vive en ../sag.transversal.api.generarcredencialV2 (plantillas en public/plantillas)
```

No levantes la API (`scripts/run-local.sh`) mientras migras: usa `bench-render` y `scripts/comparar.sh`.
`bench-render` aplica las mismas normalizaciones de datos que la API (JSON embebido como string,
fragmentos `*Html`), así que lo que ves es lo que la API produciría.

## 1. Flujo por plantilla

1. **Extraer**: `scripts/extraer-plantilla-v2.py <carpeta>/<archivo>.html`
   - deja `tmp/extraidas/<archivo>.html` sin base64 (léelo completo: CSS, estructura y `<script>`)
   - guarda las imágenes en `plantillas/<carpeta>/assets/<archivo>-imgN.<ext>`; renómbralas con un nombre
     descriptivo (`logo-sag.png`, `firma.jpg`...) y míralas con Read para saber qué son
   - imprime las variables `{x}`
2. **Entender el contrato**: qué variables llegan, cuáles pueden venir vacías, cuáles son data URL
   (fotos, QR), cuáles JSON serializado, y qué hace el `<script>` si lo hay. Si el consumidor está en
   `~/Development/emergencias-pecuarias/*` (BFF, appwebnextjs) o en otro repo local, revísalo con grep
   para armar un payload de ejemplo realista. Si no lo encuentras, inventa datos verosímiles.
3. **Escribir** `plantillas/<carpeta>/<archivo>.typ` (mismo nombre que el HTML, extensión `.typ`).
4. **Payload de ejemplo** en `scripts/ejemplo-<archivo>.json` con la forma de v2:
   `{ "PlantillaId": N, "GenerarQR": bool, "QRTag": "...", "DatosCredencial": { ... } }`.
   Para fotos/QR de prueba genera un PNG pequeño con `.venv/bin/python` (PIL) y ponlo como data URL.
5. **Comparar**: `scripts/comparar.sh <carpeta>/<archivo>.html <carpeta>/<archivo>.typ scripts/ejemplo-<archivo>.json`
   deja `tmp/comparacion/<archivo>/chromium-pN.png` y `typst-pN.png`. Míralos con Read, página por
   página, y ajusta hasta que estructura, proporciones, colores, tipografías y saltos de página coincidan.
   Objetivo: que el dueño del negocio no note diferencia a simple vista. No hace falta identidad al píxel.
   Si sólo cambiaste el `.typ`, `SOLO_TYPST=1 scripts/comparar.sh ...` evita relanzar Chromium.
6. **Probar bordes**: payload con todo vacío (`"DatosCredencial": {}`) no debe fallar; listas largas
   deben paginar bien.

## 2. Convenciones Typst del proyecto

```typst
#import sys: inputs
#let d = inputs.at("datos", default: (:))     // DatosCredencial completo

#let como-texto(x) = {                          // todo valor a string seguro
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))  // variable simple {k}
```

- **Página**: `#set page(paper: "us-letter" | "a4", margin: ...)`. Copia `@page` del CSS; si no hay,
  Chromium usó `format: 'letter'` con `preferCSSPageSize`.
- **Fuente**: `#set text(font: ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"), size: Npt)`.
  CSS `px` → `pt` × 0.75. Fuentes propias van en `fonts/` (ya están gobCL y Asap Condensed en .otf/.ttf);
  refiérelas por su nombre de familia (`"gobCL"`, `"Asap Condensed"`); comprueba el nombre exacto con
  `fc-scan fonts/x.otf | grep family` o probando. Typst no lee `.woff`.
- **Encabezado y pie repetidos**: `set page(header: ..., footer: ...)` con `context` y
  `counter(page).display()` / `counter(page).final().first()`. Ajusta `margin.top` al alto del header y
  usa `header-ascent`. Para que el header tenga altura fija, usa `table(rows: Xpt)` o `block(height:)`.
- **Marca de agua / fondo**: `set page(background: place(center + horizon, image(...)))`. La opacidad
  se hornea en el PNG con PIL (ver `assets/sag-marca-agua.png`).
- **Imágenes de la plantilla**: `image("assets/x.png", width: ...)` (ruta relativa al `.typ`).
- **Imágenes que vienen en los datos** (`foto`, `QR`, firmas): la API convierte todo string
  `data:image/...;base64,...` en un diccionario `(format: "png"|"jpg"|"svg"|"gif"|"webp", mime, bytes)`.
  Úsalo así:
  ```typst
  #let img = d.at("foto", default: none)
  #if type(img) == dictionary and "bytes" in img { image(img.bytes, format: img.format, width: 80pt) }
  ```
  Si el valor no es data URL (URL http, texto vacío) no se muestra nada. Igual que v2, que en ese caso
  pintaba una imagen rota o nada.
- **Estructuras serializadas**: si un valor string parece JSON (`[...]`/`{...}`), la API ya lo parseó;
  la plantilla lo recibe como arreglo/diccionario. Acepta ambos por robustez (ver `mascotas` en datos-cze).
- **Fragmentos HTML** (`integrantesHtml`, `hospedantesHtml`): la API agrega la clave sin sufijo
  (`integrantes`, `hospedantes`) con el contenido estructurado: `<tr>/<td>` → arreglo de filas (arreglo
  de strings); `<span>/<li>/<p>` → arreglo de strings; otro → string de texto plano. Usa la clave sin
  sufijo y, por robustez, acepta que ya venga estructurada.
- **Clases CSS inyectadas** (`nadieEnCasaChecked: "checked"`): compara el string: `val("x") == "checked"`.
- **Colores**: copia los hex del CSS. Gradientes → color sólido del primer stop.
- **Tablas**: `table(columns: (...), stroke: ..., inset: ..., fill: (x, y) => ...)`; encabezado con
  `table.header(...)`; filas desde datos con `..filas.flatten()`.
- **Bloques con borde/fondo**: `block(width: 100%, inset:, stroke:, fill:, radius:, breakable: true|false)`.
- **Posicionamiento absoluto** (carnets, certificados con fondo): `place(top + left, dx:, dy:, ...)`
  dentro de un `block(width: W, height: H)`. CSS `px` → `pt` × 0.75.
- **Saltos de página**: `pagebreak()`; `block(breakable: false)` equivale a `page-break-inside: avoid`;
  `block(sticky: true)` mantiene un título junto a lo que sigue.
- **Letter spacing / mayúsculas**: `text(tracking: 2pt)`, `upper(...)`.
- **Fechas** ISO → `dd-mm-aaaa`: `t.split("T").at(0).split("-").rev().join("-")`.
- **Escapes en markup**: `\#`, `\$`, `\*`, `\_`, `\@`. Un `#` literal en `[...]` se escribe `\#`.
- **Nombres**: no llames `v` a una función tuya (es el espaciado vertical de Typst). Usa `val`.

- **Rendimiento**: usa `#set text(hyphenate: false)` (Chromium tampoco parte palabras); la silabación
  duplica o triplica el tiempo de render en tablas grandes. Prefiere columnas de ancho fijo a `auto`
  cuando la tabla es larga.
- **Fuentes propias**: Typst registra Asap Condensed bajo la familia `"Asap"`; gobCL es `"gobCL"`.
  Para que las líneas de texto caigan donde las deja Chromium con Arial:
  `#set text(top-edge: 0.905em, bottom-edge: -0.212em)` y `#set par(leading: 0.033em)` (métricas hhea).
- **Fotos en base64 crudo**: v2 hacía `src="data:image/jpg;base64,{foto}"`, así que el consumidor manda
  solo el base64. La API también convierte eso al diccionario `(format, bytes)` (detecta jpg/png/gif/webp
  por la firma), así que la plantilla usa el mismo `if type(img) == dictionary` de arriba.
- **Variables con guion** (`{razon-social}`): válidas en v2; en Typst `d.at("razon-social")`.

## 3. Trampas conocidas

- Chromium renderiza los `pt` del CSS un poco más grandes de lo que uno espera; si Typst se ve
  "pequeño", sube tamaños e insets un 10-20 % hasta calzar visualmente con las capturas.
- `networkidle0` y `setTimeout` en la plantilla HTML no existen en Typst: si el script de v2 rellenaba
  algo tarde, reprodúcelo en la plantilla.
- Si el HTML de v2 tiene defectos (placeholders sin reemplazar, spans vacíos, elementos que nunca se
  muestran), no los copies: corrígelos y anótalos en el informe.
- Fuentes `url('fuentes/x.woff')` en v2 nunca cargaban (ruta relativa con `setContent`); en v3 sí
  cargan desde `fonts/`, así que el PDF de v3 puede verse *mejor* que el de v2. Anótalo.
- La API cachea las plantillas al arrancar; `bench-render` siempre lee el disco.

## 4. Entregables por plantilla

- `plantillas/<carpeta>/<archivo>.typ` con un comentario inicial: origen, variables, decisiones.
- `plantillas/<carpeta>/assets/*` con nombres descriptivos (borra los `-imgN` sin usar).
- `scripts/ejemplo-<archivo>.json`.
- `tmp/comparacion/<archivo>/` con las capturas finales de ambos motores.
- Un párrafo de informe: fidelidad lograda, diferencias que quedan, defectos encontrados en v2,
  supuestos sobre el payload.
