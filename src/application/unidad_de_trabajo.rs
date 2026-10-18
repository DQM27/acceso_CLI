//! La unidad de trabajo de una operación del punto de acceso: una
//! transacción con el reloj ya validado y el operador ya confirmado, dentro
//! de la cual el caso de uso lee, decide y escribe, y que se confirma una
//! sola vez al final.
//!
//! Antes, contratistas, visitas, rutas, correo y proveedores repetían este
//! armazón cada uno con sus propios nombres; lo único que de verdad cambia
//! entre ellos es **qué movimientos cuentan para saber si el reloj
//! retrocedió** y **qué tipo de error devuelven**. Esas dos cosas las
//! aporta quien llama; el resto vive aquí, una sola vez.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::reloj::reloj_retrocedido;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{
    CitaServiceError, IngresoCorreoServiceError, IngresoProveedorServiceError,
    RegistroIngresoServiceError, RutaServiceError,
};

use super::{AppCore, verificar_actor_activo};

/// Lo que la unidad de trabajo necesita de un error de dominio: poder
/// guardar una falla de la base y decir que el operador ya no es válido o
/// que el reloj retrocedió. Qué movimientos cuentan para lo segundo lo
/// decide cada fachada con su `validar_reloj` (contratistas, visitas y
/// rutas miran tablas distintas); correo y proveedores comparten
/// [`validar_reloj_operativo`].
pub(super) trait ErrorDeUnidadDeTrabajo: From<DatabaseError> {
    fn operador_no_autorizado() -> Self;
    fn reloj_retrocedido() -> Self;
}

macro_rules! error_de_unidad_de_trabajo {
    ($($tipo:ty),+ $(,)?) => {
        $(
            impl ErrorDeUnidadDeTrabajo for $tipo {
                fn operador_no_autorizado() -> Self {
                    Self::OperadorNoAutorizado
                }

                fn reloj_retrocedido() -> Self {
                    Self::RelojRetrocedido
                }
            }
        )+
    };
}

error_de_unidad_de_trabajo!(
    RegistroIngresoServiceError,
    CitaServiceError,
    RutaServiceError,
    IngresoCorreoServiceError,
    IngresoProveedorServiceError,
);

/// La validación del reloj de correo y proveedores: cualquier movimiento
/// operativo del equipo cuenta (`queries::reloj`).
pub(super) fn validar_reloj_operativo<E: ErrorDeUnidadDeTrabajo>(
    connection: &Connection,
    ahora: DateTime<Utc>,
) -> Result<(), E> {
    if reloj_retrocedido(connection, ahora)? {
        return Err(E::reloj_retrocedido());
    }
    Ok(())
}

impl AppCore {
    /// Abre una transacción `Immediate` (el bloqueo se adquiere antes de la
    /// primera lectura definitiva, así los repositorios creados sobre ella
    /// validan e insertan contra el mismo estado de `SQLite`), lee el reloj
    /// ya con el bloqueo tomado, corre `validar_reloj` y confirma que
    /// `actor` sigue siendo un operador activo; después corre `operar` y
    /// confirma.
    ///
    /// **El reloj se lee con el bloqueo tomado, no antes**: si se leyera
    /// antes de esperar el bloqueo, otro equipo de la misma base podría
    /// sellar un movimiento en medio y esta operación vería su propia hora
    /// como "retrocedida" sin que el reloj haya fallado.
    ///
    /// La validación del reloj y la del operador se quedan en esta capa, y
    /// no en los servicios (que declaran `RelojRetrocedido` pero no lo
    /// generan) **a propósito**: son comprobaciones de sanidad de todo el
    /// sistema (¿el reloj de la máquina retrocedió respecto al último
    /// movimiento conocido, sin importar de qué persona?), no reglas de
    /// negocio de una entrada o salida puntual. Moverlas al servicio se
    /// intentó y se revirtió: rompía pruebas de integración que llaman al
    /// servicio directo con datos cuyos tiempos no representan un reloj real
    /// avanzando (`tests/flujo_integracion.rs`).
    ///
    /// El operador se revisa dentro de esta misma transacción, no antes,
    /// para que una desactivación concurrente no pueda colarse entre el
    /// chequeo y la escritura (`docs/auditoria-dominio-2026-08-20.md`,
    /// hallazgo #2). La FK de `SQLite` sólo exige que el usuario exista, no
    /// que esté activo.
    pub(super) fn en_unidad_de_trabajo<T, E: ErrorDeUnidadDeTrabajo>(
        &self,
        actor: &UsuarioSesion,
        validar_reloj: impl FnOnce(&Connection, DateTime<Utc>) -> Result<(), E>,
        operar: impl FnOnce(&Transaction<'_>, DateTime<Utc>) -> Result<T, E>,
    ) -> Result<T, E> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let ahora = self.reloj.ahora_utc();
        validar_reloj(&transaction, ahora)?;
        if verificar_actor_activo(&transaction, actor)?.is_none() {
            return Err(E::operador_no_autorizado());
        }
        let resultado = operar(&transaction, ahora)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(resultado)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use chrono::TimeZone;

    use super::*;
    use crate::database::schema::initialize_database;
    use crate::models::usuario::RolUsuario;

    type Error = RegistroIngresoServiceError;

    fn nucleo() -> AppCore {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                    (1,'U1','Activo','hash','OPERADOR',1),
                    (2,'U2','Inactivo','hash','OPERADOR',0);",
            )
            .unwrap();
        AppCore::new(connection)
    }

