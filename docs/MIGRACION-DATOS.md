# Migración de datos de v2 (SQL Server) a v3 (PostgreSQL)

Traspasa las tres tablas del generador de credenciales conservando los identificadores.
Eso no es un detalle: los consumidores envían `PlantillaId` (el BFF de Emergencias
Pecuarias lo lleva en una variable de entorno) y los `guid` de las credenciales ya están
impresos en códigos QR que la gente tiene en la mano.

Son dos pasos: exportar desde v2 a un archivo, e importar ese archivo en v3.

---

## 1. Exportar desde v2

```bash
V2_DIR=../sag.transversal.api.generarcredencialV2 \
DB_HOST=<servidor> DB_PORT=1433 DB_USER=<usuario> DB_PASSWORD=<clave> \
DB_DATABASE=GeneradorCredenciales OUT=tmp/export-v2.json \
node scripts/exportar-v2.mjs
```

El script no agrega dependencias a este repositorio: reutiliza el driver `tedious` que v2
ya tiene instalado. Sólo lee, nunca escribe en SQL Server.

Produce un JSON con las categorías, las plantillas y las credenciales. Las fechas se
guardan como la "hora de pared" que tenía v2 (`2023-04-24T10:15:30.123`, sin desfase),
porque la columna `fechaCreacion` es un `datetime` sin zona.

---

## 2. Importar en v3

```bash
./target/release/migrar-datos --archivo tmp/export-v2.json --limpiar-catalogos
```

La conexión sale de la misma configuración que usa la API: `.env` en local, Consul en los
ambientes. Así el binario sirve tanto desde un puesto de trabajo con port-forward como
desde dentro del pod.

| Opción | Para qué |
|---|---|
| `--archivo <ruta>` | El JSON del paso 1 (obligatorio) |
| `--zona <tz>` | Zona en que se interpretan las fechas de v2. Por omisión `America/Santiago`, que es el `TZ` del contenedor de v2 |
| `--dry-run` | Hace todo el trabajo y deshace la transacción al final. Sirve para ver los avisos antes de tocar nada |
| `--limpiar-catalogos` | Borra las categorías y plantillas de la semilla antes de cargar las de v2. Es el modo normal en un traspaso real |

### Por qué hace falta `--limpiar-catalogos`

Una base v3 recién creada trae categorías y plantillas de ejemplo, con ids que no son los
de producción. Si se cargaran encima los de v2, dos categorías distintas quedarían con el
mismo id. La herramienta detecta ese choque y **se niega a continuar** explicando cuál es,
en vez de dejar la base inconsistente. Con `--limpiar-catalogos` reemplaza los catálogos
por los de v2. Nunca borra categorías que ya tengan credenciales asociadas.

### Qué garantiza

- **Ids intactos** en categorías, plantillas y credenciales, incluidos los saltos (si en v2
  la plantilla 4 fue eliminada, en v3 tampoco existe).
- **Rutas traducidas**: `mascotas/datos-cze.html` pasa a `mascotas/datos-cze.typ`, y el
  motor queda en `typst`.
- **Fechas equivalentes**: la hora de pared que mostraba v2 es la que devuelve v3. Los dos
  casos límite del cambio de hora en Chile se resuelven y se avisan: una hora repetida toma
  la primera ocurrencia, y una hora que no existió se corre sesenta minutos.
- **Secuencias corregidas**: tras cargar ids explícitos se ajusta el contador de cada tabla,
  para que el siguiente registro nuevo no choque con uno existente.
- **Idempotencia**: correrla dos veces no duplica nada.
- **Todo o nada**: una sola transacción. Si algo falla, la base queda como estaba.

### Avisos que conviene leer

- **Plantillas sin equivalente**: si v2 tiene una plantilla que todavía no fue migrada a
  Typst, se carga igual (para no romper el id que usa el consumidor) pero se avisa. Esa
  plantilla responderá error hasta que exista su archivo `.typ`.
- **Fechas en el borde del cambio de hora**: se listan una por una con lo que se hizo.

---

## 3. Orden recomendado para el traspaso

1. Crear la base PostgreSQL del ambiente y las keys de Consul (ver [CONSUL.md](CONSUL.md)).
2. Desplegar v3. Al arrancar aplica sus migraciones y deja el esquema listo.
3. Exportar desde v2 y correr la importación con `--dry-run`. Revisar los avisos.
4. Correr la importación de verdad con `--limpiar-catalogos`.
5. Comprobar que un `guid` conocido responde igual que en v2, y que un `PlantillaId` real
   genera su documento.
6. Recién entonces apuntar los consumidores al prefijo `/api/v3/...`.

Mientras v2 siga emitiendo credenciales, la importación se puede repetir: es idempotente y
sólo agrega las nuevas. Para el corte definitivo conviene detener las escrituras de v2,
correrla una última vez y cambiar los consumidores.

---

## Verificación hecha

El circuito completo se probó contra un SQL Server local con el esquema exacto de v2 y
datos que incluyen mayúsculas en los `guid`, ids con saltos, una plantilla sin equivalente
en v3 y las dos horas problemáticas del cambio de hora. Resultado: ids conservados, fechas
que vuelven al mismo valor que mostraba v2, secuencias corregidas, segunda corrida sin
duplicados, y la API generando documentos con los `PlantillaId` originales.
