//! Vinculación del dispositivo, sincronización, Realtime y activos de otros equipos.

use control_acceso::application::GestionNubeError as GestionNubeErrorNucleo;

use crate::firmante::{AlmacenClaveDispositivo, FirmanteMovil};
use crate::{
    FalloSincronizacion, IngresoProveedorRemoto, IngresoRemoto, Nucleo, NucleoError,
    PrestamoGafeteProvisionalRemoto, ResumenSincronizacion, SesionRealtimeNube,
    convertir_fallo_sincronizacion, interno,
};

#[uniffi::export]
impl Nucleo {
    /// Último desfase medido del reloj del teléfono contra el servidor (ms;
    /// positivo = el teléfono va adelantado), `None` si nunca se midió. Se
    /// mide en cada autenticación con precisión de milisegundos (ver
    /// `control_acceso::nube::reloj_preciso`). Lo usa la telemetría de
    /// diagnóstico para corregir la latencia de los avisos en vivo.
    pub fn desfase_reloj_ms(&self) -> Option<i64> {
        self.core_lock().desfase_reloj_ms()
    }

    /// Unidad y etiqueta de este teléfono (ver [`crate::IdentidadEquipo`]).
    /// No necesita sesión: el login también la muestra.
    pub fn identidad_equipo(&self) -> crate::IdentidadEquipo {
        let identidad = self.core_lock().identidad_equipo();
        crate::IdentidadEquipo {
            unidad: identidad.unidad,
            etiqueta: identidad.etiqueta,
        }
    }

    /// `true` mientras la base no tenga ningún usuario todavía -- Kotlin lo
    /// usa para decidir si mostrar la pantalla de arranque (vincular con un
    /// código) en vez del login (ver `MainActivity.kt`).
    pub fn requiere_configuracion_inicial(&self) -> Result<bool, NucleoError> {
        Ok(self.core_lock().requiere_configuracion_inicial()?)
    }

    /// Configura quién firma por este teléfono (Android Keystore, ver
    /// `AlmacenClaveKeystore.kt`). Kotlin lo llama una sola vez, apenas abre
    /// el núcleo y antes de cualquier operación de nube.
    pub fn establecer_almacen_clave(&self, almacen: std::sync::Arc<dyn AlmacenClaveDispositivo>) {
        self.cache_token
            .establecer_firmante(std::sync::Arc::new(FirmanteMovil(almacen)));
    }

    /// Descarta el token cacheado. Lo llama `NubeRealtime.kt` al recibir un
    /// aviso de expulsión (retirado en el panel): sin esto el teléfono seguía
    /// usando su token de hasta 1 h, las consultas en vivo le devolvían
    /// vacío (la política restrictiva filtra en silencio) y un chequeo como
    /// "ingreso activo en otro sitio" pasaba como si no hubiera conflicto.
    /// Con el token descartado, la próxima operación de nube va a
    /// `device-auth` y recibe el motivo real. Mismo criterio que
    /// `descartar_token_nube` en escritorio.
    pub fn descartar_token_nube(&self) {
        self.invalidar_token_cacheado();
    }

    /// `true` si este teléfono ya canjeó un código y tiene su clave en
    /// Android Keystore. Sin vincular no hay nube: Kotlin salta la
    /// sincronización de fondo y los chequeos en vivo no tocan la red.
    pub fn nube_configurada(&self) -> bool {
        self.cache_token.vinculado()
    }

    /// `dispositivo_id` al que está vinculada la clave de este teléfono, o
    /// `None` si nunca se vinculó.
    pub fn dispositivo_vinculado(&self) -> Option<String> {
        self.cache_token.dispositivo_vinculado()
    }

