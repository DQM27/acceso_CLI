//! Inicio y cierre de sesión (local y contra Supabase Auth) y gestión de usuarios.

use control_acceso::database::queries::usuarios::FiltroUsuarios as FiltroUsuariosNucleo;
use control_acceso::services::error::AutenticacionError as AutenticacionErrorNucleo;
use control_acceso::services::usuario_service::CrearUsuarioInput as CrearUsuarioInputNucleo;

use crate::{
    DatosUsuario, Nucleo, NucleoError, ResultadoLogin, SesionSupabaseCacheada,
    TOPE_PRESENCIA_SUPABASE, UsuarioResumen,
};

#[uniffi::export]
impl Nucleo {
    /// Login con el secreto que Kotlin descifra de Android Keystore (vacío =
    /// nube sin configurar: no se toca la red).
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
    ///    refresca sólo el catálogo (`refrescar_catalogo_sin_sesion_con_secreto`, sin
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
    pub fn autenticar_con_secreto(
        &self,
        cedula: String,
        password: String,
        secreto: String,
    ) -> Result<ResultadoLogin, NucleoError> {
        let intento = self.core_lock().autenticar_con_estado(&cedula, &password);
        let (sesion, debe_cambiar_password) = match intento {
            Ok(resultado) => resultado,
            Err(
                AutenticacionErrorNucleo::UsuarioInactivo
                | AutenticacionErrorNucleo::CredencialesInvalidas,
            ) => {
                if !secreto.trim().is_empty() {
                    let _ = self.refrescar_catalogo_sin_sesion_con_secreto(&secreto);
                }
                self.core_lock().autenticar_con_estado(&cedula, &password)?
            }
            Err(AutenticacionErrorNucleo::SinPasswordLocal) => {
                return self.autenticar_supabase(&cedula, &password, &secreto);
            }
            Err(otro) => return Err(otro.into()),
        };

        let autorizado_para_nube = self.core_lock().autorizar_uso_nube(&sesion).is_ok();
        let sigue_activo = if autorizado_para_nube && !secreto.trim().is_empty() {
            let token = self.autenticar_con_cache(&secreto).ok();
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

        *self.sesion_lock() = Some(sesion.clone());
        Ok(ResultadoLogin {
            sesion: sesion.into(),
            debe_cambiar_password,
        })
    }

    /// Cambio de contraseña obligatorio (`debe_cambiar_password` en `true`
    /// tras `autenticar_con_secreto`) o rutinario --
    /// `nube::cambiar_password` ya revalida `password_actual` con un login
    /// real antes de aceptar la nueva, no confía en que la sesión siga
    /// abierta.
    pub fn cambiar_password_supabase(
        &self,
        password_actual: String,
        password_nueva: String,
    ) -> Result<(), NucleoError> {
        let sesion = self.actor_autenticado()?;
        let access_token = self
            .access_token_supabase_vigente()
            .ok_or(NucleoError::SesionSupabaseVencida)?;

        control_acceso::nube::cambiar_password(
            control_acceso::nube::base_url(),
            control_acceso::nube::apikey(),
            &access_token,
            &sesion.cedula,
            &password_actual,
            &password_nueva,
        )?;

        // Best-effort, mismo criterio que en `autenticar_supabase` -- ver
        // el doc-comment de `AppCore::cachear_password_local`. Refresca el
        // caché con la contraseña NUEVA (y `debe_cambiar_password: false`,
        // ya que el cambio se acaba de confirmar contra Supabase Auth):
        // sin esto (hallazgo de auditoría 2026-09-24, MV-02), el caché
        // seguía teniendo la contraseña VIEJA hasta el próximo login
        // online -- la nueva contraseña quedaba rechazada offline por 24h
        // mientras la vieja (o la temporal, si el cambio era obligatorio)
        // seguía sirviendo para entrar sin conexión.
        //
        // El resultado se liga a una variable ANTES del `if let` a
        // propósito -- mismo motivo que en `autenticar_supabase` un poco
        // más arriba: `core_lock()` es un `MutexGuard`, y dejarlo como
        // temporal directo en el scrutinee lo mantendría vivo durante todo
        // el bloque, no sólo durante la llamada.
        let resultado_cache =
            self.core_lock()
                .cachear_password_local(sesion.id, &password_nueva, false);
        if let Err(error) = resultado_cache {
            log::warn!("no se pudo refrescar el cacheo de login offline: {error}");
        }

        Ok(())
    }

    /// Sólo olvida el actor en memoria — el `AppCore`/la conexión `SQLite`
    /// se quedan abiertos (son del teléfono, no de la sesión) para que
    /// `Nucleo::autenticar` pueda loguear al siguiente usuario sin
    /// reabrir la base.
    pub fn cerrar_sesion(&self) {
        *self.sesion_lock() = None;
        *self.lock_sesion_supabase() = None;
    }

    /// Sólo Root/Administrador — ver el doc-comment de `UsuarioResumen`.
    pub fn listar_usuarios(&self, texto: String) -> Result<Vec<UsuarioResumen>, NucleoError> {
        let actor = self.actor_autenticado()?;
        let core = self.core_lock();
        let texto_normalizado = texto.trim();
        let filtro = FiltroUsuariosNucleo {
            texto: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            ..Default::default()
        };
        Ok(core
            .buscar_usuarios(&actor, &filtro)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Sólo Root/Administrador — Rust ya rechaza a un actor sin
    /// `Operacion::GestionarUsuarios` con `OperacionNoAutorizada`
    /// (`verificar_creacion_usuario`), y sólo Root puede crear otro Root
    /// (`puede_gestionar_usuario`). Kotlin oculta el menú para Operador
    /// como atajo de UX, no como el control real.
    pub fn crear_usuario(&self, datos: DatosUsuario) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().crear_usuario(
            &actor,
            CrearUsuarioInputNucleo {
                cedula: datos.cedula,
                nombre: datos.nombre,
                password: datos.password,
                rol: datos.rol.into(),
                activo: datos.activo,
            },
        )?)
    }
}

impl Nucleo {
    /// Guarda (o reemplaza) la sesión de Supabase Auth -- se llama tanto en
    /// el login inicial (`autenticar_supabase`) como en cada renovación
    /// exitosa en segundo plano (`sincronizar_con_secreto`),
    /// siempre con una marca de tiempo nueva.
    pub(super) fn iniciar_sesion_supabase(&self, sesion: control_acceso::nube::SesionSupabase) {
        *self.lock_sesion_supabase() = Some(SesionSupabaseCacheada {
            access_token: sesion.access_token,
            refresh_token: sesion.refresh_token,
            expires_in: sesion.expires_in,
            confirmada_en: std::time::Instant::now(),
        });
    }

