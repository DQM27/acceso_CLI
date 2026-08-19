use chrono::{DateTime, Utc};

use super::medio_ingreso::MedioIngreso;
use super::tipo_ingreso::TipoIngreso;

/// Definida junto a las 5 reglas de `verificar_acceso` que versiona, no aquí
/// — ver `domain::acceso::VERSION_REGLAS_ACCESO`. Re-exportada para no
/// romper a quien ya la importa de este camino (struct de persistencia).
pub use crate::domain::acceso::VERSION_REGLAS_ACCESO;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoIngresoRegistrado {
    Permitido,
    /// Carga el motivo en vez de dejar que quien persista lo adivine — hoy
    /// sólo `domain::acceso::verificar_acceso` produce esta variante (regla 4,
    /// PRAIND próximo a vencer), así que quien la construye siempre sabe el
    /// motivo real; nadie tiene que asumirlo después.
    PermitidoConAdvertencia(MotivoResultadoIngreso),
    /// Registro anterior a la captura de fotografías históricas.
    Migrado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoResultadoIngreso {
    PraindProximoVencer,
    DatosReconstruidos,
}

#[derive(Debug, Clone)]
pub struct DatosHistoricosEntrada {
    pub contratista_cedula: String,
    pub contratista_nombre: String,
    pub fecha_vencimiento_praind: Option<chrono::NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
    pub resultado_acceso: ResultadoIngresoRegistrado,
    pub reglas_version: i64,
}

#[derive(Debug, Clone)]
pub struct NuevoRegistroIngreso {
    pub contratista_id: i64,
    pub empresa_id: i64,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub medio_ingreso: MedioIngreso,
    pub tipo_ingreso: TipoIngreso,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_id: i64,
    pub datos_historicos: DatosHistoricosEntrada,
}

/// Fecha y usuario van juntos a propósito, en vez de ser 2 `Option`
/// independientes en `RegistroIngreso` — la base ya exige "ambos o ninguno"
/// con un `CHECK` (ver `MIGRACION_2`/`MIGRACION_5` en `schema.rs`); este tipo
/// hace esa misma regla imposible de romper del lado de Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SalidaRegistroIngreso {
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
}

#[derive(Debug, Clone)]
pub struct RegistroIngreso {
    pub id: i64,

    pub contratista_id: i64,
    pub empresa_id: i64,

    pub fecha_hora_ingreso: DateTime<Utc>,

    pub medio_ingreso: MedioIngreso,
    pub tipo_ingreso: TipoIngreso,

    /// Número de gafete asignado durante este ingreso.
    ///
    /// `Some(numero)` = tiene gafete.
    /// `None` = sin gafete (S/G).
    pub gafete_numero: Option<i64>,

    pub usuario_ingreso_id: i64,

    /// `None` mientras el ingreso sigue activo; `Some` una vez registrada la salida.
    pub salida: Option<SalidaRegistroIngreso>,
}
