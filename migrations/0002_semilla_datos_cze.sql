-- Semilla local para la prueba de concepto: categoría Mascotas + plantilla Datos CZE.
-- En una migración real desde v2 se preservarían los Id originales de Categoria y Plantilla.
INSERT INTO categoria (nombre)
SELECT 'Mascotas'
WHERE NOT EXISTS (SELECT 1 FROM categoria WHERE nombre = 'Mascotas');

INSERT INTO plantilla (nombre, ruta, categoria_id, motor)
SELECT 'Datos CZE', 'mascotas/datos-cze.typ', c.id, 'typst'
FROM categoria c
WHERE c.nombre = 'Mascotas'
  AND NOT EXISTS (SELECT 1 FROM plantilla WHERE ruta = 'mascotas/datos-cze.typ');
