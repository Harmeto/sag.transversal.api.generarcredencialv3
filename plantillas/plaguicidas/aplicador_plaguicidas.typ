// Plantilla "Credencial Aplicador de Plaguicidas Agrícolas reconocido" migrada desde v2
// (plaguicidas/aplicador_plaguicidas.html) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que v2:
//   folio, vigencia, credencial_nombres, credencial_rut, capacitador, region,
//   firmante_nombre, firmante_cargo,
//   foto: en v2 llega como base64 CRUDO (la plantilla HTML antepone
//         `data:image/jpg;base64,`). Aquí se acepta tanto ese base64 crudo
//         (se decodifica en Typst y se detecta jpg/png/gif/webp por la firma)
//         como un data URL (la API lo entrega ya convertido a (format, bytes)).
//         Si no es decodificable queda el marco verde vacío, como en v2.
//
// Tipografía: el CSS de v2 pedía `ui-sans-serif`, que el Chromium del servidor
// no conoce, así que el PDF salía en la fuente por defecto (Times, serif). El
// @font-face con Asap Condensed (sustituta de ITC Officina Sans) estaba
// comentado. En v3 las fuentes de `fonts/` sí cargan, por lo que la credencial
// usa Asap Condensed (Typst la registra como familia "Asap"), que era la
// intención del diseño; `usar-asap = false` reproduce el aspecto serif de v2.
// La cabecera "MINISTERIO DE AGRICULTURA" siempre fue Arial.
//
// Defecto de v2 que no se copia: el pie verde ("El SAG no se hace responsable…")
// tenía `position: relative; top: 250px` y caía en una SEGUNDA página casi vacía.
// Aquí va al pie de la misma página.
//
// Geometría: 1 px de CSS = 0,75 pt (aquí Chromium no encogía la página). Los
// baselines de cada línea (`base(...)`) se tomaron del PDF que produce Chromium;
// la caja de línea se construye con las métricas hhea de la fuente usada para
// que el texto caiga exactamente en esos baselines.

#import sys: inputs
#let d = inputs.at("datos", default: (:))
#let usar-asap = true

// ---------- Utilidades ----------
#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))

// Decodificador base64 en Typst puro (Typst 0.15 no trae uno). ~1,5 µs por carácter.
#let b64-decodificar(s) = {
  let alfa = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
  let tabla = (:)
  for (i, c) in alfa.clusters().enumerate() { tabla.insert(c, i) }
  tabla.insert("-", 62)
  tabla.insert("_", 63)
  let out = ()
  let acc = 0
  let nbits = 0
  for c in s.clusters() {
    let v = tabla.at(c, default: none)
    if v == none { continue }
    acc = acc * 64 + v
    nbits += 6
    if nbits >= 8 {
      nbits -= 8
      let p = calc.pow(2, nbits)
      out.push(calc.quo(acc, p))
      acc = calc.rem(acc, p)
    }
  }
  bytes(out)
}
// Formato de imagen según los primeros bytes; none si no es una imagen conocida.
#let formato-imagen(b) = {
  if b.len() < 12 { return none }
  let a = array(b.slice(0, 12))
  if a.at(0) == 0xff and a.at(1) == 0xd8 and a.at(2) == 0xff { "jpg" }
  else if a.slice(0, 4) == (0x89, 0x50, 0x4e, 0x47) { "png" }
  else if a.slice(0, 3) == (0x47, 0x49, 0x46) { "gif" }
  else if a.slice(0, 4) == (0x52, 0x49, 0x46, 0x46) and a.slice(8, 12) == (0x57, 0x45, 0x42, 0x50) { "webp" }
  else { none }
}
// Imagen que viene en los datos: diccionario (data URL ya convertido por la API)
// o string base64 crudo (contrato de v2). Devuelve (bytes, format) o none.
#let imagen-datos(k) = {
  let x = d.at(k, default: none)
  if type(x) == dictionary and "bytes" in x { (bytes: x.bytes, format: x.format) }
  else if type(x) == str and x.trim() != "" and not x.starts-with("data:") and not x.starts-with("http") {
    let b = b64-decodificar(x)
    let f = formato-imagen(b)
    if f == none { none } else { (bytes: b, format: f) }
  } else { none }
}

// ---------- Geometría y tipografía ----------
#let px(n) = n * 0.75pt
#let ancho = px(793.7)                 // 21 cm, ancho de .pagina (centrado en la hoja)
#let izq = (612pt - ancho) / 2
#let verde = rgb("#77c37b")
#let arial = ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans")
#let serif = ("Times New Roman", "Liberation Serif", "Times", "DejaVu Serif")
#let fuente-credencial = if usar-asap { ("Asap", ..serif) } else { serif }
// Métricas hhea (ascendente / descendente en em) de la fuente de la credencial.
#let asc = if usar-asap { 0.934 } else { 0.891 }
#let desc = if usar-asap { 0.212 } else { 0.216 }
// Borde superior / inferior de una línea de tamaño `s` cuyo baseline está en `b`.
#let arriba(b, s) = b - asc * s
#let abajo(b, s) = b + desc * s
// Interlineado para que el paso entre líneas sea `paso` con tamaño `s`.
#let paso(paso, s) = paso - (asc + desc) * s
#let pie-alto = 36pt

