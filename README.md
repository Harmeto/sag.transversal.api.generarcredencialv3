# sag.transversal.api.generarcredencialV3

> Generación de documentos PDF del SAG a partir de plantillas, **sin Chromium**: Rust + Typst + PostgreSQL.
> Prueba de concepto local con dos plantillas migradas: **Datos CZE** (solicitud de exportación de perros y gatos, consumida por Procesos Digitales / appweb ciudadano) y **Bioseguridad Traspatio** (pauta F-VYC-VIS-PP-005, consumida por el BFF de Emergencias Pecuarias).

Estado: **prueba de concepto funcional en local**. No desplegada, no conectada a recursos del SAG.

---

## 1. Por qué existe

v2 (`sag.transversal.api.generarcredencialV2`, AdonisJS + Puppeteer) renderiza cada documento en un Chromium. Aun con el navegador compartido, el costo por documento lo fija el motor: parseo de HTML de 200 KB, espera `networkidle0` (mínimo 500 ms), scripts de la plantilla con `setTimeout`, y un proceso de cientos de MB en el pod.

v3 cambia el motor, no el contrato. Las plantillas se escriben en [Typst](https://typst.app) (un lenguaje de composición tipográfica compilado, escrito en Rust) y se compilan **dentro del proceso** de la API. No hay navegador, ni procesos hijos, ni red durante el render.

### Medición (mismo equipo, misma plantilla Datos CZE, mismos datos: 2 mascotas, 3 páginas)

| Métrica | v2 · Chromium (Puppeteer, navegador compartido) | v3 · Typst en proceso |
|---|---:|---:|
| Primer documento (frío) | 3 231 ms | 37 ms |
| Secuencial, p50 | 2 271 ms | 31 ms |
| Secuencial, p95 | 2 331 ms | 34 ms |
| 8 simultáneos, tiempo total | 2 437 ms | 59 ms |
| Tamaño del PDF | 799 KB | 102 KB |
| Memoria del motor | cientos de MB por navegador | decenas de MB, en el mismo proceso |

Segunda plantilla, **Bioseguridad Traspatio** (7 secciones, 15 preguntas, QR, marca de agua, encabezado repetido):

| Métrica | v2 · Chromium | v3 · Typst |
|---|---:|---:|
| Primer documento (frío) | 1 680 ms | 46 ms |
| Secuencial, p50 | 2 103 ms | 36 ms |
| 8 simultáneos, tiempo total | 2 264 ms | 100 ms |
| Tamaño del PDF | 503 KB | 240 KB |

Cómo se midió: `scripts/bench-chromium-v2.mjs` (importa `build/app/services/browser_service.js` de v2 y renderiza `datos-cze.html` con los mismos datos; se ejecuta con `NODE_ENV=local V2_DIR=<ruta v2> PUPPETEER_CACHE_DIR=<v2>/.cache/puppeteer PAYLOAD=scripts/ejemplo-datos-cze.json OUT=tmp/chromium.pdf node scripts/bench-chromium-v2.mjs`; `PLANTILLA=emergencias-pecuarias/bioseguridad-traspatio.html` para la otra) y `bench-render` de este repo. Ambos con 20/50 iteraciones y 8 en paralelo. Los números son de un Mac local; en un pod el orden de magnitud se mantiene.

---

## 2. Qué se conserva de v2 (contrato)

| Aspecto | v2 | v3 |
|---|---|---|
| Prefijo | `/api/v2/transversal/credencial` | `/api/v3/transversal/credencial` |
| Operaciones | health, categoria, plantilla, credencial (index/show), generarCredencial, generarPreview | Idénticas |
| Payload de generación | `PlantillaId`, `DatosCredencial`, `GenerarQR`, `QRTag` | Idéntico |
| Respuesta 201 | `{ guid, categoriaId, url, fechaCreacion }` | Idéntica (fecha ISO con desfase de Santiago) |
| Listados | array, o `{ meta, data }` de Lucid con `paginate=true` | Idéntico, incluidas `firstPageUrl`, `nextPageUrl`… |
| Query de listados | `sort`, `paginate`, `page`, `limit` con los mismos ids de `sort` | Idéntico |
| Errores de validación | `400` `[{ message, rule, field }]` | Idéntico |
| No encontrado | `404 { title: "Not Found", status: 404 }` | Idéntico |
| Ruta inexistente | `404 { msg: "Not found" }` | Idéntico |
| Preview | PDF inline `preview-<id>.pdf` | Idéntico |
| Nombre de contenedor y de documento | `snake_case(categoria)` / `snake_case(plantilla)_aaaa-mm-dd_uuid` | Idéntico (forman la URL pública ya impresa en QR) |
| Modelo de datos | Categoria, Plantilla, Credencial | Mismas entidades y campos, en PostgreSQL (snake_case, `uuid`, `timestamptz`) |

Diferencias deliberadas:

- **Fallo de render responde 500**, no 404. En v2 un timeout de Chromium se confundía con "plantilla no existe".
- `DatosCredencial` puede traer estructuras **como arreglo/objeto o como string JSON** (el consumidor actual de Datos CZE envía `mascotas` serializado para el `<script>` de la plantilla HTML). v3 acepta ambos.
- Si se pide QR, además del data URL bajo `QRTag` la plantilla recibe `__qr_svg` (SVG en texto, que Typst incrusta directo) y `__qr_url`.
- Cualquier valor string con forma de **data URL de imagen** (`data:image/png;base64,...`) llega a la plantilla como `(format, mime, bytes)`, para que Typst lo incruste con `image(x.bytes, format: x.format)`. Así el QR que v2 entregaba como data URL sigue funcionando sin cambiar el contrato.
- `plantilla.motor` (`typst` | `chromium`) permite, si se quisiera, convivir con un adaptador Chromium y migrar plantilla a plantilla. No se serializa en la respuesta.

---

## 3. Arquitectura

```
HTTP (axum)  ──►  services::generar_credencial  ──►  render::RenderPool ──► render::typst_renderer (Typst en proceso)
                       │                                  (semáforo N = núcleos, spawn_blocking)
                       ├──►  storage::Storage  ──► storage::local (disco + /archivos/…)   [Azure Blob: pendiente]
                       └──►  repos::*          ──► PostgreSQL (sqlx)
```

- `src/http/`: rutas, validación con las mismas reglas que VineJS, paginación con la forma de Lucid.
- `src/services/generar_credencial.rs`: caso de uso puro (no conoce HTTP). Un worker asíncrono podría invocarlo igual.
- `src/render/`: puerto `Renderer` + adaptador Typst. Un solo motor compartido, fuentes cargadas una vez, plantillas precalentadas al arrancar.
- `src/storage/`: puerto `Storage` + adaptador local.
- `src/repos/`: consultas sqlx y serialización con los nombres camelCase de v2.
- `plantillas/`: plantillas Typst. Reciben `sys.inputs.datos` con el objeto `DatosCredencial`.
- `migrations/`: esquema y semilla (se aplican al arrancar).

Concurrencia: los renders son CPU-bound y corren en hilos bloqueantes con un semáforo. Un pico de tráfico **encola**, no abre N navegadores. La memoria del pod tiene techo.

---

## 4. Plantillas migradas

### Datos CZE

`plantillas/mascotas/datos-cze.typ` reemplaza a `public/plantillas/mascotas/datos-cze.html` de v2. Lo que en HTML hacía JavaScript (clonar la página 2 por mascota, elegir la tabla de identificación según `microchip.tipo`, formatear fechas ISO a `dd-mm-aaaa`, numerar páginas) lo hace el lenguaje de la plantilla: loops, condicionales y contadores de página.

Mismas variables que v2 (`numero_solicitud`, `fecha_solicitud`, datos de solicitante, exportador, viaje, emisión y generador) y el arreglo `mascotas` con `microchip`, `veterinario`, `vacunas` y `desparasitaciones`.

Fidelidad visual: misma estructura, colores, tablas y paginación. No es idéntica al píxel (Chromium y Typst no comparten motor de layout); la validación final la debe hacer el dueño del negocio. Mejora respecto de v2: sin mascota o sin microchip, v2 dejaba placeholders `{...}` sin reemplazar; v3 muestra celdas vacías o "Sin registros".

### Bioseguridad Traspatio

`plantillas/emergencias-pecuarias/bioseguridad-traspatio.typ` reemplaza a `emergencias-pecuarias/bioseguridad-traspatio.html`. Recibe exactamente el `DatosCredencial` que arma `BioseguridadMapper.toDatosCredencialTraspatio` en `sag.emergenciaspecuarias.bff`: datos del establecimiento, `secciones` como **string JSON** (`[{ titulo, preguntas: [{ numero, pregunta, respuesta }] } | { titulo, observacion }]`) y el QR. Los logos que v2 llevaba en base64 dentro del HTML (194 KB) viven en `plantillas/emergencias-pecuarias/assets/` y la marca de agua es un PNG con la opacidad ya aplicada.

Hallazgos sobre v2 al migrarla:

- Los `<span class="pag-actual">` y `pag-total` del encabezado nunca se rellenaban: el PDF de producción dice "Página  de ". v3 numera con contadores reales.
- El BFF envía `GenerarQR: true` con `QRTag: <urlVerificacion>` (una URL, no el nombre del tag). v2 deja el QR bajo esa URL como clave, la plantilla busca `{QR}` y por tanto **el QR nunca aparece en el PDF de producción**. Además el QR que genera la API codifica la URL del blob, no la de verificación. v3 reproduce el comportamiento fielmente (con `QRTag: "QR"` el QR sí aparece, ver `scripts/ejemplo-bioseguridad-traspatio.json`); corregirlo es un cambio en el BFF.
- Con muchas preguntas el `<thead>` del `print-wrapper` hace que Chromium desborde a una segunda página con una sola fila; Typst pagina el contenido de forma natural y repite el encabezado.

---

## 5. Correr en local

Requisitos: Rust (instalado con rustup en `~/.cargo/bin`), Docker con el contenedor `sag-postgres` (o el `docker-compose.yml` de este repo).

```bash
# 1) compilar (ya compilado en target/release)
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release

# 2) levantar (crea la base generador_credenciales_v3 si no existe y aplica migraciones)
./scripts/run-local.sh

# 3) prueba de humo: health, catálogos, preview y generación persistida
./scripts/smoke.sh

# 4) benchmark del motor (sin HTTP ni base de datos)
./target/release/bench-render mascotas/datos-cze.typ scripts/ejemplo-datos-cze.json 50 8
./target/release/bench-render emergencias-pecuarias/bioseguridad-traspatio.typ scripts/ejemplo-bioseguridad-traspatio.json 50 8
```

Ejemplo de generación (mismo payload que v2):

```bash
curl -sS -H 'Content-Type: application/json' \
  --data @scripts/ejemplo-datos-cze.json \
  http://localhost:3345/api/v3/transversal/credencial/generarCredencial
```

Variables de entorno: ver `.env.example`. El puerto local es `3345` porque `3334` está ocupado en este equipo.

Las plantillas se leen y cachean al arrancar (el motor las precalienta). Si editas un `.typ`, reinicia la API para que lo tome; `bench-render` siempre lee la versión en disco.

Tests unitarios: `cargo test` (nombres de contenedor/documento, normalización de JSON embebido).

---

## 6. Cómo agregar una plantilla

1. Escribir `plantillas/<categoria>/<nombre>.typ`. Leer los datos con `#let d = sys.inputs.datos`.
2. Si usa fuentes propias, copiarlas a `fonts/`.
3. Insertar la fila en `plantilla` (`ruta`, `categoria_id`, `motor = 'typst'`) mediante una migración `migrations/NNNN_*.sql`.
4. Probar con `bench-render <ruta> <json>` y validar el PDF con el negocio.

---

## 7. Pendiente para llevarlo a producción

- **Adaptador Azure Blob Storage** (`storage::azure`): misma convención de contenedor/nombre, `Content-Type: application/pdf`, creación del contenedor si no existe. El puerto ya está definido; falta el adaptador y su prueba (Azurite en Docker sirve para probarlo sin recursos del SAG).
- **Consul**: cargar `DATABASE_URL` y la conexión de storage como hace v2 (`start/consul.ts`). En local se usa `.env`.
- **Swagger** (v2 lo genera desde la colección Postman).
- **Migración de datos** desde MSSQL: `Categoria`, `Plantilla` y `Credencial` con sus Id originales. Atención a `fechaCreacion`: v2 guarda `datetime` sin zona escrito con `TZ=America/Santiago`; verificar si el histórico está en hora local o UTC antes de copiarlo a `timestamptz`.
- **Las otras 13 plantillas.** Las 10 estáticas son trabajo de reescritura. Bioseguridad plantel es casi idéntica a traspatio (misma estructura, otro código de formulario y otros campos de cabecera). Mosca de la fruta recibe fragmentos HTML (`integrantesHtml`, `hospedantesHtml`) y requiere acordar datos estructurados con ese equipo.
- **Probes y HPA** en `deploy-openshift.yaml`: readiness sobre `/health`, escalado por CPU (ahora sí es la métrica correcta, porque el render es CPU puro).
- **Fuentes en la imagen**: Liberation Sans reemplaza a Arial en Linux (métricamente compatible). Si el negocio exige Arial, hay que licenciarla e incluirla en `fonts/`.