    /// Activación inicial de una base vacía: canjea el código de vinculación
    /// del panel (el teléfono genera su clave en Android Keystore y sólo
    /// manda la pública) y trae el catálogo. El candado del núcleo sólo se
    /// toma para lo local, nunca durante la red.
    ///
    /// Los parámetros de metadata (todos opcionales, `""` = no disponible)
    /// viajan en el mismo canje -- ver
    /// `control_acceso::nube::MetadatosDispositivo`.
    #[allow(clippy::too_many_arguments)]
    pub fn vincular_dispositivo_inicial(
        &self,
        codigo: String,
        identificador_hardware: String,
        nombre_dispositivo: String,
        plataforma: String,
        version_build: String,
        app_version: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        if !self.core_lock().requiere_configuracion_inicial()? {
            return Err(NucleoError::from(GestionNubeErrorNucleo::YaConfigurado));
        }

        let metadata = metadata_del_telefono(
            identificador_hardware,
            nombre_dispositivo,
            plataforma,
            version_build,
            app_version,
        );
        let token = self
            .vincular_y_cachear(&codigo, Some(&metadata))
            .map_err(error_de_nube_para_mostrar)?;

        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        // Base vacía: sólo catálogo y rutas, para que el primer login tenga
        // con quién autenticar y "Gafetes KOF" tenga encargados.
        let recibido = control_acceso::nube::recibir(
            &conexion,
            &contexto,
            control_acceso::nube::AlcanceSincronizacion::catalogo_y_rutas(),
            control_acceso::nube::PerfilDispositivo::Movil,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;

        Ok(ResumenSincronizacion {
            enviados: 0,
            fallidos: 0,
            remotos_abiertos: 0,
            cierres_recibidos: 0,
            empresas_recibidas: recibido.catalogo.empresas_recibidas,
            contratistas_recibidos: recibido.catalogo.contratistas_recibidos,
            gafetes_recibidos: recibido.catalogo.gafetes_recibidos,
            movimientos_historial_recibidos: 0,
            citas_recibidas: 0,
            historial_visitas_recibidos: 0,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada: false,
            sesion_en_otra_unidad: false,
            // Activación inicial: base recién configurada, sin ingresos
            // locales todavía -- mismo criterio que
            // `From<ResumenSincronizacionNucleo>` arriba y que el equivalente
            // en desktop/src-tauri/src/comandos/nube.rs.
            conflictos_ingreso: Vec::new(),
            conflictos_ingreso_proveedor: Vec::new(),
            conflictos_gafete: Vec::new(),
        })
    }

    /// Sincronización completa, autenticada con la clave del teléfono.
    pub fn sincronizar_con_nube(&self) -> Result<ResumenSincronizacion, NucleoError> {
        self.sincronizar(control_acceso::nube::AlcanceSincronizacion::completo())
    }

    /// Aviso en vivo con los datos (`cambio_nube` con `registro`, tal cual
    /// llega por el canal, en JSON): guarda la fila directo en la base local
    /// sin consultar a la nube -- ver `control_acceso::nube::en_vivo`.
    /// `true` si la aplicó; `false` si el aviso no trae datos o su tabla
    /// todavía no los manda (queda para `sincronizar_cambios`,
    /// que corre igual detrás). Sobre la conexión secundaria: nunca toma el
    /// candado del núcleo.
    pub fn aplicar_cambio_nube(&self, aviso_json: String) -> Result<bool, NucleoError> {
        let aviso: serde_json::Value =
            serde_json::from_str(&aviso_json).map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        let conexion = self.conexion_secundaria()?;
        control_acceso::nube::aplicar_cambio_en_vivo(
            &conexion,
            &aviso,
            control_acceso::nube::PerfilDispositivo::Movil,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })
    }

    /// Sincronización disparada por avisos en vivo (`cambio_nube`, ver
    /// `NubeRealtime.kt`): corre sólo las etapas de las tablas que
    /// cambiaron (`payload.table`), ver
    /// `control_acceso::nube::AlcanceSincronizacion`. Antes cada aviso
    /// corría la sincronización completa (~12 consultas a la nube por un
    /// solo cambio). Una tabla desconocida o una lista vacía caen en la
    /// completa. La bandeja de salida se drena siempre.
    pub fn sincronizar_cambios(
        &self,
        tablas: Vec<String>,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        self.sincronizar(control_acceso::nube::AlcanceSincronizacion::desde_tablas(
            &tablas,
        ))
    }

    /// Sólo vacía la bandeja de salida, sin bajar nada: lo que corre tras
    /// un cambio hecho en este teléfono (mismo criterio que
    /// `enviar_cambios_nube` en escritorio). Antes un registro local corría
    /// la sincronización completa antes de subirse (telemetría de staging:
    /// ~1,6 s de promedio y hasta 5 s), y el otro equipo recién veía el
    /// cambio al terminar. El pulso periódico sigue siendo completo.
    pub fn enviar_cambios(&self) -> Result<ResumenSincronizacion, NucleoError> {
        self.sincronizar(control_acceso::nube::AlcanceSincronizacion::solo_envio())
    }

    /// Lo mínimo para que Kotlin escuche Broadcast privado por sitio. El
    /// socket y sus
    /// reconexiones viven fuera del núcleo.
    pub fn sesion_realtime_nube(&self) -> Result<SesionRealtimeNube, NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache()
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        let topic = format!("sitio:{}", token.sitio_id);

