#!/usr/bin/env bash
# Levanta la API v3 en local: crea la base en el Postgres de Docker si no existe, aplica
# migraciones (las corre la propia app al arrancar) y ejecuta el binario release.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
set -a; source .env; set +a

DB_NAME="${DATABASE_URL##*/}"
PG_CONTAINER="${PG_CONTAINER:-sag-postgres}"
if ! docker exec "$PG_CONTAINER" psql -U postgres -Atc "select 1 from pg_database where datname='$DB_NAME'" | grep -q 1; then
  echo "Creando base de datos $DB_NAME en el contenedor $PG_CONTAINER"
  docker exec "$PG_CONTAINER" psql -U postgres -c "CREATE DATABASE \"$DB_NAME\""
fi

if [ ! -x target/release/generarcredencialv3 ]; then
  cargo build --release
fi
exec target/release/generarcredencialv3