    fn actor(id: i64) -> UsuarioSesion {
        UsuarioSesion {
            id,
            cedula: format!("U{id}"),
            nombre: "Operador".into(),
            rol: RolUsuario::Operador,
        }
    }

    fn reloj_sin_problema(_: &Connection, _: DateTime<Utc>) -> Result<(), Error> {
        Ok(())
    }

    fn empresas(core: &AppCore) -> i64 {
        core.connection
            .query_row("SELECT COUNT(*) FROM empresas", [], |fila| fila.get(0))
            .unwrap()
    }

    fn insertar_empresa(transaction: &Transaction<'_>) {
        transaction
            .execute("INSERT INTO empresas(nombre) VALUES ('Empresa')", [])
            .unwrap();
    }

    #[test]
    fn confirma_lo_escrito_cuando_operar_sale_bien() {
        let core = nucleo();

        let resultado = core.en_unidad_de_trabajo(&actor(1), reloj_sin_problema, |tx, _| {
            insertar_empresa(tx);
            Ok::<_, Error>(7)
        });

        assert_eq!(resultado.unwrap(), 7);
        assert_eq!(empresas(&core), 1);
    }

    #[test]
    fn si_operar_falla_se_revierte_todo_lo_escrito() {
        let core = nucleo();

        let resultado = core.en_unidad_de_trabajo(&actor(1), reloj_sin_problema, |tx, _| {
            insertar_empresa(tx);
            Err::<(), _>(Error::RegistroNoActivo)
        });

        assert!(matches!(resultado, Err(Error::RegistroNoActivo)));
        assert_eq!(empresas(&core), 0, "sin confirmar no se escribe nada");
    }

    #[test]
    fn despues_de_un_fallo_la_conexion_queda_libre_para_la_siguiente() {
        let core = nucleo();
        let _ = core.en_unidad_de_trabajo(&actor(1), reloj_sin_problema, |_, _| {
            Err::<(), _>(Error::RegistroNoActivo)
        });

        core.en_unidad_de_trabajo(&actor(1), reloj_sin_problema, |tx, _| {
            insertar_empresa(tx);
            Ok::<_, Error>(())
        })
        .unwrap();

        assert_eq!(empresas(&core), 1);
    }

    #[test]
    fn un_operador_inactivo_o_inexistente_no_llega_a_operar() {
        let core = nucleo();
        for id in [2, 999] {
            let operado = Cell::new(false);

            let resultado = core.en_unidad_de_trabajo(&actor(id), reloj_sin_problema, |tx, _| {
                operado.set(true);
                insertar_empresa(tx);
                Ok::<_, Error>(())
            });

            assert!(matches!(resultado, Err(Error::OperadorNoAutorizado)));
            assert!(!operado.get(), "operador {id}: no debe llegar a operar");
        }
        assert_eq!(empresas(&core), 0);
    }

    #[test]
    fn un_reloj_retrocedido_no_llega_a_operar() {
        let core = nucleo();
        let operado = Cell::new(false);

        let resultado = core.en_unidad_de_trabajo(
            &actor(1),
            |_, _| Err(Error::RelojRetrocedido),
            |_, _| {
                operado.set(true);
                Ok::<_, Error>(())
            },
        );

        assert!(matches!(resultado, Err(Error::RelojRetrocedido)));
        assert!(!operado.get());
    }

    #[test]
    fn el_reloj_se_valida_antes_que_el_operador() {
        // Un operador inactivo con el reloj retrocedido responde lo del
        // reloj: es el orden que ya tenían todas las fachadas.
        let core = nucleo();

        let resultado = core.en_unidad_de_trabajo(
            &actor(2),
            |_, _| Err(Error::RelojRetrocedido),
            |_, _| Ok::<_, Error>(()),
        );

        assert!(matches!(resultado, Err(Error::RelojRetrocedido)));
    }

    #[test]
    fn operar_recibe_la_misma_hora_que_vio_la_validacion_del_reloj() {
        let core = nucleo();
        let vista_por_la_validacion = Cell::new(None);

        let recibida = core
            .en_unidad_de_trabajo(
                &actor(1),
                |_, ahora| {
                    vista_por_la_validacion.set(Some(ahora));
                    Ok::<_, Error>(())
                },
                |_, ahora| Ok(ahora),
            )
            .unwrap();

        assert_eq!(Some(recibida), vista_por_la_validacion.get());
    }

    #[test]
    fn validar_reloj_operativo_compara_contra_el_ultimo_movimiento_del_equipo() {
        let core = nucleo();
        core.connection
            .execute_batch(
                "INSERT INTO empresas_proveedor(id,nombre,activo,uuid) VALUES (1,'Maika',1,'e');
                 INSERT INTO registro_ingresos_proveedor(
                    cedula,nombre,empresa_id,empresa_nombre,gafete_numero,fecha_hora_ingreso,
                    usuario_ingreso_id,usuario_ingreso_nombre,uuid
                 ) VALUES ('111111111','ANA',1,'Maika',7,'2026-10-17T14:00:00Z',1,'Activo','r');",
            )
            .unwrap();
        let hora = |hora| Utc.with_ymd_and_hms(2026, 10, 17, hora, 0, 0).unwrap();

        assert!(matches!(
            validar_reloj_operativo::<Error>(&core.connection, hora(13)),
            Err(Error::RelojRetrocedido)
        ));
        assert!(validar_reloj_operativo::<Error>(&core.connection, hora(14)).is_ok());
        assert!(validar_reloj_operativo::<Error>(&core.connection, hora(15)).is_ok());
    }
}
