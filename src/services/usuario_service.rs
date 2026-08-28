use chrono::{DateTime, Utc};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::{AuditoriaWriter, EntidadAuditada};
use crate::database::queries::usuarios::{FiltroUsuarios, UsuarioResumen, UsuariosQuery};
use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::{RolUsuario, Usuario};

use super::error::UsuarioServiceError;
use super::password::{generar_hash, validar_formato_hash, verificar_password};

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
        self.buscar_para_tabla_como(filtro, RolUsuario::Root)
    }

    pub fn buscar_para_tabla_como(
        &self,
        filtro: &FiltroUsuarios,
        actor: RolUsuario,
    ) -> Result<Vec<UsuarioResumen>, UsuarioServiceError> {
        Ok(self.consultas.buscar_para_actor(filtro, actor)?)
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
        self.validar_datos_para_crear(&input)?;
        let password_hash = generar_hash(&input.password)?;
        self.crear_con_hash(
            &input.cedula,
            &input.nombre,
            input.rol,
            input.activo,
            password_hash,
        )
    }

    /// Parte barata de `crear` (sin Argon2): normaliza y valida sin escribir nada. Permite
    /// correr el hash en un hilo aparte sin bloquear el hilo de eventos de la TUI.
    pub fn validar_datos_para_crear(
        &self,
        input: &CrearUsuarioInput,
    ) -> Result<(), UsuarioServiceError> {
        if self.requiere_configuracion_inicial()? {
            return Err(UsuarioServiceError::ConfiguracionInicialRequerida);
        }
        normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?;
        normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?;
        validar_password(&input.password)?;
        Ok(())
    }

    /// Parte que sí escribe, recibiendo el hash ya calculado. Repite el chequeo de
    /// configuración inicial que ya hizo `validar_datos_para_crear` — entre validar y
    /// recibir el hash pasó tiempo (Argon2 corrió en otro hilo) y la comprobación es
    /// barata; no asume que nada cambió mientras tanto.
    pub fn crear_con_hash(
        &self,
        cedula: &str,
        nombre: &str,
        rol: RolUsuario,
        activo: bool,
        password_hash: String,
    ) -> Result<i64, UsuarioServiceError> {
        if self.requiere_configuracion_inicial()? {
            return Err(UsuarioServiceError::ConfiguracionInicialRequerida);
        }
        validar_formato_hash(&password_hash)?;
        let cedula = normalizar_requerido(cedula, UsuarioServiceError::CedulaVacia)?.to_string();
        let nombre = normalizar_requerido(nombre, UsuarioServiceError::NombreVacio)?.to_string();
        let usuario = Usuario {
            id: 0,
            cedula,
            nombre,
            password_hash,
            rol,
            activo,
        };
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

    pub fn actualizar_administracion(
        &self,
        id: i64,
        input: ActualizarUsuarioInput,
        activo: bool,
    ) -> Result<(), UsuarioServiceError> {
        let mut usuario = self.buscar_por_id(id)?;
        usuario.cedula =
            normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?.into();
        usuario.nombre =
            normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?.into();
        usuario.rol = input.rol;
        usuario.activo = activo;
        self.usuarios
            .actualizar(&usuario)
            .map_err(mapear_escritura_usuario)
    }

    pub fn cambiar_password(
        &self,
        id: i64,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        self.validar_password_para_cambio(id, nueva_password)?;
        let password_hash = generar_hash(nueva_password)?;
        self.cambiar_password_con_hash(id, &password_hash)
    }

    pub fn cambiar_password_propio(
        &self,
        id: i64,
        password_actual: &str,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        self.validar_password_para_cambio(id, nueva_password)?;
        self.validar_password_actual(id, password_actual)?;
        let password_hash = generar_hash(nueva_password)?;
        self.cambiar_password_con_hash(id, &password_hash)
    }

    pub fn validar_password_actual(
        &self,
        id: i64,
        password_actual: &str,
    ) -> Result<(), UsuarioServiceError> {
        let usuario = self.buscar_por_id(id)?;
        if verificar_password(password_actual, &usuario.password_hash)? {
            Ok(())
        } else {
            Err(UsuarioServiceError::PasswordActualIncorrecta)
        }
    }

    /// Parte barata de `cambiar_password` (sin Argon2).
    pub fn validar_password_para_cambio(
        &self,
        id: i64,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        self.buscar_por_id(id)?;
        validar_password(nueva_password)?;
        Ok(())
    }

    /// Parte que sí escribe, recibiendo el hash ya calculado.
    pub fn cambiar_password_con_hash(
        &self,
        id: i64,
        password_hash: &str,
    ) -> Result<(), UsuarioServiceError> {
        validar_formato_hash(password_hash)?;
        self.usuarios
            .actualizar_password(id, password_hash)
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

    #[allow(clippy::too_many_arguments)]
    pub fn actualizar_administracion_auditada<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        input: ActualizarUsuarioInput,
        activo: bool,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), UsuarioServiceError> {
        let mut usuario = self.buscar_por_id(id)?;
        let cedula_anterior = usuario.cedula.clone();
        let nombre_anterior = usuario.nombre.clone();
        let rol_anterior = usuario.rol;
        let activo_anterior = usuario.activo;

        usuario.cedula =
            normalizar_requerido(&input.cedula, UsuarioServiceError::CedulaVacia)?.into();
        usuario.nombre =
            normalizar_requerido(&input.nombre, UsuarioServiceError::NombreVacio)?.into();
        usuario.rol = input.rol;
        usuario.activo = activo;
        self.usuarios
            .actualizar(&usuario)
            .map_err(mapear_escritura_usuario)?;

        // `entidad_nombre` es el nombre ya actualizado — mismo criterio que
        // `ContratistaService::actualizar_auditado`.
        let registrar = |campo: &str, anterior: Option<&str>, nuevo: Option<&str>| {
            auditoria.registrar_cambio(
                fecha_hora,
                actor_id,
                actor_nombre,
                EntidadAuditada::Usuario,
                id,
                &usuario.nombre,
                campo,
                anterior,
                nuevo,
            )
        };
        if cedula_anterior != usuario.cedula {
            registrar("cedula", Some(&cedula_anterior), Some(&usuario.cedula))?;
        }
        if nombre_anterior != usuario.nombre {
            registrar("nombre", Some(&nombre_anterior), Some(&usuario.nombre))?;
        }
        if rol_anterior != usuario.rol {
            registrar(
                "rol",
                Some(&texto_rol(rol_anterior)),
                Some(&texto_rol(usuario.rol)),
            )?;
        }
        if activo_anterior != usuario.activo {
            registrar(
                "activo",
                Some(texto_si_no(activo_anterior)),
                Some(texto_si_no(usuario.activo)),
            )?;
        }
        Ok(())
    }

    /// Igual que `activar`/`desactivar`, auditado — usado por el toggle
    /// rápido de la grilla (sin pasar por el formulario completo de
    /// `actualizar_administracion_auditada`).
    pub fn establecer_activo_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        activo: bool,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), UsuarioServiceError> {
        let actual = self.buscar_por_id(id)?;
        self.usuarios
            .establecer_activo(id, activo)
            .map_err(mapear_escritura_usuario)?;
        if actual.activo != activo {
            auditoria.registrar_cambio(
                fecha_hora,
                actor_id,
                actor_nombre,
                EntidadAuditada::Usuario,
                id,
                &actual.nombre,
                "activo",
                Some(texto_si_no(actual.activo)),
                Some(texto_si_no(activo)),
            )?;
        }
        Ok(())
    }

    /// Igual que `cambiar_password_con_hash`, pero deja un marcador en la
    /// auditoría — sólo la fecha importa (decisión explícita del usuario:
    /// "no valores, solo fecha"), así que `valor_anterior`/`valor_nuevo`
    /// quedan en blanco a propósito, no es un descuido.
    pub fn cambiar_password_con_hash_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        password_hash: &str,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), UsuarioServiceError> {
        let objetivo = self.buscar_por_id(id)?;
        self.cambiar_password_con_hash(id, password_hash)?;
        auditoria.registrar_cambio(
            fecha_hora,
            actor_id,
            actor_nombre,
            EntidadAuditada::Usuario,
            id,
            &objetivo.nombre,
            "password",
            None,
            None,
        )?;
        Ok(())
    }

    /// Camino síncrono (`cambiar_password`, con Argon2 adentro) + auditoría
    /// — para interfaces sin el paso off-thread de la TUI/GUI (hoy,
    /// `--comandos`). Reset administrativo: `actor` distinto del usuario que
    /// recibe la contraseña nueva.
    #[allow(clippy::too_many_arguments)]
    pub fn cambiar_password_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        nueva_password: &str,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), UsuarioServiceError> {
        self.validar_password_para_cambio(id, nueva_password)?;
        let password_hash = generar_hash(nueva_password)?;
        self.cambiar_password_con_hash_auditado(
            id,
            &password_hash,
            actor_id,
            actor_nombre,
            fecha_hora,
            auditoria,
        )
    }

    /// Igual que `cambiar_password_propio` + auditoría — el propio usuario
    /// es tanto el actor como el objetivo del cambio.
    pub fn cambiar_password_propio_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        password_actual: &str,
        nueva_password: &str,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), UsuarioServiceError> {
        self.validar_password_para_cambio(id, nueva_password)?;
        self.validar_password_actual(id, password_actual)?;
        let password_hash = generar_hash(nueva_password)?;
        self.cambiar_password_con_hash_auditado(
            id,
            &password_hash,
            id,
            actor_nombre,
            fecha_hora,
            auditoria,
        )
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

fn texto_si_no(valor: bool) -> &'static str {
    if valor { "SI" } else { "NO" }
}

fn texto_rol(rol: RolUsuario) -> String {
    format!("{rol:?}")
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
