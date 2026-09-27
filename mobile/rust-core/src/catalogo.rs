//! Empresas, alta de contratistas y reglas del formulario de alta.

use control_acceso::services::contratista_service::DatosContratista as DatosContratistaNucleo;

use crate::{DatosContratista, Empresa, Nucleo, NucleoError, TipoIngreso};

#[uniffi::export]
impl Nucleo {
    /// Es el selector del wizard de "Nuevo contratista" -- una empresa
    /// desactivada no es una opción válida ahí, ver
    /// `EmpresaService::listar_seleccionables`. No hay pantalla de
    /// administración de empresas en mobile, así que no hace falta exponer
    /// también la variante sin filtro.
    pub fn listar_empresas(&self) -> Result<Vec<Empresa>, NucleoError> {
        Ok(self
            .core_lock()
            .listar_empresas_seleccionables()?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Alta de contratista en persona, sólo creación (ver
    /// docs/plan-app-movil.md). Todas las reglas (campos obligatorios,
    /// PRAIND requerido y vigente, personal de ruta según el tipo, acceso
    /// habilitado) viven en `ContratistaService::crear`; esto
    /// sólo convierte tipos en la frontera uniffi.
    pub fn crear_contratista(&self, datos: DatosContratista) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;

        let fecha_vencimiento_praind = datos
            .fecha_vencimiento_praind
            .map(|texto| {
                texto
                    .parse()
                    .map_err(|_| NucleoError::FechaInvalida { mensaje: texto })
            })
            .transpose()?;

        Ok(self.core_lock().crear_contratista(
            &actor,
            DatosContratistaNucleo {
                cedula: datos.cedula,
                nombre: datos.nombre,
                empresa_id: datos.empresa_id,
                tipo_ingreso: datos.tipo_ingreso.into(),
                fecha_vencimiento_praind,
                es_personal_ruta: datos.es_personal_ruta,
                // Lo fija el núcleo: el alta siempre queda con acceso.
                tiene_acceso: true,
            },
        )?)
    }

    /// MV-10 (auditoría 2026-09-24): antes de este método,
    /// `PantallaNuevoContratista.kt` reimplementaba esta regla a mano en
    /// Kotlin (necesita evaluarla ANTES de tener un `Contratista` real que
    /// consultar -- el formulario todavía no se guardó) -- riesgo real de
    /// quedar desincronizada si la regla cambia acá y nadie se acuerda de
    /// tocar también el formulario móvil. No usa ningún estado de `self` --
    /// vive como método de `Nucleo` (no función libre) porque es el único
    /// patrón de export que usa este puente hoy (ver el resto de este
    /// `impl`). Fuente de verdad real: `control_acceso::domain::contratista`.
    pub fn requiere_praind_para_formulario(
        &self,
        tipo_ingreso: TipoIngreso,
        personal_ruta: bool,
    ) -> bool {
        control_acceso::domain::contratista::requiere_praind_de(tipo_ingreso.into(), personal_ruta)
    }

    /// Espejo de [`Self::requiere_praind_para_formulario`] para la otra
    /// regla del mismo formulario.
    pub fn requiere_gafete_para_formulario(
        &self,
        tipo_ingreso: TipoIngreso,
        personal_ruta: bool,
    ) -> bool {
        control_acceso::domain::contratista::requiere_gafete_de(tipo_ingreso.into(), personal_ruta)
    }

    /// Si el formulario muestra la casilla "personal de ruta" para este
    /// tipo -- `domain::contratista::admite_personal_ruta`, la misma regla
    /// que después aplica `crear_contratista`.
    pub fn admite_personal_ruta_para_formulario(&self, tipo_ingreso: TipoIngreso) -> bool {
        control_acceso::domain::contratista::admite_personal_ruta(tipo_ingreso.into())
    }

    /// Aviso inmediato de PRAIND vencido mientras se tipea la fecha (ISO
    /// `AAAA-MM-DD`). Una fecha incompleta o inválida todavía no es
    /// "vencida" (`false`): eso lo rechaza `crear_contratista` al guardar.
    /// Misma regla y mismo reloj que el alta (`AppCore::praind_vencido`).
    pub fn praind_vencido_para_formulario(&self, fecha_iso: String) -> bool {
        fecha_iso
            .parse()
            .is_ok_and(|fecha| self.core_lock().praind_vencido(fecha))
    }

    pub fn crear_empresa(&self, nombre: String) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().crear_empresa(&actor, &nombre)?)
    }
}
