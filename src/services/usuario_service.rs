use crate::database::error::DatabaseError;
use crate::database::queries::usuarios::{FiltroUsuarios, UsuarioResumen, UsuariosQuery};
use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::{RolUsuario, Usuario};

use super::error::UsuarioServiceError;
use super::password::generar_hash;

const LONGITUD_MINIMA_PASSWORD: usize = 8;

pub struct UsuarioConsultaService<'a, Q>
where
    Q: UsuariosQuery + ?Sized,
{
    consultas: &'a Q,
}

impl<'a, Q> UsuarioConsultaService<'a, Q>
where
    Q: UsuariosQuery + ?Sized,
{
    pub fn new(consultas: &'a Q) -> Self {
        Self { consultas }
    }

    pub fn buscar_para_tabla(
        &self,
        filtro: &FiltroUsuarios,
    ) -> Result<Vec<UsuarioResumen>, UsuarioServiceError> {
        Ok(self.consultas.buscar(filtro)?)
    }
}

pub struct CrearUsuarioInput {
    pub cedula: String,
    pub nombre: String,
    pub password: String,
    pub rol: RolUsuario,
    pub activo: bool,
}

pub struct ActualizarUsuarioInput {
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
}

pub struct CrearRootInicialInput {
    pub cedula: String,
    pub nombre: String,
    pub password: String,
}

pub struct UsuarioService<'a, R>
where
    R: UsuarioRepository + ?Sized,
{
    usuarios: &'a R,
}

impl<'a, R> UsuarioService<'a, R>
where
    R: UsuarioRepository + ?Sized,
{
    pub fn new(usuarios: &'a R) -> Self {
        Self { usuarios }
    }

    pub fn crear(&self, input: CrearUsuarioInput) -> Result<i64, UsuarioServiceError> {
        if self.requiere_configuracion_inicial()? {
            return Err(UsuarioServiceError::ConfiguracionInicialRequerida);
        }

        let usuario = self.construir_usuario(0, input)?;
        self.usuarios
            .crear(&usuario)
            .map_err(mapear_duplicado_usuario)
    }

    pub fn buscar_por_id(&self, id: i64) -> Result<Usuario, UsuarioServiceError> {
        self.usuarios
            .buscar_por_id(id)?
            .ok_or(UsuarioServiceError::UsuarioNoEncontrado)
    }

    pub fn buscar_por_cedula(&self, cedula: &str) -> Result<Usuario, UsuarioServiceError> {
        self.usuarios
            .buscar_por_cedula(cedula.trim())?
            .ok_or(UsuarioServiceError::UsuarioNoEncontrado)
    }

    pub fn actualizar(
        &self,
        id: i64,
        input: ActualizarUsuarioInput,
    ) -> Result<(), UsuarioServiceError> {
        self.buscar_por_id(id)?;
        let cedula = normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?;
        let nombre = normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?;

        self.usuarios
            .actualizar_identidad_y_rol(id, cedula, nombre, input.rol)
            .map_err(mapear_escritura_usuario)
    }

    pub fn cambiar_password(
        &self,
        id: i64,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        self.buscar_por_id(id)?;
        validar_password(nueva_password)?;
        let password_hash = generar_hash(nueva_password)?;
        self.usuarios
            .actualizar_password(id, &password_hash)
            .map_err(mapear_escritura_usuario)
    }

    pub fn activar(&self, id: i64) -> Result<(), UsuarioServiceError> {
        self.buscar_por_id(id)?;
        self.usuarios
            .establecer_activo(id, true)
            .map_err(mapear_escritura_usuario)
    }

    pub fn desactivar(&self, id: i64) -> Result<(), UsuarioServiceError> {
        self.buscar_por_id(id)?;
        self.usuarios
            .establecer_activo(id, false)
            .map_err(mapear_escritura_usuario)
    }

    pub fn listar(&self) -> Result<Vec<Usuario>, UsuarioServiceError> {
        Ok(self.usuarios.listar()?)
    }

    pub fn requiere_configuracion_inicial(&self) -> Result<bool, UsuarioServiceError> {
        Ok(self.usuarios.contar_usuarios()? == 0)
    }

    pub fn crear_root_inicial(
        &self,
        input: CrearRootInicialInput,
    ) -> Result<i64, UsuarioServiceError> {
        let usuario = self.construir_usuario(
            0,
            CrearUsuarioInput {
                cedula: input.cedula,
                nombre: input.nombre,
                password: input.password,
                rol: RolUsuario::Root,
                activo: true,
            },
        )?;
        self.usuarios
            .crear_root_inicial_atomico(&usuario)
            .map_err(mapear_escritura_usuario)
    }

    fn construir_usuario(
        &self,
        id: i64,
        input: CrearUsuarioInput,
    ) -> Result<Usuario, UsuarioServiceError> {
        let cedula = normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?;
        let nombre = normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?;
        validar_password(&input.password)?;

        Ok(Usuario {
            id,
            cedula: cedula.to_string(),
            nombre: nombre.to_string(),
            password_hash: generar_hash(&input.password)?,
            rol: input.rol,
            activo: input.activo,
        })
    }
}

fn mapear_duplicado_usuario(error: DatabaseError) -> UsuarioServiceError {
    if es_constraint_unique(&error) {
        UsuarioServiceError::CedulaDuplicada
    } else {
        UsuarioServiceError::Database(error)
    }
}

fn mapear_escritura_usuario(error: DatabaseError) -> UsuarioServiceError {
    match error {
        DatabaseError::ConfiguracionInicialYaRealizada => {
            UsuarioServiceError::ConfiguracionInicialYaRealizada
        }
        DatabaseError::UsuarioNoEncontrado => UsuarioServiceError::UsuarioNoEncontrado,
        DatabaseError::UltimoRootActivo => UsuarioServiceError::UltimoRootActivo,
        error => mapear_duplicado_usuario(error),
    }
}

fn es_constraint_unique(error: &DatabaseError) -> bool {
    matches!(
        error,
        DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(codigo, _))
            if codigo.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    )
}

fn normalizar_requerido(
    valor: &str,
    error: UsuarioServiceError,
) -> Result<&str, UsuarioServiceError> {
    let valor = valor.trim();
    if valor.is_empty() {
        return Err(error);
    }
    Ok(valor)
}

fn validar_password(password: &str) -> Result<(), UsuarioServiceError> {
    if password.chars().count() < LONGITUD_MINIMA_PASSWORD {
        return Err(UsuarioServiceError::PasswordDemasiadoCorto);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_sqlite_no_relacionado_permanece_tecnico() {
        let error = mapear_duplicado_usuario(DatabaseError::Sqlite(rusqlite::Error::InvalidQuery));

        assert!(matches!(error, UsuarioServiceError::Database(_)));
    }
}
