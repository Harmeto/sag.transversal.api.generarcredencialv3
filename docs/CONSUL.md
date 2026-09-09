# Configuración en Consul

Todo lo que cambia entre ambientes vive en Consul. La imagen es una sola: la construye el
pipeline una vez y el mismo tag se despliega en `dev`, `test`, `qa` y `prod`. Lo único que
distingue a un release de otro es la variable `ENVIRONMENT` del Release Pipeline.

Hay **dos niveles** de configuración, con dueños distintos:

| Nivel | Key | Quién la lee | Qué contiene |
|---|---|---|---|
| Despliegue | `{ambiente}/sag.transversal.api.generarcredencialv3/deploy` | El Release Pipeline (`release/v7`) | Proyecto de OpenShift, réplicas, CPU, memoria, rutas, HPA, credenciales del registro de imágenes, y los valores que terminan en el ConfigMap del pod |
| Aplicación | `{ambiente}/sag.transversal.api.generarcredencialv3/config` (y las que ésta referencia) | La propia API al arrancar | Puerto, nivel de log, y punteros a la configuración de base de datos y de storage |

`sag.transversal.api.generarcredencialv3` es el `name` de `package.json`, y debe coincidir
con el nombre del repositorio en minúsculas. Ese nombre arma las keys y también la
etiqueta de la imagen, así que si cambia hay que cambiarlo en los tres lados.

---

## 1. Key de despliegue

Es la del estándar v7, sin nada propio de este servicio. Una por ambiente.

```json
{
  "openshift": {
    "projectName": "credenciales-desa",
    "projectLabel": "worker",
    "appName": "generarcredencialv3"
  },
  "acr": {
    "server": "imagenesmicroservicios.azurecr.io",
    "username": "imagenesmicroservicios",
    "password": "REEMPLAZAR"
  },
  "resources": {
    "replicas": 2,
    "limits":   { "cpu": "1", "memory": "512Mi" },
    "requests": { "cpu": "250m", "memory": "256Mi" }
  },
  "hpa": {
    "minReplicas": 2,
    "maxReplicas": 6,
    "cpuPercent": 70
  },
  "networking": {
    "urlBase": "generarcredencialv3-desa",
    "domain": "apps.openshift.sag.gob.cl",
    "urlSag": "generarcredencialv3-desa.sag.gob.cl"
  },
  "consul": {
    "host": "configms.sag.gob.cl",
    "port": "443",
    "token": "TOKEN_DE_LECTURA_DE_ESTA_APP",
    "env": "dev",
    "secure": "true",
    "rejectUnauthorized": "false",
    "promisify": true
  },
  "logging": {
    "url": "http://registrologdesa.sag.gob.cl",
    "batching": "true",
    "interval": "5000",
    "rejectUnauthorized": false
  },
  "tracing": {
    "endpoint": "https://backstageio.sag.gob.cl/api/traces"
  },
  "application": {
    "port": 3000
  },
  "envVars": {
    "RENDER_CONCURRENCY": "4"
  }
}
```

Notas sobre este servicio en particular:

- **`application.port`** es el puerto que el Deployment expone y al que apunta el Service.
  La API escucha exactamente ahí, porque lee `APP_PORT` del ConfigMap.
- **Memoria**: sin Chromium, el proceso se mueve en el orden de 100 MB. Con 512 MB de
  límite sobra, incluso bajo carga.
- **CPU**: el render es cálculo puro, así que el escalado por CPU del HPA ahora sí refleja
  la carga real. Vale la pena partir con un límite de al menos 1 CPU.
- **`envVars`** es opcional y lo inyecta el release en el ConfigMap. Sirve para ajustar
  `RENDER_CONCURRENCY` (renders simultáneos) por ambiente sin tocar la configuración de la
  aplicación ni reconstruir la imagen. Si no se define, la API usa los núcleos disponibles.

---

## 2. Keys de la aplicación

### 2.1 Configuración principal

Key: `{ambiente}/sag.transversal.api.generarcredencialv3/config`

```json
{
  "port": 3000,
  "host": "0.0.0.0",
  "logLevel": "info",
  "dbConfigValue": "sag.transversal.api.generarcredencialv3/db",
  "blobStorageConfigValue": "sag.transversal.api.generarcredencialv3/blob",
  "renderConcurrency": 4
}
```

- `dbConfigValue` y `blobStorageConfigValue` son **keys relativas al ambiente**: la API lee
  `{ambiente}/{ese valor}`. Es la misma convención que usa v2.
- `logLevel` acepta `error`, `warn`, `info`, `debug` y `trace`.
- `renderConcurrency` es opcional. La variable de entorno `RENDER_CONCURRENCY` manda sobre
  este valor, para poder ajustar desde el ConfigMap sin editar esta key.
- Si falta `blobStorageConfigValue`, la API guarda los documentos en el disco del pod. Eso
  sólo sirve con una réplica, así que lo advierte en el log al arrancar.

### 2.2 Base de datos

Key: `{ambiente}/sag.transversal.api.generarcredencialv3/db`

