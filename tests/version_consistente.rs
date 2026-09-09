//! El pipeline del SAG (release/v7) toma el nombre y la versión desde `package.json`:
//! con ellos etiqueta la imagen y arma la key de Consul. Cargo.toml, en cambio, es la
//! versión "real" del binario. Este test evita que ambas se separen sin que nadie lo note.

#[test]
fn package_json_y_cargo_toml_declaran_lo_mismo() {
    let pkg: serde_json::Value =
        serde_json::from_str(include_str!("../package.json")).expect("package.json inválido");

    let version_pkg = pkg["version"].as_str().expect("package.json sin 'version'");
    assert_eq!(
        version_pkg,
        env!("CARGO_PKG_VERSION"),
        "la versión de package.json (la que etiqueta la imagen) no coincide con la de Cargo.toml"
    );

    let nombre = pkg["name"].as_str().expect("package.json sin 'name'");
    assert_eq!(
        nombre,
        nombre.to_lowercase(),
        "el nombre debe ir en minúsculas: el build publica la imagen bajo el nombre del \
         repositorio en minúsculas y el release la busca bajo este nombre"
    );
    assert!(
        nombre.starts_with("sag."),
        "el nombre debe ser el del repositorio, porque también es el prefijo de las keys de Consul"
    );
}
