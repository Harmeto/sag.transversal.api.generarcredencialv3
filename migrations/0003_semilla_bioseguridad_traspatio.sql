-- Semilla local: categoría Emergencias Pecuarias + plantilla Bioseguridad Traspatio.
INSERT INTO categoria (nombre)
SELECT 'Emergencias Pecuarias'
WHERE NOT EXISTS (SELECT 1 FROM categoria WHERE nombre = 'Emergencias Pecuarias');

INSERT INTO plantilla (nombre, ruta, categoria_id, motor)
SELECT 'Bioseguridad Traspatio', 'emergencias-pecuarias/bioseguridad-traspatio.typ', c.id, 'typst'
FROM categoria c
WHERE c.nombre = 'Emergencias Pecuarias'
  AND NOT EXISTS (SELECT 1 FROM plantilla WHERE ruta = 'emergencias-pecuarias/bioseguridad-traspatio.typ');
