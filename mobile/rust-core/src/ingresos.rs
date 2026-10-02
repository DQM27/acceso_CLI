//! Ingreso y salida de contratistas: preparar, registrar, buscar y listar activos.

use control_acceso::database::queries::Igualdad;
use control_acceso::database::queries::contratistas::FiltroContratistas as FiltroContratistasNucleo;
use control_acceso::database::queries::ingresos::FiltroIngresosActivos as FiltroIngresosActivosNucleo;

use crate::{
    ContratistaResumen, IngresoActivoResumen, MedioIngreso, ModoBusquedaActivos, Nucleo,
    NucleoError, PreparacionIngreso, ResultadoRegistroEntrada, interno,
};

#[uniffi::export]
impl Nucleo {
    /// Vista previa antes de confirmar — misma decisión que ya toma la GUI
    /// de escritorio (`desktop/src/pantallas/NuevoIngresoModal.tsx`): no
    /// rechaza PRAIND vencido/ingreso activo aquí, sólo informa; quien llama
    /// (Kotlin) decide si deja continuar mirando los campos ya calculados.
    pub fn preparar_ingreso(&self, contratista_id: i64) -> Result<PreparacionIngreso, NucleoError> {
        Ok(self.core_lock().preparar_ingreso(contratista_id)?.into())
    }

    /// Vista previa con TODAS las reglas: las locales (este equipo y el
    /// otro equipo del sitio) y, con nube, la verificación en vivo de que la
    /// persona no tenga un ingreso activo en ningún sitio. Las decide
    /// `application::preparar_ingreso_verificado`, la misma que usa
    /// escritorio; Kotlin sólo muestra `mensaje_bloqueo`. Sin vincular =
    /// nube sin configurar (sólo reglas locales). Nunca toca `core_lock()`
    /// durante la red.
    pub fn preparar_ingreso_verificado(
        &self,
        contratista_id: i64,
    ) -> Result<PreparacionIngreso, NucleoError> {
        let actor = self.actor_autenticado().ok();
        let nube = &self.cache_token;
        let (preparacion, bloqueo) = control_acceso::application::preparar_ingreso_verificado(
            || self.core_lock(),
            nube,
            actor.as_ref(),
            contratista_id,
        )?;
        let mut preparacion: PreparacionIngreso = preparacion.into();
        preparacion.mensaje_bloqueo = bloqueo
            .as_ref()
            .map(control_acceso::mensajes::mensaje_bloqueo_ingreso);
        Ok(preparacion)
    }

    pub fn registrar_ingreso(
        &self,
        contratista_id: i64,
        medio: MedioIngreso,
        gafete: Option<i64>,
        placa: Option<String>,
    ) -> Result<ResultadoRegistroEntrada, NucleoError> {
        let actor = self.actor_autenticado()?;
        // Kotlin manda lo tipeado tal cual; qué placa corresponde al medio
        // lo decide `AppCore::registrar_ingreso` (`placa_segun_medio`).
        Ok(self
            .core_lock()
            .registrar_ingreso(&actor, contratista_id, medio.into(), gafete, placa)?
            .into())
    }

    /// Ingreso con TODAS sus reglas en una sola llamada: locales, y con nube
    /// la persona sin ingreso activo en ningún sitio y el gafete libre en el
    /// otro dispositivo; si no se puede verificar, no se registra. Las
    /// decide `application::registrar_ingreso_verificado`, la misma que usa
    /// escritorio. Sin vincular = nube sin configurar.
    pub fn registrar_ingreso_verificado(
        &self,
        contratista_id: i64,
        medio: MedioIngreso,
        gafete: Option<i64>,
        placa: Option<String>,
    ) -> Result<ResultadoRegistroEntrada, NucleoError> {
        use control_acceso::application::IngresoVerificadoError;

        let actor = self.actor_autenticado()?;
        let nube = &self.cache_token;
        control_acceso::application::registrar_ingreso_verificado(
            || self.core_lock(),
            nube,
            &actor,
            contratista_id,
            medio.into(),
            gafete,
            placa,
        )
        .map(Into::into)
        .map_err(|error| match error {
            IngresoVerificadoError::Servicio(error) => error.into(),
            IngresoVerificadoError::Bloqueado(bloqueo) => NucleoError::Rechazado {
                mensaje: control_acceso::mensajes::mensaje_bloqueo_ingreso(&bloqueo),
            },
            IngresoVerificadoError::GafeteOcupadoEnSitio { numero } => {
                NucleoError::GafeteOcupadoEnSitio { numero }
            }
        })
    }

