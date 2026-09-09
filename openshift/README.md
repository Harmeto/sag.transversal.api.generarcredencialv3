# Despliegue directo en OpenShift

Este directorio permite levantar una demostración completa de Credenciales v3
sin Azure DevOps ni Consul. PostgreSQL queda accesible solo dentro del proyecto
OpenShift; la única ruta pública es la API.

## 1. Construir y publicar la imagen

Inicia sesión en el clúster desde la consola de OpenShift y publica la imagen en
un registro al que el clúster pueda acceder. Por ejemplo, usando Quay:

```bash
export IMAGE="quay.io/TU_USUARIO/generarcredencialv3:3.0.0"
podman build -t "$IMAGE" .
podman push "$IMAGE"
```

## 2. Crear secretos

Los archivos `*.example.yaml` no se aplican directamente ni se versionan con
claves reales. Cópialos fuera del repositorio, reemplaza la contraseña y aplica
ambos secretos. La clave de `DATABASE_URL` debe ser idéntica a la de PostgreSQL.

```bash
cp openshift/postgresql-secret.example.yaml /tmp/postgresql-secret.yaml
cp openshift/api-secret.example.yaml /tmp/api-secret.yaml
# Edita ambos archivos y reemplaza REEMPLAZAR_POR_UNA_CLAVE_LARGA.
oc apply -f /tmp/postgresql-secret.yaml
oc apply -f /tmp/api-secret.yaml
```

## 3. Desplegar PostgreSQL y la API

```bash
oc apply -f openshift/postgresql-pvc.yaml
oc apply -f openshift/postgresql-service.yaml
oc apply -f openshift/postgresql-deployment.yaml
oc rollout status deployment/postgresql

oc apply -f openshift/api-pvc.yaml
oc apply -f openshift/api-configmap.yaml
oc apply -f openshift/api-service.yaml
oc apply -f openshift/api-route.yaml
oc apply -f openshift/api-deployment.yaml
oc set image deployment/generarcredencialv3 api="$IMAGE"
oc rollout status deployment/generarcredencialv3
```

La API ejecuta sus migraciones y semillas automáticamente al iniciar. Verifica:

```bash
ROUTE_HOST=$(oc get route generarcredencialv3 -o jsonpath='{.spec.host}')
curl "https://${ROUTE_HOST}/api/v3/transversal/credencial/health"
```

## 4. Corregir la URL pública de documentos

Cuando la Route exista, actualiza `STORAGE_PUBLIC_URL` y reinicia el despliegue:

```bash
oc create configmap generarcredencialv3-api \
  --from-literal=APP_NAME=sag.transversal.api.generarcredencialv3 \
  --from-literal=HOST=0.0.0.0 \
  --from-literal=PORT=3345 \
  --from-literal=LOG_LEVEL=info \
  --from-literal=PLANTILLAS_DIR=/app/plantillas \
  --from-literal=FONTS_DIR=/app/fonts \
  --from-literal=STORAGE_DIR=/data/storage \
  --from-literal="STORAGE_PUBLIC_URL=https://${ROUTE_HOST}/archivos" \
  --dry-run=client -o yaml | oc apply -f -
oc rollout restart deployment/generarcredencialv3
```

Los PVC conservan la base de datos y los PDFs mientras el proyecto OpenShift no
sea eliminado. Esta configuración es para demostración; antes de producción se
deben definir respaldo, recuperación, capacidad y una base PostgreSQL gestionada.
