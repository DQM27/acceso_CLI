//! Entrega/devolución de gafetes provisionales KOF
//! (`docs/features-futuras/plan-gafetes-provisionales-kof.md`). Mucho más
//! simple que `RutaService`: sin PRAIND, sin bloqueo por documento, sin
//! reloj cruzado entre dominios -- pedido explícito del usuario, no hay más
//! verificación que la humana (cotejar la cédula física contra el nombre,
//! algo que este sistema no captura ni valida).

use chrono::{DateTime, Utc};

use crate::database::repositories::encargado_ruta_repository::EncargadoRutaRepository;
use crate::database::repositories::prestamo_gafete_provisional_repository::PrestamoGafeteProvisionalRepository;
use crate::models::prestamo_gafete_provisional::{
    NuevoPrestamoGafeteProvisional, PrestamoGafeteProvisional,
    PrestamoGafeteProvisionalActivoResumen,
};
use crate::services::error::GafeteProvisionalServiceError;

pub struct GafeteProvisionalService<'a, P, E>
where
    P: PrestamoGafeteProvisionalRepository + ?Sized,
    E: EncargadoRutaRepository + ?Sized,
{
    prestamos: &'a P,
    encargados: &'a E,
}

impl<'a, P, E> GafeteProvisionalService<'a, P, E>
where
    P: PrestamoGafeteProvisionalRepository + ?Sized,
    E: EncargadoRutaRepository + ?Sized,
{
    pub fn new(prestamos: &'a P, encargados: &'a E) -> Self {
        Self { prestamos, encargados }
    }

    /// Valida que el encargado exista y esté activo, y que ni él ni el
    /// número de gafete tengan ya un préstamo abierto, antes de crear el
    /// registro -- el `CHECK`/índices únicos del esquema son el resguardo
    /// final, esto es lo que deja mostrar un mensaje claro antes de chocar
    /// contra ellos (mismo criterio que `RutaService::registrar_salida`).
    pub fn entregar(
        &self,
        encargado_id: i64,
        gafete_numero: i64,
        usuario_id: i64,
        ahora: DateTime<Utc>,
    ) -> Result<i64, GafeteProvisionalServiceError> {
        if gafete_numero <= 0 {
            return Err(GafeteProvisionalServiceError::NumeroInvalido);
        }
        let Some(encargado) = self.encargados.buscar_por_id(encargado_id)? else {
            return Err(GafeteProvisionalServiceError::EncargadoNoEncontrado);
        };
        if !encargado.activo {
            return Err(GafeteProvisionalServiceError::EncargadoInactivo);
        }
        if self
            .prestamos
            .buscar_activo_por_encargado(encargado_id)?
            .is_some()
        {
            return Err(GafeteProvisionalServiceError::EncargadoYaTienePrestamoActivo);
        }
        if self
            .prestamos
            .buscar_activo_por_gafete(gafete_numero)?
            .is_some()
        {
            return Err(GafeteProvisionalServiceError::GafeteYaPrestado);
        }

        Ok(self.prestamos.crear(&NuevoPrestamoGafeteProvisional {
            encargado_id,
            encargado_nombre: encargado.nombre,
            encargado_codigo_empleado: encargado.codigo_empleado,
            gafete_numero,
            fecha_hora_entrega: ahora,
            usuario_entrega_id: usuario_id,
        })?)
    }

    pub fn registrar_devolucion(
        &self,
        id: i64,
        ahora: DateTime<Utc>,
        usuario_id: i64,
    ) -> Result<(), GafeteProvisionalServiceError> {
        let prestamo = self
            .prestamos
            .buscar_por_id(id)?
            .ok_or(GafeteProvisionalServiceError::PrestamoNoActivo)?;
        if prestamo.devolucion.is_some() {
            return Err(GafeteProvisionalServiceError::PrestamoNoActivo);
        }

        Ok(self.prestamos.registrar_devolucion(id, ahora, usuario_id)?)
    }

    pub fn listar_activos(
        &self,
    ) -> Result<Vec<PrestamoGafeteProvisionalActivoResumen>, GafeteProvisionalServiceError> {
        Ok(self.prestamos.listar_activos()?)
    }

    pub fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<Option<PrestamoGafeteProvisional>, GafeteProvisionalServiceError> {
        Ok(self.prestamos.buscar_por_id(id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::encargado_ruta_repository::SqliteEncargadoRutaRepository;
    use crate::database::repositories::prestamo_gafete_provisional_repository::SqlitePrestamoGafeteProvisionalRepository;
    use crate::database::schema::initialize_database;
    use chrono::Utc;
    use rusqlite::Connection;

    fn conexion_con_encargado() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                    VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO encargados_ruta (id, codigo_empleado, nombre, activo, uuid)
                    VALUES (1, '5040017', 'Michael Araya Retana', 1, 'uuid-encargado-1');
                 INSERT INTO encargados_ruta (id, codigo_empleado, nombre, activo, uuid)
                    VALUES (2, '77851', 'Ramon Rodriguez', 0, 'uuid-encargado-2');",
            )
            .unwrap();
        (connection, 1)
    }

    #[test]
    fn entregar_y_devolver_redondea_el_viaje() {
        let (connection, encargado_id) = conexion_con_encargado();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);

        let id = servicio
            .entregar(encargado_id, 12, 1, Utc::now())
            .unwrap();
        assert_eq!(servicio.listar_activos().unwrap().len(), 1);

        servicio.registrar_devolucion(id, Utc::now(), 1).unwrap();

        assert!(servicio.listar_activos().unwrap().is_empty());
    }

    #[test]
    fn entregar_a_encargado_inexistente_falla() {
        let (connection, _) = conexion_con_encargado();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);

        let error = servicio.entregar(999, 12, 1, Utc::now()).unwrap_err();

        assert!(matches!(
            error,
            GafeteProvisionalServiceError::EncargadoNoEncontrado
        ));
    }

    #[test]
    fn entregar_a_encargado_inactivo_falla() {
        let (connection, _) = conexion_con_encargado();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);

        let error = servicio.entregar(2, 12, 1, Utc::now()).unwrap_err();

        assert!(matches!(
            error,
            GafeteProvisionalServiceError::EncargadoInactivo
        ));
    }

    #[test]
    fn entregar_dos_veces_al_mismo_encargado_falla() {
        let (connection, encargado_id) = conexion_con_encargado();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);
        servicio
            .entregar(encargado_id, 12, 1, Utc::now())
            .unwrap();

        let error = servicio
            .entregar(encargado_id, 13, 1, Utc::now())
            .unwrap_err();

        assert!(matches!(
            error,
            GafeteProvisionalServiceError::EncargadoYaTienePrestamoActivo
        ));
    }

    #[test]
    fn entregar_el_mismo_numero_de_gafete_a_otra_persona_falla() {
        let (connection, encargado_id) = conexion_con_encargado();
        connection
            .execute(
                "UPDATE encargados_ruta SET activo = 1 WHERE id = 2",
                [],
            )
            .unwrap();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);
        servicio
            .entregar(encargado_id, 12, 1, Utc::now())
            .unwrap();

        let error = servicio.entregar(2, 12, 1, Utc::now()).unwrap_err();

        assert!(matches!(
            error,
            GafeteProvisionalServiceError::GafeteYaPrestado
        ));
    }

    #[test]
    fn devolver_dos_veces_falla_la_segunda() {
        let (connection, encargado_id) = conexion_con_encargado();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);
        let id = servicio
            .entregar(encargado_id, 12, 1, Utc::now())
            .unwrap();
        servicio.registrar_devolucion(id, Utc::now(), 1).unwrap();

        let error = servicio
            .registrar_devolucion(id, Utc::now(), 1)
            .unwrap_err();

        assert!(matches!(
            error,
            GafeteProvisionalServiceError::PrestamoNoActivo
        ));
    }

    #[test]
    fn numero_de_gafete_invalido_falla() {
        let (connection, encargado_id) = conexion_con_encargado();
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&connection);
        let encargados = SqliteEncargadoRutaRepository::new(&connection);
        let servicio = GafeteProvisionalService::new(&prestamos, &encargados);

        let error = servicio
            .entregar(encargado_id, 0, 1, Utc::now())
            .unwrap_err();

        assert!(matches!(
            error,
            GafeteProvisionalServiceError::NumeroInvalido
        ));
    }
}
