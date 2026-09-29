//! Configuración del dispositivo, sincronización, Realtime y activos de otros equipos.

use control_acceso::application::GestionNubeError as GestionNubeErrorNucleo;

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

    /// `true` mientras la base no tenga ningún usuario todavía -- Kotlin lo
    /// usa para decidir si mostrar la pantalla de arranque (pegar el
    /// secreto) en vez del login (ver `MainActivity.kt`).
    pub fn requiere_configuracion_inicial(&self) -> Result<bool, NucleoError> {
        Ok(self.core_lock().requiere_configuracion_inicial()?)
    }

    /// Activación inicial de una base vacía. No persiste el secreto desde
    /// Rust: Android lo guarda con Android Keystore y sólo entrega el
    /// secreto descifrado en memoria para esta autenticación inicial. El
    /// candado del núcleo sólo se toma para lo local, nunca durante la red.
    ///
    /// Los parámetros de metadata (todos opcionales, `""` = no disponible)
    /// viajan una única vez, en esta primera autenticación -- ver
    /// `control_acceso::nube::MetadatosDispositivo`. No se vuelven a
    /// reenviar en cada renovación de token porque casi nunca cambian, y
    /// esto ya alcanza para que el panel de administración distinga el
    /// teléfono físico detrás de cada secreto (ver
    /// `docs/features-futuras/plan-sesion-unica-dispositivos.md`).
    #[allow(clippy::too_many_arguments)]
    pub fn configurar_dispositivo_inicial_con_secreto(
        &self,
        secreto: String,
        identificador_hardware: String,
        nombre_dispositivo: String,
        plataforma: String,
        version_build: String,
        app_version: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        if !self.core_lock().requiere_configuracion_inicial()? {
            return Err(NucleoError::from(GestionNubeErrorNucleo::YaConfigurado));
        }

        let cadena_opcional = |texto: String| (!texto.trim().is_empty()).then_some(texto);
        let metadata = control_acceso::nube::MetadatosDispositivo {
            identificador_hardware: cadena_opcional(identificador_hardware),
            nombre_dispositivo: cadena_opcional(nombre_dispositivo),
            plataforma: cadena_opcional(plataforma),
            version_build: cadena_opcional(version_build),
            app_version: cadena_opcional(app_version),
        };
        let token = self
            .autenticar_y_cachear(&secreto, Some(&metadata))
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }

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
            // Activación inicial: base recién configurada, sin ingresos
            // locales todavía -- mismo criterio que
            // `From<ResumenSincronizacionNucleo>` arriba y que el equivalente
            // en desktop/src-tauri/src/comandos/nube.rs.
            conflictos_ingreso: Vec::new(),
            conflictos_ingreso_proveedor: Vec::new(),
            conflictos_gafete: Vec::new(),
        })
    }

    /// Lee el secreto guardado por versiones móviles anteriores a Android
    /// Keystore. Kotlin lo usa sólo para migrarlo al almacén seguro nuevo.
    pub fn cargar_secreto_dispositivo_legado(
        &self,
        directorio: String,
        identificador_dispositivo: String,
    ) -> Option<String> {
        control_acceso::nube::credenciales::cargar_secreto_en_con_identificador(
            std::path::Path::new(&directorio),
            &identificador_dispositivo,
        )
    }

    /// Borra el archivo legado de `cargar_secreto_dispositivo_legado` --
    /// Kotlin lo llama justo después de migrar ese secreto al Keystore, para
    /// no dejar la copia vieja (en texto plano, ver el módulo
    /// `nube::credenciales`) huérfana en el almacenamiento de la app.
    pub fn borrar_secreto_dispositivo_legado(&self, directorio: String) -> Result<(), NucleoError> {
        control_acceso::nube::credenciales::borrar_secreto_en(std::path::Path::new(&directorio))
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })
    }

    /// Sincroniza usando el secreto ya descifrado por Android Keystore.
    /// Evita que el núcleo móvil lea un secreto persistido en texto plano.
    pub fn sincronizar_con_nube_con_secreto(
        &self,
        secreto: String,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        self.sincronizar_con_secreto(
            &secreto,
            control_acceso::nube::AlcanceSincronizacion::completo(),
        )
    }

    /// Aviso en vivo con los datos (`cambio_nube` con `registro`, tal cual
    /// llega por el canal, en JSON): guarda la fila directo en la base local
    /// sin consultar a la nube -- ver `control_acceso::nube::en_vivo`.
    /// `true` si la aplicó; `false` si el aviso no trae datos o su tabla
    /// todavía no los manda (queda para `sincronizar_cambios_con_secreto`,
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
    pub fn sincronizar_cambios_con_secreto(
        &self,
        secreto: String,
        tablas: Vec<String>,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        self.sincronizar_con_secreto(
            &secreto,
            control_acceso::nube::AlcanceSincronizacion::desde_tablas(&tablas),
        )
    }

    /// Lo mínimo para que Kotlin escuche Broadcast privado por sitio, con el
    /// secreto que Kotlin descifra de Android Keystore. El socket y sus
    /// reconexiones viven fuera del núcleo.
    pub fn sesion_realtime_nube_con_secreto(
        &self,
        secreto: String,
    ) -> Result<SesionRealtimeNube, NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }
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
    /// Usa el secreto que Kotlin descifra de Android Keystore; el candado
    /// del núcleo sólo se toma para autorizar, nunca durante la red.
    pub fn cerrar_ingreso_remoto_con_secreto(
        &self,
        secreto: String,
        uuid: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
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
        control_acceso::nube::cerrar_ingreso_remoto(&conexion, &contexto, &uuid, &actor.nombre)
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        Ok(())
    }

    /// Espejo de [`Self::cerrar_ingreso_remoto_con_secreto`], pero contra
    /// `ingresos_proveedor`.
    pub fn cerrar_ingreso_proveedor_remoto_con_secreto(
        &self,
        secreto: String,
        uuid: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
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
        control_acceso::nube::cerrar_ingreso_proveedor_remoto(
            &conexion,
            &contexto,
            &uuid,
            &actor.nombre,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;
        Ok(())
    }

    /// Espejo de [`Self::cerrar_ingreso_proveedor_remoto_con_secreto`],
    /// pero contra `prestamos_gafete_provisional`.
    pub fn cerrar_prestamo_gafete_provisional_remoto_con_secreto(
        &self,
        secreto: String,
        uuid: String,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache(&secreto)
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
        control_acceso::nube::cerrar_prestamo_gafete_provisional_remoto(
            &conexion,
            &contexto,
            &uuid,
            &actor.nombre,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })?;
        Ok(())
    }
}

