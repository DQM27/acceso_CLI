//! Ingreso/salida "por correo" (visita autorizada por correo, comodín previo
//! al módulo de Visitas) -- mismo espíritu que `IngresoProveedorService`:
//! sin catálogo de personas (cédula/nombre son snapshot puro) y gafete
//! siempre obligatorio, pero de VISITA y con un motivo libre ("a quién
//! visita") en lugar de la empresa.

use chrono::{DateTime, Utc};

use crate::database::repositories::gafete_repository::GafeteRepository;
use crate::database::repositories::registro_ingreso_correo_repository::RegistroIngresoCorreoRepository;
use crate::domain::cedula::{Cedula, CedulaInvalida};
use crate::domain::gafete::{ValidacionAsignacion, validar_para_asignar};
use crate::domain::registro_ingreso::salida_es_cronologicamente_valida;
use crate::models::gafete::TipoGafete;
use crate::models::registro_ingreso_correo::{
    NuevoRegistroIngresoCorreo, RegistroIngresoCorreoActivoResumen,
};

use super::error::IngresoCorreoServiceError;

pub struct IngresoCorreoService<'a, R, G>
where
    R: RegistroIngresoCorreoRepository + ?Sized,
    G: GafeteRepository + ?Sized,
{
    registros: &'a R,
    gafetes: &'a G,
}

/// Datos del formulario, tal cual llegan de la pantalla.
#[derive(Debug, Clone)]
pub struct DatosIngresoCorreo<'a> {
    pub cedula: &'a str,
    pub nombre: &'a str,
    pub motivo: &'a str,
    /// Vacía o en blanco = llegó a pie (mismo criterio que proveedores).
    pub placa: Option<String>,
    pub gafete_numero: i64,
}

