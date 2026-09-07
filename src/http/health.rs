use super::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

/// Misma forma que v2 (`status`, `message`, `timestamp`, `uptime`) más el estado del motor.
pub async fn check(State(state): State<AppState>) -> Json<Value> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool).await.is_ok();
    Json(json!({
        "status": if db_ok { "UP" } else { "DEGRADED" },
        "message": if db_ok { "Service is running" } else { "Database unreachable" },
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "uptime": state.inicio.elapsed().as_secs_f64(),
        "render": {
            "motor": state.render.motor(),
            "slotsDisponibles": state.render.disponibles(),
        },
        "storage": state.generador.storage.nombre(),
    }))
}
