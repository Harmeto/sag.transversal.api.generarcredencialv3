-- Esquema v3 (PostgreSQL). Equivale a las tablas Categoria / Plantilla / Credencial de v2 (MSSQL),
-- con identificadores en snake_case y tipos nativos (uuid, timestamptz).

CREATE TABLE IF NOT EXISTS categoria (
    id      SERIAL PRIMARY KEY,
    nombre  VARCHAR(150) NOT NULL
);

CREATE TABLE IF NOT EXISTS plantilla (
    id            SERIAL PRIMARY KEY,
    nombre        VARCHAR(150) NOT NULL,
    ruta          VARCHAR(255) NOT NULL,
    categoria_id  INTEGER NOT NULL REFERENCES categoria(id),
    -- Motor que renderiza la plantilla. v3 nace con 'typst'; 'chromium' queda reservado
    -- para una migración plantilla a plantilla desde v2.
    motor         VARCHAR(20) NOT NULL DEFAULT 'typst'
);

CREATE INDEX IF NOT EXISTS plantilla_categoria_id_idx ON plantilla(categoria_id);

CREATE TABLE IF NOT EXISTS credencial (
    id              UUID PRIMARY KEY,
    categoria_id    INTEGER NOT NULL REFERENCES categoria(id),
    url             TEXT NOT NULL,
    fecha_creacion  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS credencial_categoria_id_idx ON credencial(categoria_id);
CREATE INDEX IF NOT EXISTS credencial_fecha_creacion_idx ON credencial(fecha_creacion);
