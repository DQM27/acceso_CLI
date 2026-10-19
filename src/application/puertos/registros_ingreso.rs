//! Puerto de los ingresos y salidas de contratistas.

use chrono::{DateTime, Utc};

use crate::database::error::DatabaseError;
use crate::models::registro_ingreso::{NuevoRegistroIngreso, RegistroIngreso};
use crate::models::via_ingreso::ViaIngreso;

pub trait RegistroIngresoRepository {
    fn crear(&self, registro: &NuevoRegistroIngreso) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngreso>, DatabaseError>;

    fn buscar_ingreso_activo(
        &self,
        contratista_id: i64,
    ) -> Result<Option<RegistroIngreso>, DatabaseError>;

    /// ¿El otro dispositivo del sitio tiene abierto un ingreso con esta
    /// cédula? Lee la caché `ingresos_remotos` (la llena la sincronización;
    /// nunca incluye lo que ya vive en `registro_ingresos` de este equipo).
    fn cedula_con_ingreso_abierto_en_otro_dispositivo(
        &self,
        cedula: &str,
    ) -> Result<bool, DatabaseError>;

    /// ¿Esta cédula ya está adentro por OTRA vía (contratista, proveedor o
    /// por correo), en este equipo o en el otro de la unidad? Una persona no
    /// puede estar adentro por dos vías a la vez. Ver
    /// `database::queries::persona_adentro`.
    fn adentro_por_otra_via(&self, cedula: &str) -> Result<Option<ViaIngreso>, DatabaseError>;

    /// Busca quién tiene actualmente asignado un gafete.
    ///
    /// Solo considera ingresos activos, es decir,
    /// registros que todavía no tienen fecha de salida.
    fn buscar_ingreso_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<RegistroIngreso>, DatabaseError>;

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<RegistroIngreso>, DatabaseError>;
}
