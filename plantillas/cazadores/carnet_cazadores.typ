// Plantilla "Carnet de Cazador" migrada desde v2 (cazadores/carnet_cazadores.html) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que v2:
//   carnet_tipo, folio, vigencia_desde, vigencia_hasta, nombres, n_documento,
//   nacionalidad, pais, direccion, firmante_nombre, firmante_cargo,
//   foto: en v2 llega como base64 CRUDO (la plantilla HTML antepone
//         `data:image/jpg;base64,`). Aquí se acepta tanto ese base64 crudo
//         (se decodifica en Typst y se detecta jpg/png/gif/webp por la firma)
//         como un data URL (la API lo entrega ya convertido a (format, bytes)).
//         Si no es decodificable no se muestra nada y se conserva el espacio.
//
// Geometría: en v2 el `.pagina` mide 21 cm más 2 × 2,54 cm de padding (985 px),
// que no caben en una hoja carta (816 px), así que Chromium encogía TODO el
// documento al 82,15 % (816 / 993 px) al imprimir. Para que el PDF se vea igual
// que el que hoy recibe el negocio, `px()` convierte los píxeles del CSS a
// puntos con esa misma escala (1 px = 0,75 pt × 0,8215). Las posiciones
// verticales se tomaron del PDF que produce Chromium.
//
// Defecto de v2 que no se copia: el logo (1034 × 936) se estiraba a 120 × 100 px;
// aquí se mantiene la caja pero con `fit: "contain"`.

#import sys: inputs
#let d = inputs.at("datos", default: (:))

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

// ---------- Geometría ----------
#let escala = 0.75 * 0.8215
#let px(n) = n * escala * 1pt
#let ancho = px(793.7)           // 21 cm, ancho de .pagina
#let izq = px(104)               // 8 px de margen del body + 2,54 cm de padding
#let verde = rgb("#00b050")      // style="background: #00b050" (pisa el #77c37b del CSS)

#set page(paper: "us-letter", margin: (left: izq, right: 612pt - izq - ancho, top: 0pt, bottom: 0pt))
// Caja de línea igual a la de Chromium con Arial: ascendente 0.905 em, descendente
// 0.212 em y lineGap 0.033 em (line-height "normal" = 1.15 em). Así el borde
// superior de cada bloque de texto y los baselines coinciden con el PDF de v2.
#set text(
  font: ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"),
  size: px(16), top-edge: 0.905em, bottom-edge: -0.212em,
)
#set par(leading: 0.033em, spacing: 0pt, justify: false)
#set block(above: 0pt, below: 0pt)
#let negrita(t) = text(weight: "bold", t)

// ---------- Cabecera: logo absoluto + títulos (#headsuperior) ----------
#block(width: 100%, height: 95.5pt, {
  place(top + left, dx: 3pt, dy: 31.4pt,
    image("assets/logo-sag-gobierno.png", width: px(120), height: px(100), fit: "contain"))
  place(top + left, dx: px(155), dy: 46pt,
    block(width: px(639), align(center, text(size: px(28), weight: "bold")[MINISTERIO DE AGRICULTURA])))
  place(top + left, dx: px(155), dy: 77.1pt,
    block(width: px(639), align(center, text(size: px(15), weight: "bold")[SERVICIO AGRÍCOLA Y GANADERO])))
})

// ---------- Franja verde (#headinferior / #conthi) ----------
#block(width: 100%, height: px(165), fill: verde, {
  set text(fill: white, weight: "bold")
  pad(left: px(100), block(width: px(639), {
    set align(center)
    v(3.9pt)
    block(text(size: px(25), val("carnet_tipo")))
    v(1.2pt)
    block(text(size: px(25))[N° #val("folio")])
    v(15.9pt)
    block(text(size: px(22))[Válido desde #val("vigencia_desde") hasta #val("vigencia_hasta")])
  }))
})

// ---------- Foto (#cont-foto, 180 × 180 px) ----------
#v(24pt)
#let foto = imagen-datos("foto")
#block(width: 100%, height: px(180), align(center,
  if foto != none { image(foto.bytes, format: foto.format, width: px(180), height: px(180), fit: "cover") }
))

// ---------- Datos del titular (#datoUsr) ----------
#v(12.3pt)
#align(center, {
  block(text(size: px(25), weight: "bold", val("nombres")))
  v(0.9pt)
  block(text(size: px(25), weight: "bold")[C.I o pasaporte: #val("n_documento")])
  v(0.3pt)
  block(text(size: px(19))[#negrita[Nacionalidad:] #val("nacionalidad")])
  v(0.5pt)
  block(text(size: px(19))[#negrita[País de residencia:] #val("pais")])
  v(11.7pt)
  block(text(size: px(19))[#negrita[Dirección:] #val("direccion")])
})

// ---------- Firmante (#timbre: 400 px de ancho, centrado y corrido 196 px a la derecha) ----------
#v(111.3pt)
#pad(left: (ancho - px(400)) / 2 + px(196), block(width: px(400), align(center, {
  set text(size: px(20), weight: "bold")
  block(val("firmante_nombre"))
  block(val("firmante_cargo"))
})))

// ---------- Declaración (#disclaimer) ----------
#v(61.8pt)
#pad(x: px(20), par(justify: true, text(size: px(16), style: "italic")[
  El/la cazador/a ha declarado conocer la legislación vigente sobre caza, incluida la Ley Nº 19.473 y su reglamento D.S. Nº 5
]))
