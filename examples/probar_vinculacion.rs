//! Prueba de punta a punta del registro de dispositivos por código + clave
//! (ver `docs/features-futuras/propuesta-registro-dispositivos.md`) contra
//! un proyecto real de Supabase -- pensado para staging. Uso:
//!
//! ```text
//! CONTROL_ACCESO_SUPABASE_URL=https://<staging>.supabase.co \
//! CONTROL_ACCESO_SUPABASE_APIKEY=sb_publishable_... \
//! cargo run --example probar_vinculacion --features nube -- <codigo> [<codigo-revinculacion>]
//! ```
//!
//! Con `<codigo>`: vincula un equipo nuevo, vuelve a autenticarse sólo con
//! la clave (aserción firmada), lee datos con ese token y comprueba que el
//! mismo código no se puede canjear dos veces. Con `<codigo-revinculacion>`
//! (un código nuevo del MISMO dispositivo, "Re-vincular" en el panel)
//! además: otro equipo toma ese dispositivo con su propia clave y la clave
//! anterior deja de servir, igual que su token ya emitido.
//!
//! Cada equipo usa su propio directorio temporal como almacén de la clave.

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
        eprintln!("uso: probar_vinculacion <codigo> [<codigo-revinculacion>]");
        return ExitCode::FAILURE;
    };
    println!("Proyecto: {}", nube::base_url());

    let mut resultados = Resultados::default();
    let equipo = probar_codigo(&codigo, &mut resultados);
    if let (Some(codigo_revinculacion), Some(equipo)) = (argumentos.next(), equipo) {
        probar_revinculacion(&codigo_revinculacion, &equipo, &mut resultados);
    }

    if resultados.fallas == 0 {
        println!("Todo bien.");
        ExitCode::SUCCESS
    } else {
        println!("{} comprobación(es) fallaron.", resultados.fallas);
        ExitCode::FAILURE
    }
}

/// Un equipo de prueba: su clave vive en un directorio temporal propio,
/// que se borra al soltarlo.
struct Equipo {
    _directorio: tempfile::TempDir,
    firmante: Arc<FirmanteArchivo>,
    cache: CacheTokenDispositivo,
}

fn equipo_nuevo() -> Equipo {
    let directorio = tempfile::tempdir().expect("directorio temporal");
    let firmante = Arc::new(FirmanteArchivo::en(directorio.path()));
    let cache = CacheTokenDispositivo::new();
    cache.establecer_firmante(firmante.clone());
    Equipo {
        _directorio: directorio,
        firmante,
        cache,
    }
}

/// Devuelve el equipo vinculado, para seguir probando con él.
fn probar_codigo(codigo: &str, resultados: &mut Resultados) -> Option<Equipo> {
    println!("Vinculación por código:");
    let equipo = equipo_nuevo();
    let Equipo {
        firmante, cache, ..
    } = &equipo;
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
    match cache.autenticar_y_cachear(None) {
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

    resultados.comprobar(
        "el mismo código no se canjea dos veces",
        match equipo_nuevo().cache.vincular(codigo, None, None) {
            Err(nube::NubeError::CodigoVinculacionInvalido) => Ok("rechazado".to_string()),
            otro => Err(format!(
                "se esperaba CodigoVinculacionInvalido, llegó {otro:?}"
            )),
        },
    );

    firmante.dispositivo_vinculado().is_some().then_some(equipo)
}

/// "Re-vincular" desde el panel: un equipo nuevo (clave nueva) toma el
/// mismo dispositivo y el anterior queda afuera.
fn probar_revinculacion(codigo: &str, anterior: &Equipo, resultados: &mut Resultados) {
    println!("Re-vinculación del mismo dispositivo:");
    let Some(dispositivo_id) = anterior.firmante.dispositivo_vinculado() else {
        return;
    };
    // Cuántas filas veía el equipo anterior antes de perder el dispositivo:
    // con 0 (sitio sin gafetes) la última comprobación no probaría nada.
    let token_anterior = anterior.cache.autenticar_con_cache().ok();
    let filas_antes = token_anterior
        .as_ref()
        .and_then(|token| leer_gafetes_contando(&token.access_token).ok())
        .unwrap_or(0);

    let nuevo = equipo_nuevo();
    resultados.comprobar(
        "canjear el código nuevo con otra clave",
        nuevo
            .cache
            .vincular(codigo, Some(&dispositivo_id), None)
            .map(|token| {
                if token.dispositivo_id == dispositivo_id {
                    "mismo dispositivo".to_string()
                } else {
                    format!("OJO: quedó como {}", token.dispositivo_id)
                }
            })
            .map_err(|error| error.to_string()),
    );

    anterior.cache.invalidar();
    resultados.comprobar(
        "la clave anterior ya no autentica",
        match anterior.cache.autenticar_y_cachear(None) {
            Err(nube::NubeError::CredencialesInvalidas) => Ok("rechazada".to_string()),
            otro => Err(format!("se esperaba CredencialesInvalidas, llegó {otro:?}")),
        },
    );
    match token_anterior {
        Some(token) if filas_antes > 0 => resultados.comprobar(
            "el token ya emitido a la clave anterior no lee nada",
            match leer_gafetes_contando(&token.access_token) {
                Ok(0) => Ok("0 filas (política restrictiva)".to_string()),
                Ok(filas) => Err(format!("todavía lee {filas} fila(s)")),
                Err(error) => Ok(format!("rechazado: {error}")),
            },
        ),
        _ => println!("  --    sitio sin gafetes: no se puede comprobar el token anterior"),
    }
}

fn leer_gafetes(access_token: &str) -> Result<String, String> {
    leer_gafetes_contando(access_token).map(|filas| format!("{filas} fila(s)"))
}

/// Cuántas filas de `gafetes` devuelve `PostgREST` con ese token (hasta 1).
fn leer_gafetes_contando(access_token: &str) -> Result<usize, String> {
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
        respuesta
            .json::<Vec<serde_json::Value>>()
            .map(|filas| filas.len())
            .map_err(|error| error.to_string())
    } else {
        Err(format!(
            "HTTP {estado}: {}",
            respuesta.text().unwrap_or_default()
        ))
    }
}
