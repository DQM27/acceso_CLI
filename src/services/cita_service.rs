//! Orquestación de check-in de visitas (`docs/plan-control-visitas.md`):
//! trae las citas de una cédula (`CitaRepository`, sin filtrar) y aplica
//! la regla de vigencia (`domain::cita::verificar_cita`) hasta encontrar
//! la que aplica hoy -- mismo reparto de responsabilidades que
//! `GafeteService`/`domain::gafete` (repositorio trae datos crudos,
//! dominio decide, servicio orquesta).

use chrono::NaiveDate;

use crate::database::repositories::cita_repository::CitaRepository;
use crate::domain::cita::{MotivoDenegacionVisita, ResultadoVisita, verificar_cita};
use crate::models::cita::{Cita, CitaVisitante};

use super::error::CitaServiceError;

pub struct CitaService<'a, R: CitaRepository + ?Sized> {
    citas: &'a R,
}

impl<'a, R: CitaRepository + ?Sized> CitaService<'a, R> {
    pub fn new(citas: &'a R) -> Self {
        Self { citas }
    }

    /// Recorre TODAS las citas de `cedula` (puede tener más de una a lo
    /// largo del tiempo) y devuelve la primera que `domain::cita::verificar_cita`
    /// permite hoy. Si ninguna aplica, el error lleva el motivo de la
    /// última candidata evaluada -- con más de una cita denegada
    /// simultáneamente (caso raro: dos agendas que se solapan o se
    /// vencieron juntas) no hay un criterio de prioridad entre motivos
    /// todavía, se informa el de la última que se miró.
    pub fn verificar_check_in(
        &self,
        cedula: &str,
        hoy: NaiveDate,
    ) -> Result<(Cita, CitaVisitante), CitaServiceError> {
        let candidatas = self.citas.buscar_por_cedula(cedula)?;
        if candidatas.is_empty() {
            return Err(CitaServiceError::SinCitaRegistrada);
        }

        let mut ultimo_motivo = None;
        for (cita, visitante) in candidatas {
            match verificar_cita(&cita, hoy) {
                ResultadoVisita::Permitido => return Ok((cita, visitante)),
                ResultadoVisita::Denegado(motivo) => ultimo_motivo = Some(motivo),
            }
        }

        // `candidatas` no estaba vacío y ningún brazo devolvió `Permitido`
        // antes -- el bucle siempre pasó por la rama `Denegado` al menos
        // una vez, así que este `unwrap_or` nunca debería usar su
        // respaldo. Se prefiere un valor por defecto inofensivo a un
        // `.expect()` que pudiera entrar en pánico si esta garantía
        // alguna vez deja de sostenerse.
        Err(CitaServiceError::SinCitaVigente(
            ultimo_motivo.unwrap_or(MotivoDenegacionVisita::FueraDeVigencia),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::cita_repository::SqliteCitaRepository;
    use crate::database::schema::initialize_database;
    use rusqlite::{Connection, params};

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    fn insertar_cita(connection: &Connection, id: i64, desde: &str, hasta: &str, estado: &str) {
        connection
            .execute(
                "INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                    anfitrion_correo, estado, creado_en)
                 VALUES (?1, ?2, ?3, ?4, 'Anfitrión', 'anfitrion@ejemplo.com', ?5,
                    '2026-08-01T00:00:00Z')",
                params![id, format!("uuid-cita-{id}"), desde, hasta, estado],
            )
            .unwrap();
    }

    fn insertar_visitante(connection: &Connection, id: i64, cita_id: i64, cedula: &str) {
        connection
            .execute(
                "INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
                 VALUES (?1, ?2, ?3, ?4, 'Visitante')",
                params![id, format!("uuid-visitante-{id}"), cita_id, cedula],
            )
            .unwrap();
    }

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    #[test]
    fn cedula_sin_ninguna_cita_devuelve_sin_cita_registrada() {
        let connection = conexion();
        let repo = SqliteCitaRepository::new(&connection);
        let servicio = CitaService::new(&repo);

        assert!(matches!(
            servicio.verificar_check_in("1-2345", fecha("2026-08-10")),
            Err(CitaServiceError::SinCitaRegistrada)
        ));
    }

    #[test]
    fn cita_vigente_permite_el_check_in() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let servicio = CitaService::new(&repo);

        let (cita, visitante) = servicio
            .verificar_check_in("1-2345", fecha("2026-08-12"))
            .unwrap();

        assert_eq!(cita.id, 1);
        assert_eq!(visitante.cedula, "1-2345");
    }

    #[test]
    fn cita_cancelada_devuelve_sin_cita_vigente_con_el_motivo() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "CANCELADA");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let servicio = CitaService::new(&repo);

        let error = servicio
            .verificar_check_in("1-2345", fecha("2026-08-12"))
            .unwrap_err();

        assert!(matches!(
            error,
            CitaServiceError::SinCitaVigente(MotivoDenegacionVisita::CitaCancelada)
        ));
    }

    #[test]
    fn cita_vencida_devuelve_sin_cita_vigente_con_el_motivo() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-01-10", "2026-01-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let servicio = CitaService::new(&repo);

        let error = servicio
            .verificar_check_in("1-2345", fecha("2026-08-12"))
            .unwrap_err();

        assert!(matches!(
            error,
            CitaServiceError::SinCitaVigente(MotivoDenegacionVisita::FueraDeVigencia)
        ));
    }

    #[test]
    fn con_varias_citas_encuentra_la_vigente_aunque_no_sea_la_primera() {
        let connection = conexion();
        // Vencida el año pasado.
        insertar_cita(&connection, 1, "2025-01-10", "2025-01-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        // La de hoy.
        insertar_cita(&connection, 2, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 2, 2, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let servicio = CitaService::new(&repo);

        let (cita, _) = servicio
            .verificar_check_in("1-2345", fecha("2026-08-12"))
            .unwrap();

        assert_eq!(cita.id, 2);
    }
}
