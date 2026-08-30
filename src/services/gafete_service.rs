//! Lógica de negocio del catálogo de gafetes (`docs/plan-gafetes.md`):
//! transiciones de estado válidas (Disponible → Perdido → Disponible,
//! Disponible → DeBaja) y alta individual/por rango.

use chrono::{DateTime, Utc};

use crate::database::queries::gafetes::{FiltroGafetes, GafeteResumen, GafetesQuery};
use crate::database::queries::gafetes_incidentes::GafetesIncidentesWriter;
use crate::database::repositories::contratista_repository::ContratistaRepository;
use crate::database::repositories::gafete_repository::GafeteRepository;
use crate::models::gafete::{EstadoGafete, MotivoResolucionGafete};

use super::error::GafeteServiceError;

pub struct GafeteConsultaService<'a, Q: GafetesQuery + ?Sized> {
    query: &'a Q,
}

impl<'a, Q: GafetesQuery + ?Sized> GafeteConsultaService<'a, Q> {
    pub fn new(query: &'a Q) -> Self {
        Self { query }
    }

    pub fn buscar(&self, filtro: &FiltroGafetes) -> Result<Vec<GafeteResumen>, GafeteServiceError> {
        Ok(self.query.buscar(filtro)?)
    }
}

pub struct GafeteService<'a, R, C>
where
    R: GafeteRepository + ?Sized,
    C: ContratistaRepository + ?Sized,
{
    gafetes: &'a R,
    contratistas: &'a C,
}

