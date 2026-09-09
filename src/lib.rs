//! sag.transversal.api.generarcredencialV3
//!
//! Generación de documentos PDF a partir de plantillas Typst. Misma superficie de
//! API que v2 (rutas, payloads y respuestas), sin Chromium y sobre PostgreSQL.

pub mod config;
pub mod consul;
pub mod error;
pub mod http;
pub mod render;
pub mod repos;
pub mod services;
pub mod storage;
