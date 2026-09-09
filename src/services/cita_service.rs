//! Orquestación de check-in de visitas (`docs/plan-control-visitas.md`):
//! trae las citas de una cédula (`CitaRepository`, sin filtrar) y aplica
//! la regla de vigencia (`domain::cita::verificar_cita`) hasta encontrar
//! la que aplica hoy -- mismo reparto de responsabilidades que
//! `GafeteService`/`domain::gafete` (repositorio trae datos crudos,
//! dominio decide, servicio orquesta).

use chrono::{DateTime, NaiveDate, Utc};

use crate::database::repositories::cita_repository::CitaRepository;
use crate::database::repositories::movimiento_visita_repository::MovimientoVisitaRepository;
use crate::domain::cita::{MotivoDenegacionVisita, ResultadoVisita, verificar_cita};
use crate::domain::registro_ingreso::salida_es_cronologicamente_valida;
use crate::models::cita::{Cita, CitaVisitante};
use crate::models::movimiento_visita::NuevoMovimientoVisita;

use super::error::CitaServiceError;

pub struct CitaService<'a, R, M>
where
    R: CitaRepository + ?Sized,
    M: MovimientoVisitaRepository + ?Sized,
{
    citas: &'a R,
    movimientos: &'a M,
}

impl<'a, R, M> CitaService<'a, R, M>
where
    R: CitaRepository + ?Sized,
    M: MovimientoVisitaRepository + ?Sized,
{
    pub fn new(citas: &'a R, movimientos: &'a M) -> Self {
        Self { citas, movimientos }
    }

    /// Recorre TODAS las citas de `cedula` (puede tener más de una a lo
    /// largo del tiempo) y devuelve la primera que `domain::cita::verificar_cita`
    /// permite hoy. Si ninguna aplica, el error lleva el motivo de la
    /// última candidata evaluada -- con más de una cita denegada
    /// simultáneamente (caso raro: dos agendas que se solapan o se
    /// vencieron juntas) no hay un criterio de prioridad entre motivos
    /// todavía, se informa el de la última que se miró.
    ///
    /// Sólo recorta espacios -- `CitaRepository::buscar_por_cedula` compara
    /// por igualdad exacta (`v.cedula = ?1`, sin `UPPER`/`TRIM` de guiones
    /// en SQL), y la RPC `crear_cita_anfitrion` que guarda la cédula del
    /// lado de la web (`docs/contrato-web-visitas.md`) tampoco cambia
    /// mayúsculas ni quita guiones, sólo hace `btrim` -- si un lado
    /// normalizara distinto del otro, una cédula agendada dejaría de
    /// encontrarse acá aunque el guardia la escribiera/escaneara igual.
    pub fn verificar_check_in(
        &self,
        cedula: &str,
        hoy: NaiveDate,
    ) -> Result<(Cita, CitaVisitante), CitaServiceError> {
        let cedula = cedula.trim();
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

    /// Ejecuta la decisión definitiva usando los repositorios recibidos --
    /// mismo criterio que `RegistroIngresoService::registrar_entrada`: no
    /// confía en un `verificar_check_in` previo (ej. el que ya corrió la
    /// pantalla para mostrarle la cita al guardia) como si fuera una
    /// autorización cacheada -- la vuelve a correr acá mismo, justo antes
    /// de persistir, para no dejar una ventana donde la cita se cancele
    /// entre que se mostró en pantalla y que el guardia confirma.
    ///
    /// A diferencia de `RegistroIngresoService::registrar_entrada`, todavía
    /// NO valida `gafete_numero` contra el catálogo (`gafetes`) -- esa tabla
    /// hoy sólo modela el pool de contratistas (verde); falta la migración
    /// que le suma `tipo` para poder distinguir el pool de visitas (rojo,
    /// ver `docs/plan-control-visitas.md`). Sólo se valida que ese número no
    /// esté YA asignado a otro movimiento de visita abierto -- el `CHECK`
    /// del esquema (`idx_movimientos_visita_gafete_activo`) lo garantiza de
    /// todos modos, esto sólo adelanta el mensaje de error.
    pub fn registrar_entrada(
        &self,
        cedula: &str,
        gafete_numero: Option<i64>,
        usuario_entrada_id: i64,
        fecha_hora_entrada: DateTime<Utc>,
        hoy: NaiveDate,
    ) -> Result<i64, CitaServiceError> {
        let (cita, visitante) = self.verificar_check_in(cedula, hoy)?;

        if self
            .movimientos
            .buscar_activo_por_visitante(visitante.id)?
            .is_some()
        {
            return Err(CitaServiceError::VisitanteYaEnSitio);
        }

        if let Some(numero) = gafete_numero
            && self.movimientos.buscar_activo_por_gafete(numero)?.is_some()
        {
            return Err(CitaServiceError::GafeteOcupado);
        }

        Ok(self.movimientos.crear(&NuevoMovimientoVisita {
            cita_visitante_id: visitante.id,
            gafete_numero,
            fecha_hora_entrada,
            usuario_entrada_id,
            visitante_cedula: visitante.cedula,
            visitante_nombre: visitante.nombre,
            empresa: visitante.empresa,
            anfitrion_nombre: cita.anfitrion_nombre,
            motivo: cita.motivo,
        })?)
    }

    pub fn registrar_salida(
        &self,
        movimiento_id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), CitaServiceError> {
        let movimiento = self
            .movimientos
            .buscar_por_id(movimiento_id)?
            .ok_or(CitaServiceError::MovimientoNoActivo)?;

        if movimiento.salida.is_some() {
            return Err(CitaServiceError::MovimientoNoActivo);
        }

        if !salida_es_cronologicamente_valida(movimiento.fecha_hora_entrada, fecha_hora_salida) {
            return Err(CitaServiceError::SalidaAnteriorAEntrada);
        }

        Ok(self.movimientos.registrar_salida(
            movimiento_id,
            fecha_hora_salida,
            usuario_salida_id,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::cita_repository::SqliteCitaRepository;
    use crate::database::repositories::movimiento_visita_repository::SqliteMovimientoVisitaRepository;
    use crate::database::schema::initialize_database;
    use rusqlite::{Connection, params};

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
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
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

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
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

        let (cita, visitante) = servicio
            .verificar_check_in("1-2345", fecha("2026-08-12"))
            .unwrap();

        assert_eq!(cita.id, 1);
        assert_eq!(visitante.cedula, "1-2345");
    }

    #[test]
    fn verificar_check_in_recorta_espacios_pero_no_cambia_nada_mas() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

        let (_, visitante) = servicio
            .verificar_check_in("  1-2345  ", fecha("2026-08-12"))
            .unwrap();

        assert_eq!(visitante.cedula, "1-2345");
    }

    #[test]
    fn cita_cancelada_devuelve_sin_cita_vigente_con_el_motivo() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "CANCELADA");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

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
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

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
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

        let (cita, _) = servicio
            .verificar_check_in("1-2345", fecha("2026-08-12"))
            .unwrap();

        assert_eq!(cita.id, 2);
    }

    #[test]
    fn registrar_entrada_con_cita_vigente_crea_el_movimiento() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

        let id = servicio
            .registrar_entrada("1-2345", Some(7), 1, Utc::now(), fecha("2026-08-12"))
            .unwrap();

        let movimiento = movimientos.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(movimiento.gafete_numero, Some(7));
        assert!(movimiento.salida.is_none());
    }

    #[test]
    fn registrar_entrada_guarda_el_snapshot_de_la_cita_y_el_visitante_reales() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

        let id = servicio
            .registrar_entrada("1-2345", None, 1, Utc::now(), fecha("2026-08-12"))
            .unwrap();

        let (cedula, nombre, anfitrion): (String, String, String) = connection
            .query_row(
                "SELECT visitante_cedula, visitante_nombre, anfitrion_nombre
                 FROM movimientos_visita WHERE id = ?1",
                rusqlite::params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(cedula, "1-2345");
        assert_eq!(nombre, "Visitante");
        assert_eq!(anfitrion, "Anfitrión");
    }

    #[test]
    fn registrar_entrada_sin_cita_vigente_no_crea_nada() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "CANCELADA");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);

        assert!(matches!(
            servicio.registrar_entrada("1-2345", None, 1, Utc::now(), fecha("2026-08-12")),
            Err(CitaServiceError::SinCitaVigente(
                MotivoDenegacionVisita::CitaCancelada
            ))
        ));
    }

    #[test]
    fn registrar_entrada_dos_veces_seguidas_falla_la_segunda() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);
        servicio
            .registrar_entrada("1-2345", None, 1, Utc::now(), fecha("2026-08-12"))
            .unwrap();

        assert!(matches!(
            servicio.registrar_entrada("1-2345", None, 1, Utc::now(), fecha("2026-08-12")),
            Err(CitaServiceError::VisitanteYaEnSitio)
        ));
    }

    #[test]
    fn registrar_entrada_con_gafete_ya_asignado_a_otra_visita_falla() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        insertar_visitante(&connection, 2, 1, "6-7890");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);
        servicio
            .registrar_entrada("1-2345", Some(5), 1, Utc::now(), fecha("2026-08-12"))
            .unwrap();

        assert!(matches!(
            servicio.registrar_entrada("6-7890", Some(5), 1, Utc::now(), fecha("2026-08-12")),
            Err(CitaServiceError::GafeteOcupado)
        ));
    }

    #[test]
    fn registrar_salida_cierra_el_movimiento() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);
        let entrada = Utc::now();
        let id = servicio
            .registrar_entrada("1-2345", None, 1, entrada, fecha("2026-08-12"))
            .unwrap();

        servicio.registrar_salida(id, entrada, 1).unwrap();

        let movimiento = movimientos.buscar_por_id(id).unwrap().unwrap();
        assert!(movimiento.salida.is_some());
    }

    #[test]
    fn registrar_salida_anterior_a_la_entrada_falla() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);
        let entrada = Utc::now();
        let id = servicio
            .registrar_entrada("1-2345", None, 1, entrada, fecha("2026-08-12"))
            .unwrap();

        assert!(matches!(
            servicio.registrar_salida(id, entrada - chrono::Duration::hours(1), 1),
            Err(CitaServiceError::SalidaAnteriorAEntrada)
        ));
    }

    #[test]
    fn registrar_salida_dos_veces_falla_la_segunda() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&connection);
        let servicio = CitaService::new(&repo, &movimientos);
        let entrada = Utc::now();
        let id = servicio
            .registrar_entrada("1-2345", None, 1, entrada, fecha("2026-08-12"))
            .unwrap();
        servicio.registrar_salida(id, entrada, 1).unwrap();

        assert!(matches!(
            servicio.registrar_salida(id, entrada, 1),
            Err(CitaServiceError::MovimientoNoActivo)
        ));
    }
}
