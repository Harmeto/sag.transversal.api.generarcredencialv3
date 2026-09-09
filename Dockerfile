# ============================================================================
# Imagen de sag.transversal.api.generarcredencialv3
#
# Una sola imagen para todos los ambientes: no se hornea configuración. El
# ambiente lo determina NODE_ENV (lo pone el ConfigMap del release) y la
# configuración se lee de Consul al arrancar.
#
# La compilación y las pruebas ocurren acá dentro, así que el agente de Azure
# DevOps no necesita Rust: sólo docker build.
# ============================================================================

# ---------- Etapa 1: dependencias (capa cacheable) --------------------------
FROM rust:1.98-bookworm AS deps
WORKDIR /build
# Sólo los manifiestos: mientras no cambien, esta capa se reutiliza y no se
# vuelven a compilar las dependencias (que son la mayor parte del tiempo).
COPY Cargo.toml Cargo.lock ./
# Un stub por cada binario declarado en Cargo.toml: si falta alguno, cargo falla acá
# (es justo lo que pasa al agregar un binario nuevo y olvidar esta línea).
RUN mkdir -p src/bin \
    && echo 'fn main() {}' > src/main.rs \
    && echo 'fn main() {}' > src/bin/bench_render.rs \
    && echo 'fn main() {}' > src/bin/migrar_datos.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

# ---------- Etapa 2: compilación y pruebas ----------------------------------
FROM deps AS build
ARG RUN_TESTS=true
COPY src ./src
COPY assets ./assets
COPY migrations ./migrations
COPY plantillas ./plantillas
COPY fonts ./fonts
COPY scripts ./scripts
COPY package.json ./package.json
# `touch` fuerza a recompilar el código propio sobre las dependencias cacheadas.
RUN touch src/main.rs src/lib.rs \
    && if [ "$RUN_TESTS" = "true" ]; then cargo test --release --locked; fi \
    && cargo build --release --locked --bin generarcredencialv3 \
    && strip target/release/generarcredencialv3

# ---------- Etapa 3: runtime ------------------------------------------------
# Sin Chromium, sin Node: sólo el binario, las plantillas y las fuentes.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        fonts-liberation \
        fonts-dejavu-core \
        tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=America/Santiago
WORKDIR /app

# OpenShift (SCC restricted-v2) ejecuta con un UID arbitrario que siempre pertenece al
# grupo 0, así que todo se copia ya con grupo root. Se hace en el propio COPY y no con
# un `chmod -R` posterior: cambiar permisos en una capa nueva obliga a Docker a
# duplicar los archivos, y con un binario de 50 MB eso costaba 56 MB de imagen.
COPY --from=build --chown=1001:0 /build/target/release/generarcredencialv3 /app/generarcredencialv3
COPY --chown=1001:0 plantillas /app/plantillas
COPY --chown=1001:0 fonts /app/fonts

# Valores por defecto; el ConfigMap del release puede sobrescribirlos.
ENV PLANTILLAS_DIR=/app/plantillas \
    FONTS_DIR=/app/fonts \
    STORAGE_DIR=/tmp/storage \
    HOST=0.0.0.0 \
    APP_PORT=3000

# Único directorio que la aplicación escribe (sólo con el storage local; con Azure
# Blob no se usa). Se crea con escritura para el grupo 0.
RUN mkdir -p /tmp/storage && chgrp 0 /tmp/storage && chmod 775 /tmp/storage

USER 1001
EXPOSE 3000
CMD ["/app/generarcredencialv3"]