        Ok(SesionRealtimeNube {
            base_url: control_acceso::nube::base_url().to_string(),
            apikey: control_acceso::nube::apikey().to_string(),
            access_token: token.access_token,
            expires_in: token.expires_in,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            topic,
        })
    }

    /// Lectura pura de la caché local `ingresos_remotos` -- no hace falta
    /// red para mostrarla, ya la llenó la última sincronización.
    pub fn listar_ingresos_remotos(&self) -> Result<Vec<IngresoRemoto>, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .listar_ingresos_remotos(&actor)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Espejo de [`Self::listar_ingresos_remotos`], pero contra la caché
    /// `ingresos_proveedor_remotos`.
    pub fn listar_ingresos_proveedor_remotos(
        &self,
    ) -> Result<Vec<IngresoProveedorRemoto>, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .listar_ingresos_proveedor_remotos(&actor)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Espejo de [`Self::listar_ingresos_proveedor_remotos`], pero contra
    /// la caché `prestamos_gafete_provisional_remotos`.
    pub fn listar_prestamos_gafete_provisional_remotos(
        &self,
    ) -> Result<Vec<PrestamoGafeteProvisionalRemoto>, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .listar_prestamos_gafete_provisional_remotos(&actor)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Cierra, contra la nube, un ingreso abierto por el otro dispositivo
    /// del mismo sitio -- nunca toca el historial local de este teléfono.
    /// El candado
    /// del núcleo sólo se toma para autorizar, nunca durante la red.
    pub fn cerrar_ingreso_remoto(&self, uuid: String) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache()
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let hora = self.core_lock().ahora_utc();
        control_acceso::nube::cerrar_ingreso_remoto(
            &conexion,
            &contexto,
            &uuid,
            &actor.nombre,
            hora,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;
        Ok(())
    }

    /// Espejo de [`Self::cerrar_ingreso_remoto`], pero contra
    /// `ingresos_proveedor`.
    pub fn cerrar_ingreso_proveedor_remoto(&self, uuid: String) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache()
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let hora = self.core_lock().ahora_utc();
        control_acceso::nube::cerrar_ingreso_proveedor_remoto(
            &conexion,
            &contexto,
            &uuid,
            &actor.nombre,
            hora,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;
        Ok(())
    }

    /// Espejo de [`Self::cerrar_ingreso_proveedor_remoto`],
    /// pero contra `prestamos_gafete_provisional`.
    pub fn cerrar_prestamo_gafete_provisional_remoto(
        &self,
        uuid: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache()
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let hora = self.core_lock().ahora_utc();
        control_acceso::nube::cerrar_prestamo_gafete_provisional_remoto(
            &conexion,
            &contexto,
            &uuid,
            &actor.nombre,
            hora,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;
        Ok(())
    }
}

