//! Configuración desde variables de entorno (local: `.env`; OpenShift: ConfigMap/Consul).

use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub app_name: String,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub plantillas_dir: PathBuf,
    pub fonts_dir: Option<PathBuf>,
    pub render_concurrency: usize,
    pub storage_dir: PathBuf,
    pub storage_public_url: String,
}

fn var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("falta la variable de entorno {name}"))
}

fn var_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn desde_entorno() -> Result<Self> {
        let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let render_concurrency = std::env::var("RENDER_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(cpus);

        Ok(Self {
            app_name: var_or("APP_NAME", "sag.transversal.api.generarcredencialV3"),
            host: var_or("HOST", "0.0.0.0"),
            port: var_or("PORT", "3345").parse().context("PORT inválido")?,
            database_url: var("DATABASE_URL")?,
            plantillas_dir: PathBuf::from(var_or("PLANTILLAS_DIR", "./plantillas")),
            fonts_dir: std::env::var("FONTS_DIR").ok().map(PathBuf::from),
            render_concurrency,
            storage_dir: PathBuf::from(var_or("STORAGE_DIR", "./storage")),
            storage_public_url: var_or("STORAGE_PUBLIC_URL", "http://localhost:3345/archivos")
                .trim_end_matches('/')
                .to_string(),
        })
    }
}
