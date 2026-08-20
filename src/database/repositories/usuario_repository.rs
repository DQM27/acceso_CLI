use rusqlite::{Connection, Row, Transaction, TransactionBehavior, params};

use crate::database::error::DatabaseError;
use crate::models::usuario::{RolUsuario, Usuario};

pub trait UsuarioRepository {
    fn crear(&self, usuario: &Usuario) -> Result<i64, DatabaseError>;

    fn crear_root_inicial_atomico(&self, usuario: &Usuario) -> Result<i64, DatabaseError>;

    fn buscar_por_cedula(&self, cedula: &str) -> Result<Option<Usuario>, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Usuario>, DatabaseError>;

    fn actualizar(&self, usuario: &Usuario) -> Result<(), DatabaseError>;

    fn actualizar_protegiendo_ultimo_root(&self, usuario: &Usuario) -> Result<(), DatabaseError>;

    fn establecer_activo(&self, id: i64, activo: bool) -> Result<(), DatabaseError>;

    fn actualizar_password(&self, id: i64, password_hash: &str) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<Usuario>, DatabaseError>;

    fn contar_usuarios(&self) -> Result<i64, DatabaseError>;

    fn contar_roots_activos(&self) -> Result<i64, DatabaseError>;
}

pub struct SqliteUsuarioRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteUsuarioRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Usuario> {
    let rol_texto: String = row.get(4)?;

    let rol = match rol_texto.as_str() {
        "ROOT" => RolUsuario::Root,
        "ADMINISTRADOR" => RolUsuario::Administrador,
        "OPERADOR" => RolUsuario::Operador,

        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                4,
                "rol".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
    };

    Ok(Usuario {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        password_hash: row.get(3)?,
        rol,
        activo: row.get::<_, i64>(5)? != 0,
    })
}

fn rol_a_texto(rol: RolUsuario) -> &'static str {
    match rol {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}

