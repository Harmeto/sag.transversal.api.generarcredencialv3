// Plantilla "Certificado de Inscripción – Registro Único Nacional de Fertilizantes y
// Bioestimulantes" migrada desde v2 (plaguicidas/certificado-inscripcion.html) a Typst.
//
// Recibe en `sys.inputs.datos` el mismo `DatosCredencial` que v2:
//   nombre  (empresa / persona inscrita), region, mesano ("septiembre de 2026").
//
// Tipografía: v2 declaraba @font-face para gobCL (Bold/Heavy/Light/Regular) pero
// los `.woff` con ruta relativa nunca cargaban, y todos los usos quedaron
// comentados a favor de Arial. En v3 gobCL sí carga desde `fonts/`, así que el
// certificado usa gobCL (la intención del diseño institucional); `usar-gobcl = false`
// reproduce el Arial de v2. Tamaños y posiciones se tomaron del PDF de Chromium.
//
// Particularidades de v2 que se conservan: la línea "{region}, {mesano}" está
// posicionada en absoluto al 40 % del ancho y 90 % del alto de la hoja (fuera
// del recuadro, alineada a la izquierda, no centrada).

#import sys: inputs
#let d = inputs.at("datos", default: (:))
#let usar-gobcl = true

// ---------- Utilidades ----------
#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))

// ---------- Geometría, colores y tipografía ----------
#let px(n) = n * 0.75pt
#let margen = px(40)                       // body { margin: 0 40px }
#let ancho = 612pt - 2 * margen
#let azul = rgb("#004f9d")                  // .titulo rgba(0, 79, 157)
#let gris = rgb("#808080")                  // .titulosag gray
#let gris-texto = rgb("#5a5a5a")            // .glosa / .futer rgba(90, 90, 90)
#let celeste = rgb("#c5e2f6")               // #nombre
#let arial = ("Arial", "Liberation Sans", "Helvetica", "DejaVu Sans")
#let fuente = if usar-gobcl { ("gobCL", ..arial) } else { arial }

#set page(paper: "us-letter", margin: 0pt)
// Caja de línea igual a la de Chromium con Arial (ascendente 0.905 em, descendente
// 0.212 em, lineGap 0.033 em → line-height 1.15 em) aunque la fuente sea gobCL: así
// los baselines quedan donde los dejaba v2 y sólo cambia el dibujo de las letras.
#set text(font: fuente, size: px(20), fill: gris-texto, top-edge: 0.905em, bottom-edge: -0.212em)
#set par(leading: 0.033em, spacing: 0pt, justify: false)
#set block(above: 0pt, below: 0pt)

// Región y fecha: <span style="position:absolute; left:40%; top:90%"> (coordenadas de hoja)
#place(top + left, dx: 40% * 612pt, dy: 90% * 792pt,
  block(width: 612pt - 40% * 612pt - margen, text(size: px(15))[#val("region"), #val("mesano")]))

// Recuadro con borde y marca de agua "SAG" estirada al tamaño del recuadro (.bg-imagen)
#pad(x: margen, block(
  width: ancho, stroke: 0.75pt + black, inset: 0pt,
  fill: tiling(size: (ancho, 664pt), image("assets/fondo-marca-agua-sag.png", width: ancho, height: 664pt, fit: "stretch")),
  {
    set align(center)
    // viñeta azul/roja (.vineta, 80 × 20 px, arriba a la izquierda)
    align(left, image("assets/vineta-azul-roja.png", width: px(80), height: px(20), fit: "stretch"))
    v(17.9pt)
    block(text(fill: gris)[Servicio agrícola y ganadero])
    v(28.6pt)
    // h2 dentro de .titulo (30 px × 1.5 = 45 px)
    block(text(size: px(45), weight: "bold", fill: azul)[
      Certificado de Inscripción \
      Registro Único Nacional de \
      Fertilizantes y Bioestimulantes
    ])
    v(91.1pt)
    block[De acuerdo a lo que establece la #text(weight: "bold")[Ley Nº21.349,]]
    v(15.5pt)
    block[
      El Ministerio de Agricultura, \
      a través del Servicio Agrícola y Ganadero, \
      certifica que la empresa/persona natural o jurídica
    ]
    v(15.6pt)
    // #nombre (h2 de 30 px, fondo celeste, 80 px de margen lateral, 10 px de padding)
    pad(x: px(80), block(width: 100%, fill: celeste, inset: px(10),
      text(size: px(30), weight: "bold", val("nombre"))))
    v(15pt)
    block[
      cumple con la inscripción en el \
      Registro Único Nacional de Fertilizantes y Bioestimulantes
    ]
    v(113pt)
    // .futer: logo SAG/Gobierno a la izquierda y logo SAG mini a la derecha (float)
    block(width: 100%, height: px(100), {
      place(top + left, image("assets/logo-sag-gobierno-cuadrado.png", width: px(100), height: px(100), fit: "contain"))
      place(top + right, image("assets/logo-sag-mini.png", width: px(100), height: px(50), fit: "contain"))
    })
    v(2.3pt)
  },
))
