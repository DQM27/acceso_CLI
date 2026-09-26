//! Prueba de INTEGRACIÓN del contrato de autorización que se validó a mano
//! contra `control-acceso-staging` real en la Etapa 2 (ver README.md):
//! un `phx_join` privado con el `access_token` correcto se acepta, uno con
//! el token equivocado (o ninguno) se rechaza. Codifica ese contrato de
//! forma permanente y automatizada -- no depende de que staging esté
//! arriba ni de credenciales reales.

mod comun;

use lattis_realtime_spike::{ClienteRealtime, ErrorCliente};

const TOKEN_CORRECTO: &str = "jwt-de-dispositivo-valido";

#[tokio::test]
async fn un_token_correcto_se_acepta() {
    let url = comun::servidor_con_autorizacion(TOKEN_CORRECTO).await;
    let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
    let resultado = cliente
        .unirse_privado("realtime:sitio:lab", TOKEN_CORRECTO)
        .await;
    assert!(resultado.is_ok(), "{resultado:?}");
}

#[tokio::test]
async fn un_token_incorrecto_se_rechaza() {
    let url = comun::servidor_con_autorizacion(TOKEN_CORRECTO).await;
    let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
    let resultado = cliente
        .unirse_privado("realtime:sitio:lab", "jwt-falsificado-o-expirado")
        .await;
    assert!(
        matches!(resultado, Err(ErrorCliente::JoinRechazado(_))),
        "{resultado:?}"
    );
}

#[tokio::test]
async fn un_token_vacio_se_rechaza() {
    let url = comun::servidor_con_autorizacion(TOKEN_CORRECTO).await;
    let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
    let resultado = cliente.unirse_privado("realtime:sitio:lab", "").await;
    assert!(
        matches!(resultado, Err(ErrorCliente::JoinRechazado(_))),
        "{resultado:?}"
    );
}