fn insertar_usuario(connection: &Connection, usuario: &Usuario) -> Result<i64, DatabaseError> {
    connection.execute(
        "
        INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params![
            usuario.cedula,
            usuario.nombre,
            usuario.password_hash,
            rol_a_texto(usuario.rol),
            usuario.activo as i64,
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

fn buscar_usuario_en_transaccion(
    transaction: &Transaction<'_>,
    id: i64,
) -> Result<Usuario, DatabaseError> {
    let resultado = transaction.query_row(
        "
        SELECT id, cedula, nombre, password_hash, rol, activo
        FROM usuarios
        WHERE id = ?1
        ",
        params![id],
        convertir_fila,
    );

    match resultado {
        Ok(usuario) => Ok(usuario),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(DatabaseError::UsuarioNoEncontrado),
        Err(error) => Err(DatabaseError::from(error)),
    }
}

fn actualizacion_reduce_roots(actual: &Usuario, nuevo: &Usuario) -> bool {
    actual.rol == RolUsuario::Root
        && actual.activo
        && (nuevo.rol != RolUsuario::Root || !nuevo.activo)
}

/// Nunca toca `password_hash` a propósito, aunque `usuario.password_hash` lo
/// traiga en el struct — este es el camino de edición administrativa
/// (identidad/rol/estado), no el de cambio de contraseña (`actualizar_password`,
/// la única función que sí escribe esa columna). Antes escribía el hash tal
/// cual venía en `usuario`, que en `actualizar_protegiendo_ultimo_root` es un
/// valor leído por el *servicio* **antes** de esta transacción: si otra
/// instancia cambiaba la contraseña justo en ese intervalo, esta función
/// terminaba restaurando el hash viejo sin que nadie lo pidiera. Excluir la
/// columna del `UPDATE` hace la condición de carrera imposible en vez de
/// improbable — no depende de que nadie vuelva a pasar un struct stale.
fn persistir_usuario(
    transaction: &Transaction<'_>,
    usuario: &Usuario,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "UPDATE usuarios
         SET cedula = ?1, nombre = ?2, rol = ?3, activo = ?4
         WHERE id = ?5",
        params![
            usuario.cedula,
            usuario.nombre,
            rol_a_texto(usuario.rol),
            usuario.activo as i64,
            usuario.id,
        ],
    )?;
    Ok(())
}

/// La regla del "último ROOT activo" vive aquí, en el repositorio, y no en
/// `UsuarioService` como el resto de invariantes — a propósito. Igual que
/// `crear_root_inicial_atomico`, la lectura de cuántos ROOT activos hay y la
/// escritura comparten la misma transacción `Immediate`: si se partiera en
/// "validar en el servicio, luego escribir", dos conexiones podrían leer
/// "2 ROOT activos" cada una antes de que la otra escriba, y ambas
/// desactivar/degradar el suyo, dejando cero ROOT activos. La prueba
/// `dos_conexiones_no_pueden_desactivar_ambos_roots` existe justamente para
/// esto — moverlo al servicio reabriría esa carrera.
fn validar_y_persistir(
    transaction: &Transaction<'_>,
    actual: &Usuario,
    nuevo: &Usuario,
) -> Result<(), DatabaseError> {
    if actualizacion_reduce_roots(actual, nuevo) {
        let roots_activos: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM usuarios WHERE rol = 'ROOT' AND activo = 1",
            [],
            |row| row.get(0),
        )?;
        if roots_activos <= 1 {
            return Err(DatabaseError::UltimoRootActivo);
        }
    }
    persistir_usuario(transaction, nuevo)
}

impl<'a> UsuarioRepository for SqliteUsuarioRepository<'a> {
    fn crear(&self, usuario: &Usuario) -> Result<i64, DatabaseError> {
        insertar_usuario(self.connection, usuario)
    }

    fn crear_root_inicial_atomico(&self, usuario: &Usuario) -> Result<i64, DatabaseError> {
        // IMMEDIATE reserva el turno de escritura antes de leer la condición.
        let transaction =
            Transaction::new_unchecked(self.connection, TransactionBehavior::Immediate)?;
        let cantidad: i64 =
            transaction.query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))?;
        if cantidad != 0 {
            return Err(DatabaseError::ConfiguracionInicialYaRealizada);
        }

        let id = insertar_usuario(&transaction, usuario)?;
        transaction.commit()?;
        Ok(id)
    }

    fn buscar_por_cedula(&self, cedula: &str) -> Result<Option<Usuario>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            FROM usuarios
            WHERE cedula = ?1
            ",
        )?;

        match statement.query_row(params![cedula], convertir_fila) {
            Ok(usuario) => Ok(Some(usuario)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<Usuario>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            FROM usuarios
            WHERE id = ?1
            ",
        )?;

        match statement.query_row(params![id], convertir_fila) {
            Ok(usuario) => Ok(Some(usuario)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn actualizar(&self, usuario: &Usuario) -> Result<(), DatabaseError> {
        self.actualizar_protegiendo_ultimo_root(usuario)
    }

    fn actualizar_protegiendo_ultimo_root(&self, usuario: &Usuario) -> Result<(), DatabaseError> {
        // La lectura de la transición y su escritura comparten la misma reserva de escritor.
        let transaction =
            Transaction::new_unchecked(self.connection, TransactionBehavior::Immediate)?;
        let actual = buscar_usuario_en_transaccion(&transaction, usuario.id)?;

        validar_y_persistir(&transaction, &actual, usuario)?;
        transaction.commit()?;
        Ok(())
    }

    fn establecer_activo(&self, id: i64, activo: bool) -> Result<(), DatabaseError> {
        let transaction =
            Transaction::new_unchecked(self.connection, TransactionBehavior::Immediate)?;
        let actual = buscar_usuario_en_transaccion(&transaction, id)?;
        let mut nuevo = actual.clone();
        nuevo.activo = activo;
        validar_y_persistir(&transaction, &actual, &nuevo)?;
        transaction.commit()?;
        Ok(())
    }

    fn actualizar_password(&self, id: i64, password_hash: &str) -> Result<(), DatabaseError> {
        let filas = self.connection.execute(
            "UPDATE usuarios SET password_hash = ?1 WHERE id = ?2",
            params![password_hash, id],
        )?;
        if filas == 0 {
            return Err(DatabaseError::UsuarioNoEncontrado);
        }
        Ok(())
    }

    fn listar(&self) -> Result<Vec<Usuario>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            FROM usuarios
            ORDER BY nombre
            ",
        )?;

        let usuarios = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(usuarios)
    }

    fn contar_usuarios(&self) -> Result<i64, DatabaseError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))?)
    }

    fn contar_roots_activos(&self) -> Result<i64, DatabaseError> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM usuarios WHERE rol = 'ROOT' AND activo = 1",
            [],
            |row| row.get(0),
        )?)
    }
}
