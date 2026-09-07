// Benchmark del motor de v2 (Chromium vía Puppeteer) con la MISMA plantilla y los MISMOS datos
// que el bench de v3. Importa el browser_service compilado en build/ sin tocar el repo.
import fs from 'node:fs/promises'
import path from 'node:path'
const V2 = process.env.V2_DIR
const { renderizarPdf, cerrarBrowser } = await import(path.join(V2, 'build/app/services/browser_service.js'))

const payload = JSON.parse(await fs.readFile(process.env.PAYLOAD, 'utf8'))
const datos = { ...payload.DatosCredencial }
// Igual que v2: si GenerarQR, la API genera un data URL PNG bajo QRTag (default "QR")
if (payload.GenerarQR) {
  const { default: QRCode } = await import(path.join(V2, 'node_modules/qrcode/lib/index.js'))
  datos[payload.QRTag || 'QR'] = await QRCode.toDataURL('https://storage.example/contenedor/documento', { errorCorrectionLevel: 'M', type: 'image/png', width: 200, margin: 1 })
}
const PLANTILLA = process.env.PLANTILLA || 'mascotas/datos-cze.html'
// El consumidor real envía las estructuras serializadas para el <script> de la plantilla HTML
for (const k of ['mascotas', 'secciones']) if (k in datos && typeof datos[k] !== 'string') datos[k] = JSON.stringify(datos[k])
let html = await fs.readFile(path.join(V2, 'public/plantillas', PLANTILLA), 'utf8')
for (const [k, v] of Object.entries(datos)) html = html.replace(new RegExp(`{${k}}`, 'g'), String(v))

const OPCIONES = { format: 'letter', printBackground: true, preferCSSPageSize: true, margin: { bottom: '0 px' } }
const N = Number(process.env.N || 20)
const PAR = Number(process.env.PAR || 8)
const ms = (t) => `${t.toFixed(1)} ms`
const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return s[Math.round((s.length - 1) * p)] }

let t = performance.now()
let pdf = await renderizarPdf(html, OPCIONES)
console.log(`primer render (frío, incluye launch de Chromium): ${ms(performance.now() - t)} -> ${pdf.length} bytes`)

const tiempos = []
for (let i = 0; i < N; i++) { const t0 = performance.now(); pdf = await renderizarPdf(html, OPCIONES); tiempos.push(performance.now() - t0) }
console.log(`secuencial x${N}: min ${ms(Math.min(...tiempos))} | p50 ${ms(pct(tiempos, 0.5))} | p95 ${ms(pct(tiempos, 0.95))} | max ${ms(Math.max(...tiempos))}`)

t = performance.now()
const res = await Promise.allSettled(Array.from({ length: PAR }, () => renderizarPdf(html, OPCIONES)))
const total = performance.now() - t
console.log(`paralelo x${PAR} simultáneos: ${res.filter(r => r.status === 'fulfilled').length}/${PAR} ok en ${ms(total)} (≈ ${ms(total / PAR)} por documento efectivo)`)

await fs.writeFile(process.env.OUT, pdf)
console.log(`PDF de muestra: ${process.env.OUT} (${pdf.length} bytes)`)
await cerrarBrowser()
