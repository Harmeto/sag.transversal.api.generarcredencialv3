// Exporta las tablas Categoria, Plantilla y Credencial de la base SQL Server de v2
// a un JSON que consume `migrar-datos` (el binario de v3).
//
// No agrega dependencias a este repositorio: reutiliza el driver `tedious` que v2 ya
// tiene instalado, apuntando a su node_modules.
//
// Uso:
//   V2_DIR=../sag.transversal.api.generarcredencialV2 \
//   DB_HOST=... DB_PORT=1433 DB_USER=... DB_PASSWORD=... DB_DATABASE=GeneradorCredenciales \
//   OUT=tmp/export-v2.json node scripts/exportar-v2.mjs
//
// Nota sobre fechas: `fechaCreacion` es DATETIME sin zona. Se lee en UTC (useUTC) y se
// escribe tal cual quedó guardada ("hora de pared", sin desfase). Quien decide en qué
// zona interpretarla es `migrar-datos`, con --zona (por omisión America/Santiago, que
// es el TZ del contenedor de v2).

import fs from 'node:fs/promises'
import path from 'node:path'

const V2 = process.env.V2_DIR ?? '../sag.transversal.api.generarcredencialV2'
const OUT = process.env.OUT ?? 'tmp/export-v2.json'
const { Connection, Request, TYPES } = await import(path.resolve(V2, 'node_modules/tedious/lib/tedious.js'))

const config = {
  server: process.env.DB_HOST ?? 'localhost',
  authentication: {
    type: 'default',
    options: { userName: process.env.DB_USER, password: process.env.DB_PASSWORD },
  },
  options: {
    port: Number(process.env.DB_PORT ?? 1433),
    database: process.env.DB_DATABASE ?? 'GeneradorCredenciales',
    encrypt: process.env.DB_ENCRYPT !== 'false',
    trustServerCertificate: process.env.DB_TRUST_CERT !== 'false',
    useUTC: true,
    rowCollectionOnRequestCompletion: true,
    requestTimeout: 120000,
  },
}

const conexion = new Connection(config)

function consultar(sql) {
  return new Promise((resolve, reject) => {
    const req = new Request(sql, (err, _n, filas) => {
      if (err) return reject(err)
      resolve(
        (filas ?? []).map((fila) =>
          Object.fromEntries(fila.map((c) => [c.metadata.colName, c.value])),
        ),
      )
    })
    conexion.execSql(req)
  })
}

/** DATETIME sin zona -> "aaaa-mm-ddThh:mm:ss.mmm" tal como está guardado. */
function fechaDePared(valor) {
  if (valor == null) return null
  if (typeof valor === 'string') return valor
  return valor.toISOString().replace('Z', '').slice(0, 23)
}

await new Promise((resolve, reject) => {
  conexion.on('connect', (err) => (err ? reject(err) : resolve()))
  conexion.connect()
})

const categorias = await consultar('SELECT Id, Nombre FROM Categoria ORDER BY Id')
const plantillas = await consultar('SELECT Id, Nombre, Ruta, CategoriaId FROM Plantilla ORDER BY Id')
const credenciales = await consultar(
  'SELECT Id, CategoriaId, url, fechaCreacion FROM Credencial ORDER BY fechaCreacion',
)

const salida = {
  origen: {
    servidor: config.server,
    base: config.options.database,
    exportadoEn: new Date().toISOString(),
  },
  categorias: categorias.map((c) => ({ id: c.Id, nombre: c.Nombre })),
  plantillas: plantillas.map((p) => ({
    id: p.Id,
    nombre: p.Nombre,
    ruta: p.Ruta,
    categoriaId: p.CategoriaId,
  })),
  credenciales: credenciales.map((c) => ({
    id: String(c.Id).toLowerCase(),
    categoriaId: c.CategoriaId,
    url: c.url,
    fechaCreacion: fechaDePared(c.fechaCreacion),
  })),
}

await fs.mkdir(path.dirname(OUT), { recursive: true })
await fs.writeFile(OUT, JSON.stringify(salida, null, 2))
console.log(
  `${OUT}: ${salida.categorias.length} categorías, ${salida.plantillas.length} plantillas, ${salida.credenciales.length} credenciales`,
)
conexion.close()
