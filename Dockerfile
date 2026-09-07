# Etapa 1: compilación
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

# Etapa 2: runtime mínimo (sin Chromium, sin Node). Fuentes: Liberation reemplaza a Arial.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates fonts-liberation fonts-dejavu-core tzdata \
    && rm -rf /var/lib/apt/lists/*
ENV TZ=America/Santiago
WORKDIR /app
COPY --from=build /app/target/release/generarcredencialv3 /app/generarcredencialv3
COPY plantillas ./plantillas
COPY fonts ./fonts
ENV PLANTILLAS_DIR=/app/plantillas FONTS_DIR=/app/fonts STORAGE_DIR=/tmp/storage HOST=0.0.0.0 PORT=3345
# OpenShift (restricted-v2) ejecuta con uid arbitrario: sin root, sin permisos especiales.
RUN mkdir -p /tmp/storage && chmod -R g+rwX /app /tmp/storage
USER 1001
EXPOSE 3345
CMD ["/app/generarcredencialv3"]
