//! Gestión de la persistencia en la nube (`docs/planes-implementados/plan-persistencia-nube.md`)
//! desde la fachada de aplicación. El dispositivo se vincula en el arranque
//! inicial canjeando un código del panel (`vincular_dispositivo_inicial`);
//! desde ahí se autentica con su propia clave (ver `nube::firmante` y
//! `docs/features-futuras/propuesta-registro-dispositivos.md`). Sincronizar, leer y cerrar ingresos remotos
//! (`Operacion::UsarNube`) es de cualquier rol -- uso diario normal (la
//! pantalla Activos los usa), no administración.

use crate::database::error::DatabaseError;
use crate::domain::autorizacion::Operacion;
use crate::nube::{IngresoProveedorRemoto, IngresoRemoto, PrestamoGafeteProvisionalRemoto};
use crate::services::autenticacion_service::UsuarioSesion;

use super::{AppCore, verificar_actor_activo};

/// Movimiento del espejo de Supabase, sin inventar valores para datos antiguos ausentes.
#[derive(Debug, Clone)]
pub struct MovimientoHistorialSitio {
    pub uuid: String,
    pub cedula: Option<String>,
    pub contratista_nombre: String,
    pub empresa_nombre: Option<String>,
    pub fecha_hora_ingreso: String,
    pub fecha_hora_salida: Option<String>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
    pub motivo_resultado: Option<String>,
    /// `"pc"`/`"mobile"` (o `None` para filas sincronizadas antes de que
    /// esto existiera) -- ver `nube::sincronizacion::FilaHistorialRemota`.
    pub dispositivo_entrada_tipo: Option<String>,
}