    /// Token de acceso vigente para usar como `Authorization: Bearer`, si lo
    /// hay -- `None` si nunca hubo sesión de Supabase, si el token técnico
    /// ya venció, o si pasó `TOPE_PRESENCIA_SUPABASE` desde la última
    /// confirmación real (aunque el token en sí siga sin vencer).
    pub(super) fn access_token_supabase_vigente(&self) -> Option<String> {
        let guard = self.lock_sesion_supabase();
        let entrada = guard.as_ref()?;
        let vigente_por = std::time::Duration::from_secs(entrada.expires_in);
        let vencido = entrada.confirmada_en.elapsed() >= vigente_por
            || entrada.confirmada_en.elapsed() >= TOPE_PRESENCIA_SUPABASE;
        let token = entrada.access_token.clone();
        drop(guard);
        if vencido { None } else { Some(token) }
    }

    /// `refresh_token` actual, para la renovación en segundo plano -- `None`
    /// si nunca hubo sesión de Supabase (usuario logueado localmente) o si
    /// ya se cerró sesión.
    pub(super) fn refresh_token_supabase(&self) -> Option<String> {
        self.lock_sesion_supabase()
            .as_ref()
            .map(|entrada| entrada.refresh_token.clone())
    }

    pub(super) fn lock_sesion_supabase(
        &self,
    ) -> std::sync::MutexGuard<'_, Option<SesionSupabaseCacheada>> {
        self.sesion_supabase
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Login contra Supabase Auth para un usuario global (Administrador/
    /// Operador, o un ROOT ya sincronizado a otro sitio) que todavía no
    /// tiene contraseña local en este teléfono -- ver
    /// docs/planes-implementados/plan-autenticacion-supabase-auth.md y el equivalente en
    /// escritorio (`desktop/src-tauri/src/comandos/autenticacion.rs::login_supabase`).
    /// El refresco del catálogo con `secreto` es best-effort y sólo se
    /// intenta si la identidad todavía no está en el catálogo local (sitio
    /// recién conectado, o el alta acaba de ocurrir); secreto vacío = sin
    /// nube, no se intenta. No exportado a `uniffi` (vive en este
    /// `impl Nucleo` plano).
    pub(super) fn autenticar_supabase(
        &self,
        cedula: &str,
        password: &str,
        secreto: &str,
    ) -> Result<ResultadoLogin, NucleoError> {
        let sesion_supabase = control_acceso::nube::login(
            control_acceso::nube::base_url(),
            control_acceso::nube::apikey(),
            cedula,
            password,
        )?;

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
                if !secreto.trim().is_empty() {
                    let _ = self.refrescar_catalogo_sin_sesion_con_secreto(secreto);
                }
                self.core_lock().resolver_identidad_local(cedula)?
            }
            Err(otro) => return Err(otro.into()),
        };

        *self.sesion_lock() = Some(identidad.clone());
        self.iniciar_sesion_supabase(sesion_supabase.clone());

        // Best-effort a propósito -- mismo criterio que `login_supabase` en
        // desktop (ver el doc-comment de `AppCore::cachear_password_local`):
        // un fallo acá no debe tumbar un login que ya fue exitoso contra
        // Supabase, sólo deja sin el atajo offline a esta cuenta hasta el
        // próximo login online. Ver `Usuario::password_hash_confirmado_en`
        // y docs/decisiones-tecnicas.md, entrada 2026-09-18.
        //
        // Propaga `sesion_supabase.debe_cambiar_password` al caché a
        // propósito (hallazgo de auditoría 2026-09-24, MV-01/DF-03): si la
        // contraseña recién verificada es una temporal todavía sin
        // cambiar, un login sin conexión más adelante debe seguir
        // exigiendo el cambio, no aceptarla como si ya fuera definitiva.
        //
        // El resultado se liga a una variable ANTES del `if let` a propósito
        // -- `core_lock()` es un `MutexGuard`, y dejarlo como temporal
        // directo en el scrutinee lo mantendría vivo durante todo el bloque
        // (hasta la llave de cierre), no sólo durante la llamada -- riesgo
        // real de deadlock si algo más adelante necesitara el mismo candado.
        let resultado_cache = self.core_lock().cachear_password_local(
            identidad.id,
            password,
            sesion_supabase.debe_cambiar_password,
        );
        if let Err(error) = resultado_cache {
            log::warn!("no se pudo cachear el login offline: {error}");
        }

        Ok(ResultadoLogin {
            sesion: identidad.into(),
            debe_cambiar_password: sesion_supabase.debe_cambiar_password,
        })
    }
}
