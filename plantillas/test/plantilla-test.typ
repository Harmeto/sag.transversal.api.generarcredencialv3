// Plantilla de prueba migrada desde v2 (test/plantilla-test.html):
//   <html><body><h1>{nombre}</h1><h2>{apellido}</h2></body></html>
// Sin CSS: Chromium la imprimía en carta sin márgenes de página (sólo los 8px del
// <body>) y con los estilos de agente de usuario de <h1> (2em, negrita, margen 0.67em)
// y <h2> (1.5em, negrita, margen 0.83em) sobre Times 16px. Variables: nombre, apellido.

#import sys: inputs
#let d = inputs.at("datos", default: (:))
#let como-texto(x) = {
  if x == none { "" }
  else if type(x) == str { x }
  else if type(x) == bool { if x { "Sí" } else { "No" } }
  else if type(x) == int or type(x) == float { str(x) }
  else { repr(x) }
}
#let val(k) = como-texto(d.at(k, default: ""))

// margen del <body> (8px); el margen superior del <h1> (0.67em = 21px) colapsa con él
#set page(paper: "us-letter", margin: 6pt)
#set text(font: ("Times New Roman", "Liberation Serif", "Times", "DejaVu Serif"), size: 12pt)
#set par(leading: 0.35em, spacing: 0pt)

#v(10pt)
#block(below: 16pt, text(size: 24pt, weight: "bold", val("nombre")))
#block(above: 18pt, text(size: 18pt, weight: "bold", val("apellido")))
