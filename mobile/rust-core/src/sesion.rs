//! Inicio y cierre de sesión (local y contra Supabase Auth). Los usuarios
//! se gestionan sólo desde el panel web.

use control_acceso::services::error::AutenticacionError as AutenticacionErrorNucleo;

use crate::{Nucleo, NucleoError, UsuarioSesion};

/// El celular no cambia contraseñas: sólo el escritorio (en Supabase Auth).
/// Una cuenta con la contraseña temporal del alta todavía sin cambiar no
/// entra acá hasta que la cambie en una computadora.
const MENSAJE_PASSWORD_TEMPORAL: &str = "Su contraseña es temporal: cámbiela primero en una \
     computadora del puesto de seguridad y luego ingrese aquí con la nueva.";

fn rechazo_password_temporal() -> NucleoError {
    NucleoError::Rechazado {
        mensaje: MENSAJE_PASSWORD_TEMPORAL.to_string(),
    }
}

#[uniffi::export]
impl Nucleo {
    /// Login local, con chequeos best-effort contra la nube si el teléfono
    /// está vinculado (sin vincular no se toca la red).
    ///
    /// Dos chequeos contra la nube, uno para cada dirección de un cambio de
    /// estado remoto -- decisión explícita: "por seguridad, pero nunca
    /// bloqueante" (el teléfono tiene que poder operar sin internet), así
    /// que los dos son best-effort (con el tope de `nube::cliente::TIMEOUT_HTTP`):
    ///
    /// 1. **Alta o reactivación**: si el chequeo local dice "inactivo"
    ///    (`AutenticacionErrorNucleo::UsuarioInactivo`) o "no existe"
    ///    (`CredencialesInvalidas` -- que es la misma variante que una
    ///    contraseña incorrecta, ver `autenticacion_service.rs`), puede ser
    ///    que a este usuario lo hayan reactivado en otro dispositivo, o
    ///    creado en el panel/otro sitio DESPUÉS del primer arranque de este
    ///    teléfono, y esta base todavía no se enteró -- antes de rendirse,
    ///    refresca sólo el catálogo (`refrescar_catalogo_sin_sesion`, sin
    ///    sesión) y reintenta el login local una vez más. Sin esto, un
    ///    usuario nuevo o una reactivación remota nunca se podían reflejar
    ///    acá: la sincronización periódica (`SincronizacionPeriodica.kt`)
    ///    recién arranca DESPUÉS de un primer login exitoso, así que una
    ///    cédula que todavía no existe en este teléfono se quedaba
    ///    "credenciales inválidas" para siempre, sin importar cuánto se
    ///    esperara -- reportado en vivo: un ROOT creado en Supabase después
    ///    del primer arranque del emulador nunca podía entrar. Costo
    ///    aceptado: una contraseña tipeada mal también dispara este
    ///    refresco de más (no hay forma barata de distinguir los dos casos
    ///    antes de sincronizar) -- mismo costo que ya paga escritorio, que
    ///    sincroniza el catálogo en CADA intento de login, acierte o no
    ///    (`desktop/src-tauri/src/comandos/autenticacion.rs`).
    /// 2. **Baja**: tras un login local exitoso, confirma en vivo que la
    ///    cédula sigue activa (`usuario_sigue_activo_remoto` -- una fila,
    ///    una columna, no la sincronización completa que hacía esto antes:
    ///    medida como la causa real del retraso de "un par de segundos"
    ///    que se sentía al entrar). La sincronización completa (cola,
    ///    catálogo, historial...) sigue disparándose, pero Kotlin la lanza
    ///    aparte (ver `LoginViewModel.autenticar`) sin que este método la
    ///    espere -- acá retener el candado durante una sincronización
    ///    entera hubiera vuelto a sentirse lento.
    pub fn autenticar(
        &self,
        cedula: String,
        password: String,
    ) -> Result<UsuarioSesion, NucleoError> {
        let intento = self.core_lock().autenticar_con_estado(&cedula, &password);
        let (sesion, debe_cambiar_password) = match intento {
            Ok(resultado) => resultado,
            Err(
                AutenticacionErrorNucleo::UsuarioInactivo
                | AutenticacionErrorNucleo::CredencialesInvalidas,
            ) => {
                if self.nube_configurada() {
                    let _ = self.refrescar_catalogo_sin_sesion();
                }
                self.core_lock().autenticar_con_estado(&cedula, &password)?
            }
            Err(AutenticacionErrorNucleo::SinPasswordLocal) => {
                return self.autenticar_supabase(&cedula, &password);
            }
            Err(otro) => return Err(otro.into()),
        };
        // Contraseña temporal cacheada de un login anterior sin conexión.
        if debe_cambiar_password {
            return Err(rechazo_password_temporal());
        }

        let autorizado_para_nube = self.core_lock().autorizar_uso_nube(&sesion).is_ok();
        let sigue_activo = if autorizado_para_nube && self.nube_configurada() {
            let token = self.autenticar_con_cache().ok();
            token
                .and_then(|token| {
                    let contexto = control_acceso::nube::ContextoSincronizacion {
                        base_url: control_acceso::nube::base_url(),
                        apikey: control_acceso::nube::apikey(),
                        token: &token.access_token,
                        dispositivo_id: &token.dispositivo_id,
                        sitio_id: &token.sitio_id,
                    };
                    control_acceso::nube::usuario_sigue_activo_remoto(&contexto, &sesion.cedula)
                        .ok()
                })
                .unwrap_or(true)
        } else {
            true
        };
        if !sigue_activo {
            return Err(NucleoError::UsuarioInactivo);
        }

        *self.sesion_lock() = Some((sesion.clone(), chrono::Utc::now()));
        Ok(sesion.into())
    }