impl<'a, R, C> GafeteService<'a, R, C>
where
    R: GafeteRepository + ?Sized,
    C: ContratistaRepository + ?Sized,
{
    pub fn new(gafetes: &'a R, contratistas: &'a C) -> Self {
        Self {
            gafetes,
            contratistas,
        }
    }

    fn buscar_por_id(&self, id: i64) -> Result<crate::models::gafete::Gafete, GafeteServiceError> {
        self.gafetes
            .buscar_por_id(id)?
            .ok_or(GafeteServiceError::GafeteNoEncontrado)
    }

    pub fn crear_uno(&self, numero: i64) -> Result<i64, GafeteServiceError> {
        if numero <= 0 {
            return Err(GafeteServiceError::NumeroInvalido);
        }
        self.gafetes.crear(numero).map_err(|error| {
            if error.es_constraint_unique() {
                GafeteServiceError::NumeroDuplicado
            } else {
                GafeteServiceError::Database(error)
            }
        })
    }

    /// Si un número del rango falla (típicamente duplicado), el rango
    /// completo aborta sin alta parcial — no hace falta deshacer nada acá:
    /// el llamador (`AppCore::crear_gafetes_rango`) sólo comitea la
    /// transacción cuando esta función devuelve `Ok`.
    pub fn crear_rango(&self, desde: i64, hasta: i64) -> Result<Vec<i64>, GafeteServiceError> {
        if desde <= 0 || hasta < desde {
            return Err(GafeteServiceError::RangoInvalido);
        }
        (desde..=hasta)
            .map(|numero| self.crear_uno(numero))
            .collect()
    }

    pub fn dar_de_baja(&self, id: i64) -> Result<(), GafeteServiceError> {
        let gafete = self.buscar_por_id(id)?;
        if gafete.estado != EstadoGafete::Disponible {
            return Err(GafeteServiceError::EstadoInvalido);
        }
        Ok(self.gafetes.dar_de_baja(id)?)
    }

    pub fn marcar_perdido<W: GafetesIncidentesWriter + ?Sized>(
        &self,
        incidentes: &W,
        id: i64,
        contratista_id: i64,
        usuario_id: i64,
        ahora: DateTime<Utc>,
    ) -> Result<(), GafeteServiceError> {
        let gafete = self.buscar_por_id(id)?;
        if gafete.estado != EstadoGafete::Disponible {
            return Err(GafeteServiceError::EstadoInvalido);
        }
        if self.contratistas.buscar_por_id(contratista_id)?.is_none() {
            return Err(GafeteServiceError::ContratistaNoEncontrado);
        }
        self.gafetes.marcar_perdido(id, contratista_id)?;
        incidentes.registrar_perdido(id, ahora, usuario_id, contratista_id)?;
        Ok(())
    }

    pub fn resolver<W: GafetesIncidentesWriter + ?Sized>(
        &self,
        incidentes: &W,
        id: i64,
        motivo: MotivoResolucionGafete,
        usuario_id: i64,
        ahora: DateTime<Utc>,
    ) -> Result<(), GafeteServiceError> {
        let gafete = self.buscar_por_id(id)?;
        if gafete.estado != EstadoGafete::Perdido {
            return Err(GafeteServiceError::EstadoInvalido);
        }
        self.gafetes.resolver(id)?;
        incidentes.registrar_resuelto(id, ahora, usuario_id, motivo)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::queries::gafetes_incidentes::SqliteGafetesIncidentes;
    use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
    use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
    use crate::database::schema::initialize_database;
    use rusqlite::Connection;

    fn conexion_con_contratista() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Acme');
                 INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso)
                 VALUES ('1', 'Juan', 1, 'PRAIND', 0, 1);
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
                 VALUES ('9', 'Root', 'hash', 'ROOT', 1);",
            )
            .unwrap();
        (connection, 1)
    }

    #[test]
    fn numero_cero_o_negativo_es_invalido() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let contratistas = SqliteContratistaRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes, &contratistas);

        assert!(matches!(
            servicio.crear_uno(0),
            Err(GafeteServiceError::NumeroInvalido)
        ));
        assert!(matches!(
            servicio.crear_uno(-1),
            Err(GafeteServiceError::NumeroInvalido)
        ));
    }

    #[test]
    fn rango_invalido_no_crea_nada() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let contratistas = SqliteContratistaRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes, &contratistas);

        assert!(matches!(
            servicio.crear_rango(5, 3),
            Err(GafeteServiceError::RangoInvalido)
        ));
    }

    #[test]
    fn rango_con_un_numero_ya_existente_aborta_completo() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let contratistas = SqliteContratistaRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes, &contratistas);
        servicio.crear_uno(3).unwrap();

        let resultado = servicio.crear_rango(1, 5);

        assert!(matches!(
            resultado,
            Err(GafeteServiceError::NumeroDuplicado)
        ));
    }

    #[test]
    fn transiciones_de_estado_siguen_disponible_perdido_disponible() {
        let (connection, contratista_id) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let contratistas = SqliteContratistaRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let servicio = GafeteService::new(&gafetes, &contratistas);
        let id = servicio.crear_uno(1).unwrap();
        let ahora = Utc::now();

        servicio
            .marcar_perdido(&incidentes, id, contratista_id, 1, ahora)
            .unwrap();
        assert!(matches!(
            servicio.marcar_perdido(&incidentes, id, contratista_id, 1, ahora),
            Err(GafeteServiceError::EstadoInvalido)
        ));

        servicio
            .resolver(&incidentes, id, MotivoResolucionGafete::Pagado, 1, ahora)
            .unwrap();
        assert!(matches!(
            servicio.resolver(&incidentes, id, MotivoResolucionGafete::Pagado, 1, ahora),
            Err(GafeteServiceError::EstadoInvalido)
        ));
    }

    #[test]
    fn dar_de_baja_solo_si_disponible() {
        let (connection, contratista_id) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let contratistas = SqliteContratistaRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let servicio = GafeteService::new(&gafetes, &contratistas);
        let id = servicio.crear_uno(1).unwrap();

        servicio
            .marcar_perdido(&incidentes, id, contratista_id, 1, Utc::now())
            .unwrap();
        assert!(matches!(
            servicio.dar_de_baja(id),
            Err(GafeteServiceError::EstadoInvalido)
        ));
    }

    #[test]
    fn marcar_perdido_con_contratista_inexistente_falla() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let contratistas = SqliteContratistaRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let servicio = GafeteService::new(&gafetes, &contratistas);
        let id = servicio.crear_uno(1).unwrap();

        assert!(matches!(
            servicio.marcar_perdido(&incidentes, id, 999, 1, Utc::now()),
            Err(GafeteServiceError::ContratistaNoEncontrado)
        ));
    }
}