impl Nucleo {
    pub(super) fn refrescar_catalogo_sin_sesion_con_secreto(
        &self,
        secreto: &str,
    ) -> Result<(), NucleoError> {
        let mapear_nube = |error: control_acceso::nube::NubeError| NucleoError::Interno {
            mensaje: interno(error),
        };
        let token = self.autenticar_con_cache(secreto).map_err(mapear_nube)?;
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
    pub(super) fn sincronizar_con_secreto(
        &self,
        secreto: &str,
        alcance: control_acceso::nube::AlcanceSincronizacion,
    ) -> Result<ResumenSincronizacion, NucleoError> {
        let _sincronizacion = self
            .sincronizacion_en_curso
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match self.intentar_sincronizar_con_secreto(secreto, alcance) {
            Err(FalloSincronizacion::TokenVencido) => {
                self.invalidar_token_cacheado();
                self.intentar_sincronizar_con_secreto(secreto, alcance)
                    .map_err(convertir_fallo_sincronizacion)
            }
            Err(otro) => Err(convertir_fallo_sincronizacion(otro)),
            Ok(resumen) => Ok(resumen),
        }
    }

    pub(super) fn intentar_sincronizar_con_secreto(
        &self,
        secreto: &str,
        alcance: control_acceso::nube::AlcanceSincronizacion,
    ) -> Result<ResumenSincronizacion, FalloSincronizacion> {
        let actor = self.actor_autenticado()?;
        self.core_lock().autorizar_uso_nube(&actor)?;

        // Renovación silenciosa de la sesión de Supabase Auth (mejor
        // esfuerzo). Sólo en la completa: una llamada de red más que el
        // pulso ya hace.
        if alcance.es_completo()
            && let Some(refresh_token) = self.refresh_token_supabase()
            && let Ok(sesion) = control_acceso::nube::refrescar(
                control_acceso::nube::base_url(),
                control_acceso::nube::apikey(),
                &refresh_token,
            )
        {
            self.iniciar_sesion_supabase(sesion);
        }

        let token = self.autenticar_con_cache(secreto).map_err(|error| {
            FalloSincronizacion::Nucleo(NucleoError::Interno {
                mensaje: interno(error),
            })
        })?;
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }

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

        let sesion_expulsada = !self.core_lock().sesion_sigue_activa(&actor);
        if sesion_expulsada {
            *self.sesion_lock() = None;
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