    /// Sólo olvida el actor en memoria — el `AppCore`/la conexión `SQLite`
    /// se quedan abiertos (son del teléfono, no de la sesión) para que
    /// `Nucleo::autenticar` pueda loguear al siguiente usuario sin
    /// reabrir la base.
    pub fn cerrar_sesion(&self) {
        *self.sesion_lock() = None;
    }

    /// Avisa a la nube que `cedula` salió en este teléfono (sesión única por
    /// unidad y bitácora de sesiones del panel). Hace red: Kotlin la llama en
    /// segundo plano DESPUÉS de `cerrar_sesion`, que es instantánea.
    /// Best-effort: sin red o sin vincular no hace nada; la sesión de la
    /// nube queda hasta que otro ingreso la reemplace.
    pub fn cerrar_sesion_en_la_nube(&self, cedula: String) {
        let resultado = self.autenticar_con_cache().and_then(|token| {
            control_acceso::nube::cerrar_sesion_en_unidad(
                control_acceso::nube::base_url(),
                control_acceso::nube::apikey(),
                &token,
                &cedula,
            )
        });
        if let Err(error) = resultado {
            log::info!("no se pudo cerrar la sesión en la nube: {error}");
        }
    }
}

impl Nucleo {
    /// Login contra Supabase Auth para un usuario global (Administrador/
    /// Operador, o un ROOT ya sincronizado a otro sitio) que todavía no
    /// tiene contraseña local en este teléfono -- ver
    /// docs/planes-implementados/plan-autenticacion-supabase-auth.md y el equivalente en
    /// escritorio (`desktop/src-tauri/src/comandos/autenticacion.rs::login_supabase`).
    /// El refresco del catálogo es best-effort y sólo se intenta si la
    /// identidad todavía no está en el catálogo local (sitio recién
    /// conectado, o el alta acaba de ocurrir) y el teléfono está vinculado. No exportado a `uniffi` (vive en este
    /// `impl Nucleo` plano).
    pub(super) fn autenticar_supabase(
        &self,
        cedula: &str,
        password: &str,
    ) -> Result<UsuarioSesion, NucleoError> {
        let sesion_supabase = control_acceso::nube::login(
            control_acceso::nube::base_url(),
            control_acceso::nube::apikey(),
            cedula,
            password,
        )?;
        // La temporal del alta se cambia en escritorio; ni se abre sesión ni
        // se cachea para entrar sin conexión.
        if sesion_supabase.debe_cambiar_password {
            return Err(rechazo_password_temporal());
        }

        // La identidad (nombre/rol/activo) ya está local -- llegó por el
        // catálogo sincronizado, Supabase Auth sólo confirmó que la
        // contraseña era correcta. Si esta cédula todavía no está en el
        // catálogo local, se intenta refrescar una vez antes de rendirse.
        let intento_identidad = self.core_lock().resolver_identidad_local(cedula);
        let identidad = match intento_identidad {
            Ok(identidad) => identidad,
            Err(
                AutenticacionErrorNucleo::CredencialesInvalidas
                | AutenticacionErrorNucleo::UsuarioInactivo,
            ) => {
                if self.nube_configurada() {
                    let _ = self.refrescar_catalogo_sin_sesion();
                }
                self.core_lock().resolver_identidad_local(cedula)?
            }
            Err(otro) => return Err(otro.into()),
        };

        *self.sesion_lock() = Some((identidad.clone(), chrono::Utc::now()));

        // Best-effort a propósito -- mismo criterio que `login_supabase` en
        // desktop (ver el doc-comment de `AppCore::cachear_password_local`):
        // un fallo acá no debe tumbar un login que ya fue exitoso contra
        // Supabase, sólo deja sin el atajo offline a esta cuenta hasta el
        // próximo login online. Ver `Usuario::password_hash_confirmado_en`
        // y docs/decisiones-tecnicas.md, entrada 2026-09-18.
        //
        // El resultado se liga a una variable ANTES del `if let` a propósito
        // -- `core_lock()` es un `MutexGuard`, y dejarlo como temporal
        // directo en el scrutinee lo mantendría vivo durante todo el bloque
        // (hasta la llave de cierre), no sólo durante la llamada -- riesgo
        // real de deadlock si algo más adelante necesitara el mismo candado.
        // `false`: una temporal ya fue rechazada más arriba.
        let resultado_cache =
            self.core_lock()
                .cachear_password_local(identidad.id, password, false);
        if let Err(error) = resultado_cache {
            log::warn!("no se pudo cachear el login offline: {error}");
        }

        Ok(identidad.into())
    }
}
