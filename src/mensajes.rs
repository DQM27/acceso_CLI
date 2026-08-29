//! Mensajes de error en español, compartidos por cualquier interfaz (TUI
//! clásica, comandos, futura GUI): traducen errores de servicio a texto
//! accionable sin exponer detalles internos de base de datos.

use crate::domain::resultado_acceso::MotivoDenegacion;
use crate::services::error::{
    AutenticacionError, ContratistaServiceError, EmpresaServiceError, RegistroIngresoServiceError,
    UsuarioServiceError,
};

/// `HashInvalido` va junto con `Database` a propósito: ambos son fallos de
/// infraestructura (hash corrupto en la fila, SQLite bloqueada/dañada), no
/// algo que el usuario hizo mal — no tiene sentido distinguirlos en pantalla,
/// y mucho menos dejar pasar el mensaje crudo de SQLite (`Database` es
/// `#[error(transparent)]` sobre `DatabaseError`, que sí interpola detalles
/// internos en su propio `Display`).
pub fn mensaje_autenticacion(error: AutenticacionError) -> String {
    match error {
        AutenticacionError::CredencialesInvalidas => "Credenciales inválidas".into(),
        AutenticacionError::UsuarioInactivo => "Usuario inactivo".into(),
        AutenticacionError::HashInvalido | AutenticacionError::Database(_) => {
            "No se pudo iniciar sesión, intentá de nuevo".into()
        }
    }
}

pub fn mensaje_empresa(error: EmpresaServiceError) -> String {
    match error {
        EmpresaServiceError::NombreDuplicado => "Ya existe una empresa con ese nombre".into(),
        EmpresaServiceError::NombreEmpresaVacio => "El nombre es obligatorio".into(),
        EmpresaServiceError::EmpresaNoEncontrada => "La empresa ya no existe".into(),
        EmpresaServiceError::OperacionNoAutorizada => {
            "Su sesión no está autorizada para esta operación".into()
        }
        EmpresaServiceError::Database(_) => "No se pudo guardar la empresa".into(),
    }
}

pub fn mensaje_contratista(error: ContratistaServiceError) -> String {
    use ContratistaServiceError::*;

    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        EmpresaNoEncontrada => "La empresa seleccionada ya no existe".into(),
        CedulaVacia => "La cédula es obligatoria".into(),
        NombreVacio => "El nombre es obligatorio".into(),
        PraindRequerido => "Fecha PRAIND requerida".into(),
        CedulaDuplicada => "Ya existe un contratista con esa cédula".into(),
        OperacionNoAutorizada => "Su sesión no está autorizada para esta operación".into(),
        Database(_) => "No se pudo guardar el contratista".into(),
    }
}

pub fn mensaje_usuario(error: UsuarioServiceError) -> String {
    match error {
        UsuarioServiceError::UsuarioNoEncontrado => "El usuario ya no existe".into(),
        UsuarioServiceError::CedulaVacia => "La cédula es obligatoria".into(),
        UsuarioServiceError::NombreVacio => "El nombre es obligatorio".into(),
        UsuarioServiceError::PasswordDemasiadoCorto => {
            "La contraseña debe tener al menos 8 caracteres".into()
        }
        UsuarioServiceError::CedulaDuplicada => "Ya existe un usuario con esa cédula".into(),
        UsuarioServiceError::UltimoRootActivo => {
            "Debe existir al menos un usuario ROOT activo".into()
        }
        UsuarioServiceError::OperacionNoAutorizada => {
            "Su sesión no está autorizada para gestionar ese usuario".into()
        }
        UsuarioServiceError::PasswordActualIncorrecta => {
            "La contraseña actual es incorrecta".into()
        }
        _ => "No se pudo guardar el usuario".into(),
    }
}

pub fn mensaje_salida(error: RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::*;

    match error {
        RegistroNoActivo => "El ingreso ya no está activo".into(),
        SalidaAnteriorAIngreso => "La salida no puede ser anterior al ingreso".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar la salida".into(),
    }
}

pub fn mensaje_ingreso(error: RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::*;

    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        IngresoActivo => "El contratista ya tiene un ingreso activo".into(),
        GafeteRequerido => "El gafete es requerido".into(),
        GafeteOcupado => "El gafete ya está en uso".into(),
        AccesoDenegado(MotivoDenegacion::SinAcceso) => "No tiene acceso autorizado".into(),
        AccesoDenegado(MotivoDenegacion::PraindVencido) => "PRAIND vencido".into(),
        AccesoDenegado(MotivoDenegacion::PraindNoRegistrado) => {
            "PRAIND sin fecha registrada".into()
        }
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar el ingreso".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::error::DatabaseError;

    #[test]
    fn los_errores_tecnicos_no_exponen_detalles_en_la_tui() {
        let empresa =
            EmpresaServiceError::Database(DatabaseError::FechaCorrupta("detalle interno".into()));
        let usuario =
            UsuarioServiceError::Database(DatabaseError::FechaCorrupta("detalle interno".into()));

        assert_eq!(mensaje_empresa(empresa), "No se pudo guardar la empresa");
        assert_eq!(mensaje_usuario(usuario), "No se pudo guardar el usuario");
    }

    #[test]
    fn un_fallo_de_sqlite_en_el_login_no_filtra_el_mensaje_crudo() {
        let error = AutenticacionError::Database(DatabaseError::FechaCorrupta(
            "Error de SQLite: detalle interno".into(),
        ));
        assert_eq!(mensaje_autenticacion(error), "No se pudo iniciar sesión, intentá de nuevo");
        assert_eq!(
            mensaje_autenticacion(AutenticacionError::HashInvalido),
            "No se pudo iniciar sesión, intentá de nuevo"
        );
    }

    #[test]
    fn credenciales_invalidas_y_usuario_inactivo_conservan_su_mensaje() {
        assert_eq!(
            mensaje_autenticacion(AutenticacionError::CredencialesInvalidas),
            "Credenciales inválidas"
        );
        assert_eq!(
            mensaje_autenticacion(AutenticacionError::UsuarioInactivo),
            "Usuario inactivo"
        );
    }

    #[test]
    fn los_errores_semanticos_conservan_mensajes_accionables() {
        assert_eq!(
            mensaje_contratista(ContratistaServiceError::CedulaDuplicada),
            "Ya existe un contratista con esa cédula"
        );
        assert_eq!(
            mensaje_salida(RegistroIngresoServiceError::RelojRetrocedido),
            "Revise la fecha y hora del equipo antes de continuar"
        );
    }

    #[test]
    fn las_denegaciones_de_ingreso_distinguen_el_motivo() {
        assert_eq!(
            mensaje_ingreso(RegistroIngresoServiceError::AccesoDenegado(
                MotivoDenegacion::SinAcceso,
            )),
            "No tiene acceso autorizado"
        );
        assert_eq!(
            mensaje_ingreso(RegistroIngresoServiceError::AccesoDenegado(
                MotivoDenegacion::PraindNoRegistrado,
            )),
            "PRAIND sin fecha registrada"
        );
    }
}