```json
{
  "server": "postgres-desa.sag.gob.cl",
  "port": 5432,
  "user": "generarcredencial",
  "password": "REEMPLAZAR",
  "database": "generador_credenciales_v3",
  "sslMode": "prefer"
}
```

- Mismos nombres de campo que la configuración de base de datos de v2, para no inventar una
  convención nueva. La diferencia es que apunta a **PostgreSQL**, no a SQL Server.
- `sslMode` es opcional: `disable`, `allow`, `prefer` (por omisión), `require`, `verify-ca`
  o `verify-full`.
- Alternativamente se puede entregar `"url": "postgres://usuario:clave@host:5432/base"`, que
  manda sobre los campos sueltos. Los campos sueltos son preferibles: la contraseña no
  necesita escaparse.
- La base debe existir antes del primer despliegue. Las tablas no: la API aplica sus
  migraciones al arrancar.

### 2.3 Azure Blob Storage

Key: `{ambiente}/sag.transversal.api.generarcredencialv3/blob`

```json
{
  "blobStorageConnectionString": "DefaultEndpointsProtocol=https;AccountName=storagecredenciales;AccountKey=REEMPLAZAR;EndpointSuffix=core.windows.net",
  "blobStorageConnectionStringComentario": "mismos campos que usa v2",
  "blobStorageAccountName": "storagecredenciales"
}
```

- Son los mismos dos valores que usa v2, así que se pueden copiar de la key equivalente.
- La API conserva la convención de nombres de v2: el contenedor es la categoría en
  `snake_case` y el documento es `plantilla_fecha_uuid`. Las URL públicas quedan idénticas,
  que es lo que exige el hecho de que ya estén impresas en códigos QR emitidos.

---

## 3. Cómo cargarlas

Con el CLI de Consul:

```bash
export CONSUL_HTTP_ADDR="configms.sag.gob.cl:443"
export CONSUL_HTTP_TOKEN="TOKEN_DE_INFRAESTRUCTURA"
export CONSUL_HTTP_SSL=true
export CONSUL_HTTP_SSL_VERIFY=false

APP=sag.transversal.api.generarcredencialv3
for AMBIENTE in dev test qa prod; do
  consul kv put "$AMBIENTE/$APP/deploy"  @deploy-$AMBIENTE.json
  consul kv put "$AMBIENTE/$APP/config"  @config-$AMBIENTE.json
  consul kv put "$AMBIENTE/$APP/db"      @db-$AMBIENTE.json
  consul kv put "$AMBIENTE/$APP/blob"    @blob-$AMBIENTE.json
done
```

El repositorio de templates trae un validador para la key de despliegue:

```bash
release/v7/consul/validate-deploy-config.sh deploy-dev.json
```

---

## 4. De dónde sale cada variable en el pod

| Variable | Origen | Quién la usa |
|---|---|---|
| `NODE_ENV` | ConfigMap, desde `ENVIRONMENT` del release | Prefijo de todas las keys de Consul |
| `APP_NAME` | ConfigMap, desde el `name` de `package.json` | Segundo tramo de las keys de Consul |
| `APP_PORT` | ConfigMap, desde `application.port` | Puerto en que escucha la API |
| `CONSUL_HOST`, `CONSUL_PORT`, `CONSUL_SECURE`, `CONSUL_REJECT_UNAUTHORIZED`, `CONSUL_ACL_TOKEN` | ConfigMap, desde el bloque `consul` | Conexión a Consul al arrancar |
| `RENDER_CONCURRENCY` | ConfigMap, desde `envVars` (opcional) | Renders simultáneos |
| Base de datos y storage | Consul, keys `db` y `blob` | Conexión a PostgreSQL y a Azure Blob |

La aplicación no lee ninguna otra variable en los ambientes desplegados. `LOGS_*` y
`TRACER_ENDPOINT` los deja el ConfigMap por el estándar, pero esta versión todavía no los
consume.

---

## 5. Verificación local contra un Consul propio

Sin VPN y sin tocar los ambientes, se puede comprobar que el arranque "modo ambiente"
funciona:

```bash
docker run -d --name consul-v3-test -p 8500:8500 hashicorp/consul:1.20 agent -dev -client=0.0.0.0

APP=sag.transversal.api.generarcredencialv3
curl -X PUT --data '{"port":3000,"logLevel":"info","dbConfigValue":"'"$APP"'/db"}' \
  "http://localhost:8500/v1/kv/dev/$APP/config"
curl -X PUT --data '{"server":"localhost","port":5432,"user":"postgres","password":"postgres","database":"generador_credenciales_v3","sslMode":"disable"}' \
  "http://localhost:8500/v1/kv/dev/$APP/db"

NODE_ENV=dev APP_NAME=$APP APP_PORT=3346 \
CONSUL_HOST=localhost CONSUL_PORT=8500 CONSUL_SECURE=false \
PLANTILLAS_DIR=./plantillas FONTS_DIR=./fonts \
./target/release/generarcredencialv3
```

Cambiando `dev` por `qa` y apuntando a otra base se comprueba que el mismo binario sirve a
dos ambientes sin recompilar, que es exactamente lo que hace el release.