#set page(
  paper: "us-letter",
  margin: (left: izq, right: izq, top: 0pt, bottom: pie-alto + px(8)),
  footer-descent: 0pt,
  // #texto-pie: franja verde con el descargo, al pie de la misma página
  footer: block(width: 100%, height: pie-alto, fill: verde, inset: (x: px(5), top: arriba(14.4pt, px(18))),
    par(justify: true, leading: paso(16.5pt, px(18)),
      text(font: fuente-credencial, size: px(18), weight: "bold", fill: white,
        top-edge: asc * 1em, bottom-edge: -desc * 1em)[
        El SAG no se hace responsable de la calidad y eficacia de los trabajos efectuados por el aplicador ni de los daños que éste pudiere provocar en su desempeño.
      ])),
)
#set text(font: fuente-credencial, size: px(16), top-edge: asc * 1em, bottom-edge: -desc * 1em)
#set par(leading: 0.05em, spacing: 0pt, justify: false)
#set block(above: 0pt, below: 0pt)
#set smartquote(enabled: false)

// ---------- Cabecera: franja verde, títulos y logo encima (z-index 9) ----------
#block(width: 100%, height: 225pt, {
  // #headinferior
  // Cada línea se ancla al baseline que tenía en v2 (título en dos líneas con la
  // fuente serif; con Asap Condensed cabe en una y el folio no se sube).
  let linea-conthi(base, s, cuerpo) = place(top + left, dx: px(160), dy: arriba(base, s) - 101.2pt,
    block(width: px(639), align(center, text(size: s, cuerpo))))
  place(top + left, dy: 101.2pt, block(width: 100%, height: px(165), fill: verde, {
    set text(fill: white, weight: "bold")
    linea-conthi(124pt, px(26), par(leading: paso(23.3pt, px(26)))[Credencial Aplicador de Plaguicidas Agrícolas reconocido])
    linea-conthi(185.2pt, px(26))[Folio N° #val("folio")]
    linea-conthi(211.7pt, px(24))[Vigente hasta: #val("vigencia")]
  }))
  // #headsuperior (Arial: caja de línea 0.905 / 0.212 em)
  place(top + left, dx: px(155), dy: 41pt,
    block(width: px(639), align(center,
      text(font: arial, size: px(28), weight: "bold", top-edge: 0.905em, bottom-edge: -0.212em)[MINISTERIO DE AGRICULTURA])))
  place(top + left, dx: px(155), dy: 79.8pt,
    block(width: px(639), align(center,
      text(font: arial, size: px(15), weight: "bold", top-edge: 0.905em, bottom-edge: -0.212em)[SERVICIO AGRÍCOLA Y GANADERO])))
  // #logo
  place(top + left, dx: 3.75pt, dy: 23.2pt, image("assets/logo-sag-gobierno.png", width: px(180)))
})

// ---------- Foto con marco verde (#cont-foto img, 150 px + borde) ----------
#v(27pt)
#let foto = imagen-datos("foto")
#align(center, block(width: 135pt, height: 135pt, fill: verde, inset: 11.25pt,
  if foto != none { image(foto.bytes, format: foto.format, width: 112.5pt, height: 112.5pt, fit: "cover") }
))

// ---------- Nombre y RUT (#datoUsr) ----------
#v(arriba(411pt, px(19)) - 387pt)
#align(center, {
  set text(size: px(19), weight: "bold")
  block(val("credencial_nombres"))
  v(arriba(427.7pt, px(19)) - abajo(411pt, px(19)))
  block(val("credencial_rut"))
})

// ---------- Glosa (#disclaimer) ----------
#v(arriba(481.6pt, px(18)) - abajo(427.7pt, px(19)))
#pad(x: px(20), par(justify: true, leading: paso(16.5pt, px(18)), text(size: px(18))[
  La persona identificada en esta credencial, ha aprobado el "curso general de capacitación sobre manejo y uso de plaguicidas agrícolas" impartido bajo la normativa del SAG por el proveedor de capacitación autorizado #text(weight: "bold", val("capacitador")), en #text(weight: "bold", val("region")).
]))

// ---------- Firmante (#timbre: 400 px de ancho, centrado y corrido 196 px a la derecha) ----------
// Se ancla a la hoja (no al flujo) para que quede donde v2 lo dejaba aunque la glosa
// ocupe menos líneas con Asap Condensed que con la fuente serif de v2.
#place(top + left, dx: (ancho - px(400)) / 2 + px(196), dy: arriba(651.8pt, px(20)),
  block(width: px(400), align(center, {
    set text(size: px(20), weight: "bold")
    set par(leading: paso(17.2pt, px(20)))
    block(val("firmante_nombre"))
    block(val("firmante_cargo"))
  })))
