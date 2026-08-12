use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::{RolUsuario, Usuario};

use super::error::UsuarioServiceError;
use super::password::generar_hash;

const LONGITUD_MINIMA_PASSWORD: usize = 8;

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
        Ok(self.usuarios.crear(&usuario)?)
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
        let mut usuario = self.buscar_por_id(id)?;
        let cedula = normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?;
        let nombre = normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?;

        if usuario.rol == RolUsuario::Root
            && usuario.activo
            && input.rol != RolUsuario::Root
            && self.usuarios.contar_roots_activos()? == 1
        {
            return Err(UsuarioServiceError::UltimoRootActivo);
        }

        usuario.cedula = cedula.to_string();
        usuario.nombre = nombre.to_string();
        usuario.rol = input.rol;
        Ok(self.usuarios.actualizar(&usuario)?)
    }

    pub fn cambiar_password(
        &self,
        id: i64,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        let mut usuario = self.buscar_por_id(id)?;
        validar_password(nueva_password)?;
        usuario.password_hash = generar_hash(nueva_password)?;
        Ok(self.usuarios.actualizar(&usuario)?)
    }

    pub fn activar(&self, id: i64) -> Result<(), UsuarioServiceError> {
        let mut usuario = self.buscar_por_id(id)?;
        usuario.activo = true;
        Ok(self.usuarios.actualizar(&usuario)?)
    }

    pub fn desactivar(&self, id: i64) -> Result<(), UsuarioServiceError> {
        let mut usuario = self.buscar_por_id(id)?;

        if usuario.rol == RolUsuario::Root
            && usuario.activo
            && self.usuarios.contar_roots_activos()? == 1
        {
            return Err(UsuarioServiceError::UltimoRootActivo);
        }

        usuario.activo = false;
        Ok(self.usuarios.actualizar(&usuario)?)
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
        if !self.requiere_configuracion_inicial()? {
            return Err(UsuarioServiceError::ConfiguracionInicialYaRealizada);
        }

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
        Ok(self.usuarios.crear(&usuario)?)
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