impl Nucleo {
    pub(super) fn refrescar_catalogo_sin_sesion(&self) -> Result<(), NucleoError> {
        let mapear_nube = |error: control_acceso::nube::NubeError| NucleoError::Interno {
            mensaje: interno(error),
        };
        let token = self.autenticar_con_cache().map_err(mapear_nube)?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        control_acceso::nube::recibir(
            &conexion,
            &contexto,
            control_acceso::nube::AlcanceSincronizacion::solo_catalogo(),
            control_acceso::nube::PerfilDispositivo::Movil,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;
        Ok(())
    }

    /// Reintenta UNA vez si el intento falla porque el token de dispositivo
    /// cacheado, que `autenticar_con_cache` creía vigente, resultó
    /// rechazado por el receptor a mitad de camino -- ver
    /// `SincronizacionError::token_dispositivo_vencido` y el mismo patrón en
    /// `desktop/src-tauri/src/comandos/nube.rs::ejecutar_sincronizacion`.
    /// El candado de `sincronizacion_en_curso` envuelve los DOS intentos,
    /// así ninguna otra sincronización se cuela entre el fallo y el
    /// reintento. Las etapas las decide `control_acceso::nube::sincronizar`
    /// con `alcance` y el perfil móvil (sin historiales).
    pub(super) fn sincronizar(
        &self,
        alcance: control_acceso::nube::AlcanceSincronizacion,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        let _sincronizacion = self
            .sincronizacion_en_curso
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match self.intentar_sincronizar(alcance) {
            Err(FalloSincronizacion::TokenVencido) => {
                self.invalidar_token_cacheado();
                self.intentar_sincronizar(alcance)
                    .map_err(convertir_fallo_sincronizacion)
            }
            Err(otro) => Err(convertir_fallo_sincronizacion(otro)),
            Ok(resumen) => Ok(resumen),
        }
    }

    pub(super) fn intentar_sincronizar(
        &self,
        alcance: control_acceso::nube::AlcanceSincronizacion,
    ) -> Result<ResumenSincronizacion, FalloSincronizacion> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;

        let token = self.autenticar_con_cache().map_err(|error| {
            FalloSincronizacion::Nucleo(NucleoError::Interno {
                mensaje: interno(error),
            })
        })?;

        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let resumen = control_acceso::nube::sincronizar(
            &conexion,
            &contexto,
            alcance,
            control_acceso::nube::PerfilDispositivo::Movil,
        )?;

        let mut sesion_expulsada = !self.core_lock().sesion_sigue_activa(&actor);
        if sesion_expulsada {
            *self.sesion_lock() = None;
        }

        // Sesión única por unidad -- mismo criterio que escritorio
        // (`desktop/src-tauri/src/comandos/nube.rs`): cada sincronización,
        // incluida la que sigue al login, registra la sesión en la nube y
        // pregunta si sigue vigente. Falla "abierto": sin red no expulsa.
        let mut sesion_en_otra_unidad = false;
        let inicio = self
            .sesion_lock()
            .as_ref()
            .map(|(_, inicio)| inicio.clone());
        if let (false, Some(inicio)) = (sesion_expulsada, inicio) {
            match control_acceso::nube::sesion_en_unidad(
                control_acceso::nube::base_url(),
                control_acceso::nube::apikey(),
                &token,
                &actor.cedula,
                &inicio,
            ) {
                Ok(control_acceso::nube::EstadoSesionUnidad::Desplazada) => {
                    *self.sesion_lock() = None;
                    sesion_expulsada = true;
                    sesion_en_otra_unidad = true;
                }
                Ok(_) => {}
                Err(error) => log::info!("no se pudo verificar la sesión en la nube: {error}"),
            }
        }

        Ok(ResumenSincronizacion {
            enviados: resumen.enviados,
            fallidos: resumen.fallidos,
            remotos_abiertos: resumen.remotos_abiertos,
            cierres_recibidos: resumen.cierres_recibidos,
            empresas_recibidas: resumen.catalogo.empresas_recibidas,
            contratistas_recibidos: resumen.catalogo.contratistas_recibidos,
            gafetes_recibidos: resumen.catalogo.gafetes_recibidos,
            movimientos_historial_recibidos: resumen.movimientos_historial_recibidos,
            citas_recibidas: resumen.citas_recibidas,
            historial_visitas_recibidos: resumen.historial_visitas_recibidos,
            sitio_id: token.sitio_id,
            dispositivo_id: token.dispositivo_id,
            tipo: token.tipo,
            sesion_expulsada,
            sesion_en_otra_unidad,
            conflictos_ingreso: resumen
                .conflictos_ingreso
                .into_iter()
                .map(Into::into)
                .collect(),
            conflictos_ingreso_proveedor: resumen
                .conflictos_ingreso_proveedor
                .into_iter()
                .map(Into::into)
                .collect(),
            conflictos_gafete: resumen
                .conflictos_gafete
                .into_iter()
                .map(Into::into)
                .collect(),
        })
    }
}

/// Metadata del teléfono para el canje: `""` significa "no disponible".
fn metadata_del_telefono(
    identificador_hardware: String,
    nombre_dispositivo: String,
    plataforma: String,
    version_build: String,
    app_version: String,
) -> control_acceso::nube::MetadatosDispositivo {
    let opcional = |texto: String| (!texto.trim().is_empty()).then_some(texto);
    control_acceso::nube::MetadatosDispositivo {
        identificador_hardware: opcional(identificador_hardware),
        nombre_dispositivo: opcional(nombre_dispositivo),
        plataforma: opcional(plataforma),
        version_build: opcional(version_build),
        app_version: opcional(app_version),
    }
}

/// Los rechazos de la vinculación (código inválido o vencido, clave en uso)
/// son para la persona que está frente al teléfono: viajan como
/// `Rechazado`, con el mismo texto que en escritorio.
fn error_de_nube_para_mostrar(error: control_acceso::nube::NubeError) -> NucleoError {
    NucleoError::Rechazado {
        mensaje: control_acceso::mensajes::mensaje_nube(error),
    }
}