impl AppCore {
    /// Lee el historial recibido del sitio con el mismo rango y búsqueda del móvil.
    pub fn listar_historial_sitio(
        &self,
        actor: &UsuarioSesion,
        desde: chrono::DateTime<chrono::Utc>,
        hasta: chrono::DateTime<chrono::Utc>,
        texto: &str,
    ) -> Result<Vec<MovimientoHistorialSitio>, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let mut consulta = self.connection.prepare(
            "SELECT uuid, contratista_cedula, contratista_nombre, empresa_nombre,
                    hora_entrada, hora_salida, gafete_numero, usuario_entrada_nombre,
                    usuario_salida_nombre, motivo_resultado, dispositivo_entrada_tipo
             FROM historial_sitio
             WHERE hora_entrada >= ?1 AND hora_entrada < ?2
               AND (?3 = '' OR instr(lower(contratista_nombre), lower(?3)) > 0
                    OR instr(contratista_cedula, ?3) > 0)
             ORDER BY hora_entrada DESC, uuid LIMIT 30",
        )?;
        Ok(consulta
            .query_map(
                rusqlite::params![
                    crate::tiempo::serializar_utc(desde),
                    crate::tiempo::serializar_utc(hasta),
                    texto.trim()
                ],
                |row| {
                    Ok(MovimientoHistorialSitio {
                        uuid: row.get(0)?,
                        cedula: row.get(1)?,
                        contratista_nombre: row.get(2)?,
                        empresa_nombre: row.get(3)?,
                        fecha_hora_ingreso: row.get(4)?,
                        fecha_hora_salida: row.get(5)?,
                        gafete_numero: row.get(6)?,
                        usuario_ingreso_nombre: row.get(7)?,
                        usuario_salida_nombre: row.get(8)?,
                        motivo_resultado: row.get(9)?,
                        dispositivo_entrada_tipo: row.get(10)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GestionNubeError {
    #[error("Sólo una sesión ROOT activa puede gestionar la nube")]
    OperacionNoAutorizada,
    #[error("Su sesión no está autorizada para usar la nube")]
    UsoNoAutorizado,
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Autenticacion(#[from] crate::nube::NubeError),
    #[error(transparent)]
    Sincronizacion(#[from] crate::nube::SincronizacionError),
    #[error("Este dispositivo ya tiene usuarios locales -- no es un bootstrap de base vacía")]
    YaConfigurado,
    #[error(transparent)]
    Usuario(#[from] crate::services::error::UsuarioServiceError),
}

/// Resultado de una sincronización manual -- lo suficiente para que la
/// pantalla muestre "sitio X, dispositivo Y: 12 enviados, 0 fallidos, 2
/// abiertos del otro lado".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumenSincronizacion {
    pub enviados: u32,
    pub fallidos: u32,
    pub remotos_abiertos: u32,
    pub cierres_recibidos: u32,
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub gafetes_recibidos: u32,
    /// Vehículos/encargados KOF del catálogo de rutas recibidos -- ver
    /// `nube::recibir_catalogo_rutas_del_sitio`.
    pub vehiculos_ruta_recibidos: u32,
    pub encargados_ruta_recibidos: u32,
    pub movimientos_historial_recibidos: u32,
    /// Citas nuevas/actualizadas recibidas para el punto de acceso (con sus
    /// visitantes) -- ver `nube::recibir_citas_del_sitio`.
    pub citas_recibidas: u32,
    /// Movimientos de visita (entrada/salida) del sitio recibidos hacia la
    /// caché local `historial_visitas_sitio` -- ver
    /// `nube::recibir_historial_visitas_del_sitio`.
    pub historial_visitas_recibidos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    /// `true` si esta misma sincronización trajo la baja/desactivación de
    /// quien la disparó -- ver `AppCore::sesion_sigue_activa`. Decisión
    /// explícita del usuario: el login local (offline-first) no puede
    /// exigir estar en línea para dejar operar, pero una sesión YA abierta
    /// que se entera -- en cuanto vuelve a tener señal -- de que a su
    /// usuario lo desactivaron en otro dispositivo, se cierra sola en vez
    /// de seguir operando con un permiso que ya no existe. Quien recibe
    /// esto debe cerrar la sesión local y volver al login.
    pub sesion_expulsada: bool,
}

impl AppCore {
    /// Bootstrap de una base sin ningún usuario todavía
    /// (`requiere_configuracion_inicial() == true`) -- sin sesión posible,
    /// porque no hay con quién autenticar todavía. Canjea el código de
    /// vinculación que emitió el panel (el equipo genera su clave y sólo
    /// manda la pública, ver `nube::firmante`) y trae el catálogo remoto
    /// (usuarios incluidos), para que el próximo Login tenga con quién
    /// autenticar. Requiere [`Self::establecer_firmante_dispositivo`].
    /// Cada usuario que llega así arranca con el centinela
    /// `SIN_PASSWORD_LOCAL` (ver `recibir_catalogo_del_sitio`), así que el
    /// primer login de cualquiera de ellos cae en `AutenticacionError::SinPasswordLocal`
    /// y de ahí al login contra Supabase Auth (`login_supabase`/
    /// `autenticar_supabase`, ver docs/planes-implementados/plan-autenticacion-supabase-auth.md)
    /// -- no en `AppCore::fijar_password_inicial`, eliminada en la
    /// auditoría 2026-09-24 (NR-07/NS-25) por no tener ningún llamador real.
    ///
    /// Se rechaza a propósito si ya existe algún usuario local -- este
    /// camino es sólo el bootstrap de una base vacía: un equipo reinstalado
    /// se registra como dispositivo nuevo y el anterior se retira en el panel.
    /// `metadata`, si viene, viaja en el mismo request que el canje -- ver
    /// `nube::MetadatosDispositivo`. Cada plataforma decide qué mandar (o
    /// `None`): escritorio arma la suya en
    /// `comandos::nube::vincular_dispositivo_inicial`.
    ///
    /// Única excepción a la regla "nunca red con el candado del núcleo
    /// tomado" (ver `AppCore`): quien la llama la usa con el candado tomado
    /// y habla con la nube. Se acepta porque corre una sola vez, con la base
    /// vacía y la pantalla de activación esperando: no hay otra operación
    /// que pueda quedar trabada. El móvil ni siquiera la usa
    /// (`Nucleo::vincular_dispositivo_inicial` suelta el candado antes de la
    /// red).
    pub fn vincular_dispositivo_inicial(
        &self,
        codigo: &str,
        metadata: Option<&crate::nube::MetadatosDispositivo>,
        perfil: crate::nube::PerfilDispositivo,
    ) -> Result<ResumenSincronizacion, GestionNubeError> {
        if !self.requiere_configuracion_inicial()? {
            return Err(GestionNubeError::YaConfigurado);
        }

        let token = self.vincular_y_cachear(codigo, metadata)?;
        self.recibir_catalogo_inicial(token, perfil)
    }

    /// Configura quién firma por este equipo (ver `nube::firmante`). Cada
    /// plataforma lo llama una vez al arrancar, antes de cualquier operación
    /// de nube.
    pub fn establecer_firmante_dispositivo(
        &self,
        firmante: std::sync::Arc<dyn crate::nube::FirmanteDispositivo>,
    ) {
        self.cache_token.establecer_firmante(firmante);
    }

    /// Base vacía recién vinculada: nada propio que mandar, sólo el catálogo
    /// para que el primer login tenga con quién autenticar.
    fn recibir_catalogo_inicial(
        &self,
        token: crate::nube::TokenDispositivo,
        perfil: crate::nube::PerfilDispositivo,
    ) -> Result<ResumenSincronizacion, GestionNubeError> {
        let contexto = crate::nube::ContextoSincronizacion {
            base_url: crate::nube::base_url(),
            apikey: crate::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let recibido = crate::nube::recibir(
            &self.connection,
            &contexto,
            crate::nube::AlcanceSincronizacion::catalogo_y_rutas(),
            perfil,
        )?;

        Ok(ResumenSincronizacion {
            enviados: 0,
            fallidos: 0,
            remotos_abiertos: 0,
            cierres_recibidos: 0,
            empresas_recibidas: recibido.catalogo.empresas_recibidas,
            contratistas_recibidos: recibido.catalogo.contratistas_recibidos,
            gafetes_recibidos: recibido.catalogo.gafetes_recibidos,
            vehiculos_ruta_recibidos: recibido.catalogo_rutas.vehiculos_recibidos,
            encargados_ruta_recibidos: recibido.catalogo_rutas.encargados_recibidos,
            movimientos_historial_recibidos: 0,
            citas_recibidas: 0,
            historial_visitas_recibidos: 0,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada: false,
        })
    }

    /// Lectura pura de la caché local `ingresos_remotos` -- ya la llenó la
    /// última `nube::sincronizar`, no hace falta red para mostrarla.
    pub fn listar_ingresos_remotos(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<IngresoRemoto>, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let mut statement = self.connection.prepare(
            "SELECT uuid, contratista_nombre, hora_entrada, usuario_entrada_nombre,
                    contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero,
                    placa
             FROM ingresos_remotos ORDER BY hora_entrada",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok(IngresoRemoto {
                    uuid: row.get(0)?,
                    contratista_nombre: row.get(1)?,
                    hora_entrada: row.get(2)?,
                    usuario_entrada_nombre: row.get(3)?,
                    contratista_cedula: row.get(4)?,
                    empresa_nombre: row.get(5)?,
                    tipo_ingreso: row.get(6)?,
                    medio_ingreso: row.get(7)?,
                    gafete_numero: row.get(8)?,
                    placa: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
    }

    /// Espejo de [`Self::listar_ingresos_remotos`], pero contra la caché
    /// `ingresos_proveedor_remotos`.
    pub fn listar_ingresos_proveedor_remotos(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<IngresoProveedorRemoto>, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let mut statement = self.connection.prepare(
            "SELECT uuid, cedula, nombre, empresa_nombre, placa, gafete_numero,
                    hora_entrada, usuario_entrada_nombre
             FROM ingresos_proveedor_remotos ORDER BY hora_entrada",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok(IngresoProveedorRemoto {
                    uuid: row.get(0)?,
                    cedula: row.get(1)?,
                    nombre: row.get(2)?,
                    empresa_nombre: row.get(3)?,
                    placa: row.get(4)?,
                    gafete_numero: row.get(5)?,
                    hora_entrada: row.get(6)?,
                    usuario_entrada_nombre: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
    }

    /// Espejo de [`Self::listar_ingresos_proveedor_remotos`], pero contra
    /// la caché `prestamos_gafete_provisional_remotos`.
    pub fn listar_prestamos_gafete_provisional_remotos(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<PrestamoGafeteProvisionalRemoto>, GestionNubeError> {
        self.autorizar_uso_nube(actor)?;
        let mut statement = self.connection.prepare(
            "SELECT uuid, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                    hora_entrega, usuario_entrega_nombre
             FROM prestamos_gafete_provisional_remotos ORDER BY hora_entrega",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok(PrestamoGafeteProvisionalRemoto {
                    uuid: row.get(0)?,
                    encargado_nombre: row.get(1)?,
                    encargado_codigo_empleado: row.get(2)?,
                    gafete_numero: row.get(3)?,
                    hora_entrega: row.get(4)?,
                    usuario_entrega_nombre: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
    }

    /// Exclusivo de ROOT a propósito (no pasa por `RolUsuario::puede()`, que
    /// quedó aplanado a `true` siempre en el aplanado de roles -- ver
    /// `domain::autorizacion`): a diferencia de las operaciones que sí se
    /// aplanaron, nadie decidió abrir la gestión de la vinculación a
    /// cualquier rol, así que se restaura el chequeo directo. Dormido en la
    /// práctica (ver el doc-comment del módulo), pero sigue siendo la única
    /// puerta real si algún día algo vuelve a llamarlo.
    pub fn autorizar_gestion_nube(&self, actor: &UsuarioSesion) -> Result<(), GestionNubeError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => GestionNubeError::Sqlite(error),
                _ => GestionNubeError::OperacionNoAutorizada,
            })?
            .ok_or(GestionNubeError::OperacionNoAutorizada)?;
        if usuario.rol != crate::models::usuario::RolUsuario::Root {
            return Err(GestionNubeError::OperacionNoAutorizada);
        }
        Ok(())
    }

    /// Sólo autoriza -- no toca la red ni la clave del dispositivo.
    pub fn autorizar_uso_nube(&self, actor: &UsuarioSesion) -> Result<(), GestionNubeError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => GestionNubeError::Sqlite(error),
                _ => GestionNubeError::UsoNoAutorizado,
            })?
            .ok_or(GestionNubeError::UsoNoAutorizado)?;
        if !usuario.rol.puede(Operacion::UsarNube) {
            return Err(GestionNubeError::UsoNoAutorizado);
        }
        Ok(())
    }

    /// Todo lo que un token recién entregado le enseña a este núcleo: el
    /// desfase de reloj (si lo trae) y la unidad y etiqueta del equipo, para
    /// mostrarlas. Un solo lugar para todos los que reciben tokens.
    pub fn aplicar_token(&self, token: &crate::nube::TokenDispositivo) {
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.actualizar_desfase_reloj(desfase_ms);
        }
        self.recordar_identidad_equipo(token);
    }

    /// Guarda la unidad y la etiqueta que trae el token, sólo si cambiaron.
    /// Un token sin ellas (servidor anterior) no borra lo que ya se sabía.
    fn recordar_identidad_equipo(&self, token: &crate::nube::TokenDispositivo) {
        use crate::database::queries::identidad_equipo;

        let nueva = identidad_equipo::IdentidadEquipo {
            unidad: token.sitio_nombre.clone(),
            etiqueta: token.etiqueta.clone(),
        };
        if nueva.esta_vacia()
            || identidad_equipo::leer(&self.connection).is_ok_and(|actual| actual == nueva)
        {
            return;
        }
        if let Err(error) = identidad_equipo::guardar(&self.connection, &nueva) {
            log::warn!("no se pudo guardar la unidad de este equipo: {error}");
        }
    }

    /// Unidad y etiqueta de este equipo, para el login y la barra de
    /// estado. Vacía si todavía no llegaron de la nube.
    pub fn identidad_equipo(&self) -> crate::database::queries::identidad_equipo::IdentidadEquipo {
        crate::database::queries::identidad_equipo::leer(&self.connection).unwrap_or_default()
    }

    /// Canjea el código (ver [`Self::vincular_dispositivo_inicial`]). Sin
    /// `metadata`, pero si ya se llamó [`AppCore::establecer_version_app`],
    /// arma una `MetadatosDispositivo` mínima (sólo `app_version`) para que
    /// el receptor pueda aplicar `VERSION_MINIMA_ACEPTADA` -- ver
    /// `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto 9.
    fn vincular_y_cachear(
        &self,
        codigo: &str,
        metadata: Option<&crate::nube::MetadatosDispositivo>,
    ) -> Result<crate::nube::TokenDispositivo, GestionNubeError> {
        let metadata_con_version;
        let metadata = match (metadata, &self.version_app) {
            (Some(metadata), _) => Some(metadata),
            (None, Some(version)) => {
                metadata_con_version = crate::nube::MetadatosDispositivo {
                    app_version: Some(version.clone()),
                    ..Default::default()
                };
                Some(&metadata_con_version)
            }
            (None, None) => None,
        };
        let token = self.cache_token.vincular(codigo, metadata)?;
        self.aplicar_token(&token);
        Ok(token)
    }

    /// Corrige el reloj de este `AppCore` con un desfase ya medido (ver
    /// `TokenDispositivo::desfase_reloj_ms`) -- no-op si el reloj inyectado
    /// no es [`crate::tiempo::RelojCorregido`] (CLI/TUI siguen con
    /// `RelojSistema`, que ignora esta llamada). Pública porque algunas
    /// autenticaciones pasan por fuera de `AppCore` a propósito (Tauri
    /// evita retener el `Mutex` compartido durante la parte de red, ver
    /// `comandos/nube.rs::autenticar` en el escritorio) y necesitan un
    /// punto para devolver lo medido. Cada autenticación exitosa vuelve a
    /// medir y sobrescribe -- no acumula, así que un desfase que ya se
    /// corrigió (o empeoró) en Windows se refleja solo, sin reiniciar la
    /// app. También se guarda en la base (ver
    /// `database::queries::desfase_reloj`) para aplicarlo desde el próximo
    /// arranque, antes de hablar con la nube o si no hay internet.
    pub fn actualizar_desfase_reloj(&self, desfase_ms: i64) {
        use crate::database::queries::desfase_reloj;

        self.reloj.actualizar_desfase_ms(desfase_ms);
        // La corrección en memoria ya quedó aplicada; si guardar falla sólo
        // se pierde tenerla lista en el próximo arranque.
        if let Err(error) = desfase_reloj::guardar(&self.connection, desfase_ms) {
            log::warn!("no se pudo guardar el desfase de reloj medido: {error}");
        }
        if let Some(ancla) = self.reloj.ancla()
            && let Err(error) = desfase_reloj::guardar_ancla(&self.connection, &ancla)
        {
            log::warn!("no se pudo guardar el ancla de hora: {error}");
        }
    }

    /// La hora actual según el reloj de este núcleo (corregido contra el
    /// servidor, ver [`crate::tiempo::RelojCorregido`]). Para lo que se
    /// sella fuera de `AppCore` -- los cierres directos contra la nube (ver
    /// `nube::cerrar_ingreso_remoto`) -- con la misma hora que un registro
    /// local, nunca con el reloj crudo del equipo.
    pub fn ahora_utc(&self) -> chrono::DateTime<chrono::Utc> {
        self.reloj.ahora_utc()
    }

    /// Último desfase medido (reloj del equipo MENOS el del servidor, en
    /// ms), o `None` si nunca se midió. Para diagnóstico: permite a la app
    /// comparar horas del servidor con las del equipo (p. ej. la latencia
    /// de los avisos en vivo).
    pub fn desfase_reloj_ms(&self) -> Option<i64> {
        crate::database::queries::desfase_reloj::leer(&self.connection)
            .ok()
            .flatten()
    }
}
