//! Prueba de punta a punta del registro de dispositivos por código + clave
//! (ver `docs/features-futuras/propuesta-registro-dispositivos.md`) contra
//! un proyecto real de Supabase -- pensado para staging. Uso:
//!
//! ```text
//! CONTROL_ACCESO_SUPABASE_URL=https://<staging>.supabase.co \
//! CONTROL_ACCESO_SUPABASE_APIKEY=sb_publishable_... \
//! cargo run --example probar_vinculacion --features nube -- <codigo> [<secreto-legado>]
//! ```
//!
//! Con `<codigo>`: vincula un equipo nuevo, vuelve a autenticarse sólo con
//! la clave (aserción firmada), lee datos con ese token y comprueba que el
//! mismo código no se puede canjear dos veces. Con `<secreto-legado>`
//! además: un equipo con secreto migra solo a clave en su primera
//! autenticación y el secreto deja de servir.
//!
//! Cada paso usa su propio directorio temporal como almacén de la clave.

use std::process::ExitCode;
use std::sync::Arc;

use control_acceso::nube::{self, CacheTokenDispositivo, FirmanteArchivo, FirmanteDispositivo};

/// Cuenta las comprobaciones que fallaron, imprimiendo cada una.
#[derive(Default)]
struct Resultados {
    fallas: usize,
}

impl Resultados {
    fn comprobar(&mut self, nombre: &str, resultado: Result<String, String>) {
        match resultado {
            Ok(detalle) => println!("  OK    {nombre}: {detalle}"),
            Err(detalle) => {
                self.fallas += 1;
                println!("  FALLA {nombre}: {detalle}");
            }
        }
    }
}

fn main() -> ExitCode {
    let mut argumentos = std::env::args().skip(1);
    let Some(codigo) = argumentos.next() else {
        eprintln!("uso: probar_vinculacion <codigo> [<secreto-legado>]");
        return ExitCode::FAILURE;
    };
    println!("Proyecto: {}", nube::base_url());

    let mut resultados = Resultados::default();
    probar_codigo(&codigo, &mut resultados);
    if let Some(secreto) = argumentos.next() {
        probar_migracion(&secreto, &mut resultados);
    }

    if resultados.fallas == 0 {
        println!("Todo bien.");
        ExitCode::SUCCESS
    } else {
        println!("{} comprobación(es) fallaron.", resultados.fallas);
        ExitCode::FAILURE
    }
}

/// Firmante en un directorio temporal nuevo, ya configurado en una caché.
fn equipo_nuevo() -> (
    tempfile::TempDir,
    Arc<FirmanteArchivo>,
    CacheTokenDispositivo,
) {
    let directorio = tempfile::tempdir().expect("directorio temporal");
    let firmante = Arc::new(FirmanteArchivo::en(directorio.path()));
    let cache = CacheTokenDispositivo::new();
    cache.establecer_firmante(firmante.clone());
    (directorio, firmante, cache)
}

fn probar_codigo(codigo: &str, resultados: &mut Resultados) {
    println!("Vinculación por código:");
    let (_directorio, firmante, cache) = equipo_nuevo();
    let metadata = nube::MetadatosDispositivo {
        nombre_dispositivo: Some("prueba-e2e".to_string()),
        plataforma: Some("Prueba".to_string()),
        app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        ..Default::default()
    };

    resultados.comprobar(
        "canjear el código",
        cache
            .vincular(codigo, None, Some(&metadata))
            .map(|token| {
                format!(
                    "dispositivo {} del sitio {}",
                    token.dispositivo_id, token.sitio_id
                )
            })
            .map_err(|error| error.to_string()),
    );
    resultados.comprobar(
        "la clave queda marcada como vinculada",
        firmante
            .dispositivo_vinculado()
            .ok_or_else(|| "sin dispositivo_id guardado".to_string()),
    );

    cache.invalidar();
    match cache.autenticar_y_cachear("", None) {
        Ok(token) => {
            resultados.comprobar(
                "autenticarse sólo con la clave",
                Ok(format!("token de {} s", token.expires_in)),
            );
            resultados.comprobar(
                "leer con ese token (RLS)",
                leer_gafetes(&token.access_token),
            );
        }
        Err(error) => {
            resultados.comprobar("autenticarse sólo con la clave", Err(error.to_string()));
        }
    }

    let (_otro_directorio, _, otra_cache) = equipo_nuevo();
    resultados.comprobar(
        "el mismo código no se canjea dos veces",
        match otra_cache.vincular(codigo, None, None) {
            Err(nube::NubeError::CodigoVinculacionInvalido) => Ok("rechazado".to_string()),
            otro => Err(format!(
                "se esperaba CodigoVinculacionInvalido, llegó {otro:?}"
            )),
        },
    );
}

fn probar_migracion(secreto: &str, resultados: &mut Resultados) {
    println!("Migración desde secreto legado:");
    let (_directorio, firmante, cache) = equipo_nuevo();

    resultados.comprobar(
        "autenticarse con el secreto",
        cache
            .autenticar_y_cachear(secreto, None)
            .map(|token| format!("dispositivo {}", token.dispositivo_id))
            .map_err(|error| error.to_string()),
    );
    resultados.comprobar(
        "el equipo quedó migrado a clave",
        firmante
            .dispositivo_vinculado()
            .ok_or_else(|| "no se registró la clave".to_string()),
    );
    cache.invalidar();
    resultados.comprobar(
        "siguiente autenticación, con la clave",
        cache
            .autenticar_y_cachear(secreto, None)
            .map(|_| "ok".to_string())
            .map_err(|error| error.to_string()),
    );
    resultados.comprobar(
        "el secreto ya no sirve solo",
        match nube::autenticar_dispositivo(nube::base_url(), secreto, None) {
            Err(nube::NubeError::CredencialesInvalidas) => Ok("rechazado".to_string()),
            otro => Err(format!("se esperaba CredencialesInvalidas, llegó {otro:?}")),
        },
    );
}

fn leer_gafetes(access_token: &str) -> Result<String, String> {
    let respuesta = reqwest::blocking::Client::new()
        .get(format!(
            "{}/rest/v1/gafetes?select=id&limit=1",
            nube::base_url()
        ))
        .header("apikey", nube::apikey())
        .bearer_auth(access_token)
        .send()
        .map_err(|error| error.to_string())?;
    let estado = respuesta.status();
    if estado.is_success() {
        Ok(format!("HTTP {estado}"))
    } else {
        Err(format!(
            "HTTP {estado}: {}",
            respuesta.text().unwrap_or_default()
        ))
    }
}
