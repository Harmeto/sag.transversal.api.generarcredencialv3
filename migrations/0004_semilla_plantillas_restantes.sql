-- Semilla local: todas las categorías y plantillas restantes de v2, con ruta .typ.
-- Los nombres de categoría definen el contenedor (snake_case); en una migración real
-- se preservan los Id y nombres de la base de v2.
INSERT INTO categoria (nombre)
SELECT n FROM (VALUES ('Alcoholes'), ('Alimentación Animal'), ('Cazadores'), ('Expendios Veterinarios'),
                      ('Mosca Fruta'), ('Plaguicidas'), ('Test')) AS v(n)
WHERE NOT EXISTS (SELECT 1 FROM categoria c WHERE c.nombre = v.n);

INSERT INTO plantilla (nombre, ruta, categoria_id, motor)
SELECT v.nombre, v.ruta, c.id, 'typst'
FROM (VALUES
  ('Certificado Alcoholes',                      'alcoholes/certificado_alcoholes.typ',                                   'Alcoholes'),
  ('Inicio Actividades Establecimiento',        'alimentacion-animal/inicio-actividades-establecimiento.typ',            'Alimentación Animal'),
  ('Carnet Cazadores',                          'cazadores/carnet_cazadores.typ',                                        'Cazadores'),
  ('Bioseguridad Plantel',                      'emergencias-pecuarias/bioseguridad-plantel.typ',                        'Emergencias Pecuarias'),
  ('Registro Mortalidad Aves Silvestres',       'emergencias-pecuarias/registro-mortalidad-aves-silvestres.typ',         'Emergencias Pecuarias'),
  ('Certificado Expendio Veterinario Ninguna',  'expendios-veterinarios/certificado-expendio-veterinario-ninguna.typ',   'Expendios Veterinarios'),
  ('Certificado Expendio Veterinario',          'expendios-veterinarios/certificado-expendio-veterinario.typ',           'Expendios Veterinarios'),
  ('Certificado Expendio Veterinario Ninguno',  'expendios-veterinarios/ninguno/certificado-expendio-veterinario.typ',   'Expendios Veterinarios'),
  ('Certificado Expendio Veterinario No Ninguno','expendios-veterinarios/noninguno/certificado-expendio-veterinario.typ','Expendios Veterinarios'),
  ('Certificado Argentina',                     'mascotas/certificado_argentina.typ',                                    'Mascotas'),
  ('CZE Brasil',                                'mascotas/cze-brasil.typ',                                               'Mascotas'),
  ('Visita Propiedad',                          'mosca-fruta/visita-propiedad.typ',                                      'Mosca Fruta'),
  ('Aplicador Plaguicidas',                     'plaguicidas/aplicador_plaguicidas.typ',                                 'Plaguicidas'),
  ('Certificado Inscripción',                   'plaguicidas/certificado-inscripcion.typ',                               'Plaguicidas'),
  ('Plantilla Test',                            'test/plantilla-test.typ',                                               'Test')
) AS v(nombre, ruta, categoria)
JOIN categoria c ON c.nombre = v.categoria
WHERE NOT EXISTS (SELECT 1 FROM plantilla p WHERE p.ruta = v.ruta);
