//! Benchmark del motor de render, sin HTTP ni base de datos.
//!
//! Uso: bench-render [plantilla] [json] [iteraciones] [hilos]
//!   plantilla   ruta relativa a PLANTILLAS_DIR (default mascotas/datos-cze.typ)
//!   json        archivo con el payload v2 (se usa DatosCredencial) (default scripts/ejemplo-datos-cze.json)
//!   iteraciones renders secuenciales (default 50)
//!   hilos       renders simultáneos en la fase paralela (default núcleos)
//! Escribe el último PDF en tmp/bench-<plantilla>.pdf

use sag_transversal_api_generarcredencialv3::render::typst_renderer::TypstRenderer;
use sag_transversal_api_generarcredencialv3::render::Renderer;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn percentil(v: &[Duration], p: f64) -> Duration {
    let mut s = v.to_vec();
    s.sort();
    s[((s.len() as f64 - 1.0) * p).round() as usize]
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    let plantilla = args.get(1).cloned().unwrap_or_else(|| "mascotas/datos-cze.typ".into());
    let json = args.get(2).cloned().unwrap_or_else(|| "scripts/ejemplo-datos-cze.json".into());
    let iteraciones: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    let hilos: usize = args
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));

    let raiz = PathBuf::from(std::env::var("PLANTILLAS_DIR").unwrap_or_else(|_| "./plantillas".into()));
    let fuentes = std::env::var("FONTS_DIR").ok().map(PathBuf::from);

    let payload: serde_json::Value = serde_json::from_slice(&std::fs::read(&json)?)?;
    let mut datos = payload.get("DatosCredencial").cloned().unwrap_or(payload.clone());
    if let serde_json::Value::Object(m) = &mut datos {
        sag_transversal_api_generarcredencialv3::services::generar_credencial::preparar_datos(m);
    }

    let t = Instant::now();
    let renderer = Arc::new(TypstRenderer::nuevo(&raiz, fuentes.as_deref())?);
    println!("motor typst listo (fuentes + plantillas) en {}", ms(t.elapsed()));

    let t = Instant::now();
    let primero = renderer.renderizar(&plantilla, &datos)?;
    println!("primer render (frío): {} -> {} bytes", ms(t.elapsed()), primero.len());

    let mut tiempos = Vec::with_capacity(iteraciones);
    let mut ultimo = Vec::new();
    for _ in 0..iteraciones {
        let t = Instant::now();
        ultimo = renderer.renderizar(&plantilla, &datos)?;
        tiempos.push(t.elapsed());
    }
    println!(
        "secuencial x{iteraciones}: min {} | p50 {} | p95 {} | max {}",
        ms(*tiempos.iter().min().unwrap()),
        ms(percentil(&tiempos, 0.5)),
        ms(percentil(&tiempos, 0.95)),
        ms(*tiempos.iter().max().unwrap())
    );

    let t = Instant::now();
    let handles: Vec<_> = (0..hilos)
        .map(|_| {
            let r = renderer.clone();
            let p = plantilla.clone();
            let d = datos.clone();
            std::thread::spawn(move || r.renderizar(&p, &d).map(|b| b.len()))
        })
        .collect();
    let mut ok = 0;
    for h in handles {
        if h.join().expect("hilo").is_ok() {
            ok += 1;
        }
    }
    let total = t.elapsed();
    println!(
        "paralelo x{hilos} simultáneos: {ok}/{hilos} ok en {} (≈ {} por documento efectivo)",
        ms(total),
        ms(total / hilos as u32)
    );

    let salida = Path::new("tmp").join(format!("bench-{}.pdf", plantilla.replace('/', "_").replace(".typ", "")));
    std::fs::create_dir_all("tmp")?;
    std::fs::write(&salida, &ultimo)?;
    println!("PDF de muestra: {} ({} bytes)", salida.display(), ultimo.len());
    Ok(())
}