impl<'a, R, G> IngresoCorreoService<'a, R, G>
where
    R: RegistroIngresoCorreoRepository + ?Sized,
    G: GafeteRepository + ?Sized,
{
    pub fn new(registros: &'a R, gafetes: &'a G) -> Self {
        Self { registros, gafetes }
    }

    /// Valida cédula, nombre y motivo, que la cédula no tenga ya un ingreso
    /// por correo abierto, y que el gafete de visita exista, esté disponible
    /// y no esté en uso en este equipo -- mismo orden que
    /// `IngresoProveedorService::registrar_ingreso`.
    pub fn registrar_ingreso(
        &self,
        datos: &DatosIngresoCorreo<'_>,
        usuario_id: i64,
        ahora: DateTime<Utc>,
    ) -> Result<i64, IngresoCorreoServiceError> {
        let cedula = match Cedula::normalizar(datos.cedula) {
            Ok(cedula) if cedula.es_nacional_o_de_extranjero() => cedula,
            Err(CedulaInvalida::Vacia) => return Err(IngresoCorreoServiceError::CedulaVacia),
            Ok(_) | Err(_) => return Err(IngresoCorreoServiceError::CedulaInvalida),
        };
        let cedula = cedula.as_str();
        // A quien se le negó el acceso como contratista tampoco entra por
        // correo con la misma cédula.
        if self.registros.cedula_con_acceso_negado(cedula)? {
            return Err(IngresoCorreoServiceError::AccesoNegado);
        }
        // Como todo nombre de persona o empresa: en mayúscula.
        let nombre = control_acceso_reglas::nombre::nombre_en_mayusculas(datos.nombre);
        if nombre.is_empty() {
            return Err(IngresoCorreoServiceError::NombreVacio);
        }
        let motivo = datos.motivo.trim();
        if motivo.is_empty() {
            return Err(IngresoCorreoServiceError::MotivoVacio);
        }
        let placa = datos
            .placa
            .as_deref()
            .map(str::trim)
            .filter(|placa| !placa.is_empty())
            .map(str::to_uppercase);

        if self.registros.buscar_ingreso_activo(cedula)?.is_some() {
            return Err(IngresoCorreoServiceError::IngresoActivo);
        }
        // Una persona no puede estar adentro por dos vías: si ya entró como
        // contratista o como proveedor, no entra también por correo.
        if let Some(via) = self.registros.adentro_por_otra_via(cedula)? {
            return Err(IngresoCorreoServiceError::AdentroPorOtraVia(via));
        }

        let gafete = self
            .gafetes
            .buscar_por_numero(datos.gafete_numero, TipoGafete::Visita)?;
        match validar_para_asignar(gafete.as_ref()) {
            ValidacionAsignacion::NoRegistrado => {
                return Err(IngresoCorreoServiceError::GafeteNoRegistrado);
            }
            ValidacionAsignacion::NoDisponible(estado) => {
                return Err(IngresoCorreoServiceError::GafeteNoDisponible(estado));
            }
            ValidacionAsignacion::Asignable => {}
        }
        if self
            .registros
            .gafete_de_visita_en_uso(datos.gafete_numero)?
        {
            return Err(IngresoCorreoServiceError::GafeteOcupado);
        }

        Ok(self.registros.crear(&NuevoRegistroIngresoCorreo {
            cedula: cedula.to_string(),
            nombre,
            motivo: motivo.to_string(),
            placa,
            gafete_numero: datos.gafete_numero,
            fecha_hora_ingreso: ahora,
            usuario_ingreso_id: usuario_id,
        })?)
    }

    pub fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), IngresoCorreoServiceError> {
        let registro = self
            .registros
            .buscar_por_id(id)?
            .ok_or(IngresoCorreoServiceError::RegistroNoActivo)?;
        if registro.salida.is_some() {
            return Err(IngresoCorreoServiceError::RegistroNoActivo);
        }
        if !salida_es_cronologicamente_valida(registro.fecha_hora_ingreso, fecha_hora_salida) {
            return Err(IngresoCorreoServiceError::SalidaAnteriorAIngreso);
        }
        self.registros
            .registrar_salida(id, fecha_hora_salida, usuario_salida_id)
            .map_err(|error| match error {
                crate::database::error::DatabaseError::RegistroCorreoNoActivo => {
                    IngresoCorreoServiceError::RegistroNoActivo
                }
                otro => IngresoCorreoServiceError::Database(otro),
            })
    }

    pub fn listar_activos(
        &self,
    ) -> Result<Vec<RegistroIngresoCorreoActivoResumen>, IngresoCorreoServiceError> {
        Ok(self.registros.listar_activos()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
    use crate::database::repositories::registro_ingreso_correo_repository::SqliteRegistroIngresoCorreoRepository;
    use crate::database::schema::initialize_database;
    use chrono::TimeZone;
    use rusqlite::Connection;

    fn conexion_con_gafetes() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        let gafetes = SqliteGafeteRepository::new(&connection);
        gafetes.crear(3, TipoGafete::Visita).unwrap();
        gafetes.crear(4, TipoGafete::Visita).unwrap();
        gafetes.crear(9, TipoGafete::Proveedor).unwrap();
        connection
    }

    fn ahora() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 3, 14, 0, 0).unwrap()
    }

    fn datos(cedula: &str, gafete: i64) -> DatosIngresoCorreo<'_> {
        DatosIngresoCorreo {
            cedula,
            nombre: " Ana Solano ",
            motivo: " Entrevista RH - Laura ",
            placa: Some("  ".to_string()),
            gafete_numero: gafete,
        }
    }

    #[test]
    fn registra_con_gafete_de_visita_y_limpia_los_textos() {
        let connection = conexion_con_gafetes();
        let registros = SqliteRegistroIngresoCorreoRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoCorreoService::new(&registros, &gafetes);

        let id = servicio
            .registrar_ingreso(&datos("1-1111-1111", 3), 1, ahora())
            .unwrap();
        let registro = registros.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(registro.cedula, "111111111");
        assert_eq!(registro.nombre, "ANA SOLANO");
        assert_eq!(registro.motivo, "Entrevista RH - Laura");
        assert_eq!(registro.placa, None, "placa en blanco = a pie");

        let mut en_vehiculo = datos("222222222", 4);
        en_vehiculo.placa = Some(" abc123 ".to_string());
        let id = servicio
            .registrar_ingreso(&en_vehiculo, 1, ahora())
            .unwrap();
        assert_eq!(
            registros
                .buscar_por_id(id)
                .unwrap()
                .unwrap()
                .placa
                .as_deref(),
            Some("ABC123")
        );
    }

    #[test]
    fn rechaza_datos_incompletos_y_gafetes_que_no_corresponden() {
        let connection = conexion_con_gafetes();
        let registros = SqliteRegistroIngresoCorreoRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoCorreoService::new(&registros, &gafetes);

        let mut sin_motivo = datos("111111111", 3);
        sin_motivo.motivo = "  ";
        assert!(matches!(
            servicio.registrar_ingreso(&sin_motivo, 1, ahora()),
            Err(IngresoCorreoServiceError::MotivoVacio)
        ));
        let mut sin_nombre = datos("111111111", 3);
        sin_nombre.nombre = "";
        assert!(matches!(
            servicio.registrar_ingreso(&sin_nombre, 1, ahora()),
            Err(IngresoCorreoServiceError::NombreVacio)
        ));
        assert!(matches!(
            servicio.registrar_ingreso(&datos("", 3), 1, ahora()),
            Err(IngresoCorreoServiceError::CedulaVacia)
        ));
        // El 9 es de proveedor: no sirve como gafete de visita.
        assert!(matches!(
            servicio.registrar_ingreso(&datos("111111111", 9), 1, ahora()),
            Err(IngresoCorreoServiceError::GafeteNoRegistrado)
        ));

        servicio
            .registrar_ingreso(&datos("111111111", 3), 1, ahora())
            .unwrap();
        assert!(matches!(
            servicio.registrar_ingreso(&datos("111111111", 4), 1, ahora()),
            Err(IngresoCorreoServiceError::IngresoActivo)
        ));
        assert!(matches!(
            servicio.registrar_ingreso(&datos("222222222", 3), 1, ahora()),
            Err(IngresoCorreoServiceError::GafeteOcupado)
        ));
    }

    /// Una persona no puede estar adentro por dos vías: si el otro equipo de
    /// la unidad la tiene adentro como proveedor (caché de remotos), no
    /// entra también por correo.
    #[test]
    fn rechaza_a_quien_ya_esta_adentro_como_proveedor_en_el_otro_equipo() {
        let connection = conexion_con_gafetes();
        connection
            .execute(
                "INSERT INTO ingresos_proveedor_remotos (uuid, sitio_id, cedula, nombre,
                    empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre,
                    dispositivo_entrada_id, actualizado_en)
                 VALUES ('r1', 's1', '111111111', 'Ana Solano', 'ACME', 9,
                    '2026-10-03T13:00:00Z', 'Otro', 'd2', '2026-10-03T13:00:00Z')",
                [],
            )
            .unwrap();
        let registros = SqliteRegistroIngresoCorreoRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoCorreoService::new(&registros, &gafetes);

        assert!(matches!(
            servicio.registrar_ingreso(&datos("1-1111-1111", 3), 1, ahora()),
            Err(IngresoCorreoServiceError::AdentroPorOtraVia(
                crate::models::via_ingreso::ViaIngreso::Proveedor
            ))
        ));
    }

    #[test]
    fn la_salida_libera_el_gafete_y_no_se_repite() {
        let connection = conexion_con_gafetes();
        let registros = SqliteRegistroIngresoCorreoRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoCorreoService::new(&registros, &gafetes);

        let id = servicio
            .registrar_ingreso(&datos("111111111", 3), 1, ahora())
            .unwrap();
        assert!(matches!(
            servicio.registrar_salida(id, ahora() - chrono::Duration::minutes(1), 1),
            Err(IngresoCorreoServiceError::SalidaAnteriorAIngreso)
        ));
        servicio
            .registrar_salida(id, ahora() + chrono::Duration::hours(1), 1)
            .unwrap();
        assert!(matches!(
            servicio.registrar_salida(id, ahora() + chrono::Duration::hours(2), 1),
            Err(IngresoCorreoServiceError::RegistroNoActivo)
        ));
        assert!(servicio.listar_activos().unwrap().is_empty());
        servicio
            .registrar_ingreso(&datos("222222222", 3), 1, ahora())
            .unwrap();
    }
}
