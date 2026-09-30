use crate::database::error::DatabaseError;
use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::{RolUsuario, Usuario};

use super::error::UsuarioServiceError;
use super::password::{generar_hash, validar_formato_hash};

const LONGITUD_MINIMA_PASSWORD: usize = 8;

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
        self.validar_datos_para_root_inicial(&input)?;
        let password_hash = generar_hash(&input.password)?;
        self.crear_root_inicial_con_hash(input, password_hash)
    }

    /// Parte barata de `crear_root_inicial` (sin Argon2). Deliberadamente **no** incluye
    /// la comprobación de "ya existe un ROOT": esa sigue siendo atómica con el insert en
    /// `crear_root_inicial_atomico` (ver `crear_root_inicial_con_hash`), porque repartirla
    /// aparte reabriría la ventana de carrera entre dos instancias que
    /// `crear_root_inicial_atomico` existe justamente para cerrar.
    pub fn validar_datos_para_root_inicial(
        &self,
        input: &CrearRootInicialInput,
    ) -> Result<(), UsuarioServiceError> {
        normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?;
        normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?;
        validar_password(&input.password)?;
        Ok(())
    }

    /// Parte que sí escribe, recibiendo el hash ya calculado. El chequeo-e-inserción
    /// atómico de "sólo un ROOT inicial" ocurre aquí, no antes.
    pub fn crear_root_inicial_con_hash(
        &self,
        input: CrearRootInicialInput,
        password_hash: String,
    ) -> Result<i64, UsuarioServiceError> {
        validar_formato_hash(&password_hash)?;
        let cedula =
            normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?.to_string();
        let nombre =
            normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?.to_string();
        let usuario = Usuario {
            id: 0,
            cedula,
            nombre,
            password_hash,
            rol: RolUsuario::Root,
            activo: true,
            password_hash_confirmado_en: None,
            password_temporal_cacheada: false,
        };
        self.usuarios
            .crear_root_inicial_atomico(&usuario)
            .map_err(mapear_escritura_usuario)
    }
}

fn mapear_duplicado_usuario(error: DatabaseError) -> UsuarioServiceError {
    if error.es_constraint_unique() {
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