    /// Búsqueda en vivo (la vía primaria del guardia — ver
    /// docs/plan-app-movil.md, "Prioridad de esfuerzo: el buscador"). Un
    /// `texto` vacío trae la primera página completa, no una lista vacía.
    ///
    /// A diferencia del desktop (Tauri/AG Grid), que carga el universo
    /// completo de contratistas al cliente y filtra ahí, el teléfono no
    /// tiene esos recursos de sobra — se pide una página acotada
    /// (`LIMITE_MOVIL`, más chica que la paginación normal de 100 que usa
    /// TUI/CLI) filtrada ya en SQL, nunca la lista entera.
    pub fn buscar_contratistas(
        &self,
        texto: String,
    ) -> Result<Vec<ContratistaResumen>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let texto_normalizado = texto.trim();
        let filtro = FiltroContratistasNucleo {
            texto: (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string()),
            limite: LIMITE_MOVIL,
            ..Default::default()
        };
        let pagina = self
            .core_lock()
            .buscar_contratistas(&filtro)
            .map_err(|origen| NucleoError::Interno {
                mensaje: interno(origen),
            })?;
        Ok(pagina.items.into_iter().map(Into::into).collect())
    }

    /// Mismo criterio tacaño que `buscar_contratistas`: página acotada, no
    /// el listado completo que carga AG Grid en desktop.
    ///
    /// `modo` decide cómo se interpreta `texto` — separado a propósito de
    /// `NombreCedula`: la búsqueda de texto libre de Rust ya hace `OR` entre
    /// cédula/nombre (`LIKE`) y gafete exacto en la misma consulta, así que
    /// buscar "7" como gafete también trae cualquier cédula que *contenga*
    /// un 7 — ruidoso con muchos activos a la vez. En modo `Gafete` se
    /// filtra sólo por `gafete_numero` exacto, sin ese ruido.
    pub fn listar_ingresos_activos(
        &self,
        texto: String,
        modo: ModoBusquedaActivos,
    ) -> Result<Vec<IngresoActivoResumen>, NucleoError> {
        const LIMITE_MOVIL: usize = 30;

        let texto_normalizado = texto.trim();
        let mut filtro = FiltroIngresosActivosNucleo {
            limite: LIMITE_MOVIL,
            ..Default::default()
        };
        match modo {
            ModoBusquedaActivos::NombreCedula => {
                filtro.texto =
                    (!texto_normalizado.is_empty()).then(|| texto_normalizado.to_string());
            }
            ModoBusquedaActivos::Gafete => match texto_normalizado.parse::<i64>() {
                Ok(numero) => filtro.gafete_numero = Some(Igualdad::Incluye(numero)),
                Err(_) if texto_normalizado.is_empty() => {}
                // Texto no numérico en modo gafete: no hay coincidencia
                // posible, no es un error del usuario.
                Err(_) => return Ok(Vec::new()),
            },
        }
        let lista = self
            .core_lock()
            .listar_ingresos_activos(&filtro)
            .map_err(|origen| NucleoError::Interno {
                mensaje: interno(origen),
            })?;
        Ok(lista.items.into_iter().map(Into::into).collect())
    }

    pub fn registrar_salida(&self, registro_id: i64) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().registrar_salida(&actor, registro_id)?)
    }
}
