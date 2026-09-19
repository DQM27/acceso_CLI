//! Mensajes de error en español, compartidos por cualquier interfaz (TUI
//! clásica, CLI, futura GUI): traducen errores de servicio a texto
//! accionable sin exponer detalles internos de base de datos.
//!
//! Además, este es el cuello de botella por donde pasa TODO error de
//! servicio antes de llegar a una interfaz (TUI, comandos Tauri de
//! escritorio, y la mayoría de las llamadas `UniFFI` de mobile) -- por eso es
//! el lugar correcto para loguear el detalle técnico que el mensaje al
//! usuario nunca expone (`docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`,
//! punto 5.1). Sólo se loguean las variantes técnicas/inesperadas
//! (`Database`, `Red`, `Io`, etc.) -- un rechazo de negocio normal (cédula
//! duplicada, sesión no autorizada) no es un fallo, es el flujo esperado, y
//! loguear cada uno sería puro ruido.

use crate::domain::cita::MotivoDenegacionVisita;
use crate::domain::resultado_acceso::MotivoDenegacion;
use crate::models::gafete::EstadoGafete;
use crate::services::error::{
    AutenticacionError, CitaServiceError, ContratistaServiceError, EmpresaProveedorServiceError,
    EmpresaServiceError, EncargadoRutaServiceError, GafeteProvisionalServiceError,
    GafeteServiceError, IngresoProveedorServiceError, RegistroIngresoServiceError,
    RutaCatalogoServiceError, RutaServiceError, UsuarioServiceError, VehiculoRutaServiceError,
};

/// `HashInvalido` va junto con `Database` a propósito: ambos son fallos de
/// infraestructura (hash corrupto en la fila, `SQLite` bloqueada/dañada), no
/// algo que el usuario hizo mal — no tiene sentido distinguirlos en pantalla,
/// y mucho menos dejar pasar el mensaje crudo de `SQLite` (`Database` es
/// `#[error(transparent)]` sobre `DatabaseError`, que sí interpola detalles
/// internos en su propio `Display`).
pub fn mensaje_autenticacion(error: AutenticacionError) -> String {
    match error {
        AutenticacionError::CredencialesInvalidas => "Credenciales inválidas".into(),
        AutenticacionError::UsuarioInactivo => "Usuario inactivo".into(),
        AutenticacionError::SinPasswordLocal => {
            "Todavía no tenés contraseña en este dispositivo -- fijá una para continuar".into()
        }
        AutenticacionError::HashInvalido => {
            log::error!("autenticación: hash de contraseña almacenado inválido");
            "No se pudo iniciar sesión, intentá de nuevo".into()
        }
        AutenticacionError::Database(error) => {
            log::error!("autenticación: {error}");
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
        EmpresaServiceError::Database(error) => {
            log::error!("empresa: {error}");
            "No se pudo guardar la empresa".into()
        }
    }
}

pub fn mensaje_contratista(error: ContratistaServiceError) -> String {
    use ContratistaServiceError::{
        CedulaDuplicada, CedulaVacia, ContratistaNoEncontrado, Database, EmpresaNoEncontrada,
        NombreVacio, OperacionNoAutorizada, PraindRequerido,
    };

    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        EmpresaNoEncontrada => "La empresa seleccionada ya no existe".into(),
        CedulaVacia => "La cédula es obligatoria".into(),
        NombreVacio => "El nombre es obligatorio".into(),
        PraindRequerido => "Fecha PRAIND requerida".into(),
        CedulaDuplicada => "Ya existe un contratista con esa cédula".into(),
        OperacionNoAutorizada => "Su sesión no está autorizada para esta operación".into(),
        Database(error) => {
            log::error!("contratista: {error}");
            "No se pudo guardar el contratista".into()
        }
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
        UsuarioServiceError::ConfiguracionInicialRequerida => {
            "Se requiere crear el usuario ROOT inicial".into()
        }
        UsuarioServiceError::ConfiguracionInicialYaRealizada => {
            "La configuración inicial ya fue realizada".into()
        }
        UsuarioServiceError::PasswordActualIncorrecta => {
            "La contraseña actual es incorrecta".into()
        }
        UsuarioServiceError::UsuarioInactivo => "Usuario inactivo".into(),
        UsuarioServiceError::YaTienePasswordLocal => {
            "Este usuario ya tiene contraseña en este dispositivo".into()
        }
        UsuarioServiceError::Password(error) => {
            log::error!("usuario: {error}");
            "No se pudo guardar el usuario".into()
        }
        UsuarioServiceError::Database(error) => {
            log::error!("usuario: {error}");
            "No se pudo guardar el usuario".into()
        }
    }
}

pub fn mensaje_gafete(error: GafeteServiceError) -> String {
    match error {
        GafeteServiceError::NumeroInvalido => "El número de gafete debe ser mayor a cero".into(),
        GafeteServiceError::NumeroDuplicado => "Ya existe un gafete con ese número".into(),
        GafeteServiceError::GafeteNoEncontrado => "El gafete ya no existe".into(),
        GafeteServiceError::RangoInvalido => "El rango de números no es válido".into(),
        GafeteServiceError::ContratistaDeudorRequerido => {
            "Debe indicar el contratista deudor".into()
        }
        GafeteServiceError::ContratistaNoEncontrado => "El contratista ya no existe".into(),
        GafeteServiceError::VisitaNoEncontrada => "El visitante ya no existe".into(),
        GafeteServiceError::EncargadoRutaNoEncontrado => "El encargado de ruta ya no existe".into(),
        GafeteServiceError::EstadoInvalido => {
            "El gafete no está en un estado válido para esa operación".into()
        }
        GafeteServiceError::GafeteConIngresoActivo => {
            "El gafete está asignado a un ingreso activo".into()
        }
        GafeteServiceError::OperacionNoAutorizada => {
            "Su sesión no está autorizada para esta operación".into()
        }
        GafeteServiceError::Database(error) => {
            log::error!("gafete: {error}");
            "No se pudo guardar el gafete".into()
        }
    }
}

pub fn mensaje_salida(error: RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::{RegistroNoActivo, RelojRetrocedido, SalidaAnteriorAIngreso};

    match error {
        RegistroNoActivo => "El ingreso ya no está activo".into(),
        SalidaAnteriorAIngreso => "La salida no puede ser anterior al ingreso".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        RegistroIngresoServiceError::Database(error) => {
            log::error!("salida: {error}");
            "No se pudo registrar la salida".into()
        }
        _ => "No se pudo registrar la salida".into(),
    }
}

pub fn mensaje_cita(error: CitaServiceError) -> String {
    use CitaServiceError::{
        GafeteNoDisponible, GafeteNoRegistrado, GafeteOcupado, MovimientoNoActivo,
        OperadorNoAutorizado, RelojRetrocedido, SalidaAnteriorAEntrada, SinCitaRegistrada,
        SinCitaVigente, VisitanteYaEnSitio,
    };

    match error {
        SinCitaRegistrada => "No hay ninguna visita agendada para esta cédula".into(),
        SinCitaVigente(MotivoDenegacionVisita::CitaCancelada) => "Esta visita fue cancelada".into(),
        SinCitaVigente(MotivoDenegacionVisita::FueraDeVigencia) => {
            "Esta visita no está vigente hoy".into()
        }
        VisitanteYaEnSitio => "Este visitante ya tiene un ingreso activo".into(),
        GafeteOcupado => "El gafete ya está en uso por otra visita".into(),
        GafeteNoRegistrado => "El número de gafete no existe en el catálogo".into(),
        GafeteNoDisponible(EstadoGafete::Perdido) => "El gafete está marcado como perdido".into(),
        GafeteNoDisponible(EstadoGafete::DeBaja) => "El gafete está dado de baja".into(),
        GafeteNoDisponible(EstadoGafete::Disponible) => {
            unreachable!("GafeteNoDisponible nunca se genera con estado Disponible")
        }
        MovimientoNoActivo => "El movimiento ya no está activo".into(),
        SalidaAnteriorAEntrada => "La salida no puede ser anterior a la entrada".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        OperadorNoAutorizado => {
            "La sesión que registra el movimiento no existe o está inactiva".into()
        }
        CitaServiceError::Database(error) => {
            log::error!("cita: {error}");
            "No se pudo registrar el movimiento de la visita".into()
        }
    }
}

pub fn mensaje_ingreso(error: RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::{
        AccesoDenegado, ContratistaNoEncontrado, GafeteNoDisponible, GafeteNoRegistrado,
        GafeteOcupado, GafeteRequerido, IngresoActivo, RelojRetrocedido,
    };

    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        IngresoActivo => "El contratista ya tiene un ingreso activo".into(),
        GafeteRequerido => "El gafete es requerido".into(),
        GafeteOcupado => "El gafete ya está en uso".into(),
        GafeteNoRegistrado => "El número de gafete no existe en el catálogo".into(),
        GafeteNoDisponible(EstadoGafete::Perdido) => "El gafete está marcado como perdido".into(),
        GafeteNoDisponible(EstadoGafete::DeBaja) => "El gafete está dado de baja".into(),
        GafeteNoDisponible(EstadoGafete::Disponible) => {
            unreachable!("GafeteNoDisponible nunca se genera con estado Disponible")
        }
        AccesoDenegado(MotivoDenegacion::SinAcceso) => "No tiene acceso autorizado".into(),
        AccesoDenegado(MotivoDenegacion::PraindVencido) => "PRAIND vencido".into(),
        AccesoDenegado(MotivoDenegacion::PraindNoRegistrado) => {
            "PRAIND sin fecha registrada".into()
        }
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        RegistroIngresoServiceError::Database(error) => {
            log::error!("ingreso: {error}");
            "No se pudo registrar el ingreso".into()
        }
        _ => "No se pudo registrar el ingreso".into(),
    }
}

pub fn mensaje_vehiculo_ruta(error: VehiculoRutaServiceError) -> String {
    match error {
        VehiculoRutaServiceError::OperacionNoAutorizada => {
            "Su sesión no está autorizada para esta operación".into()
        }
        VehiculoRutaServiceError::Database(error) => {
            log::error!("vehículo de ruta: {error}");
            "No se pudo guardar el vehículo".into()
        }
    }
}

pub fn mensaje_encargado_ruta(error: EncargadoRutaServiceError) -> String {
    match error {
        EncargadoRutaServiceError::OperacionNoAutorizada => {
            "Su sesión no está autorizada para esta operación".into()
        }
        EncargadoRutaServiceError::Database(error) => {
            log::error!("encargado de ruta: {error}");
            "No se pudo guardar el encargado".into()
        }
    }
}

pub fn mensaje_ruta(error: RutaServiceError) -> String {
    use RutaServiceError::{
        DocumentoRequiereAutorizacion, EncargadoVacio, NumeroDocumentoVacio, OperadorNoAutorizado,
        PlacaVacia, RelojRetrocedido, RetornoAnteriorASalida, RutaInactiva, RutaNoEncontrada,
        SalidaNoActiva, SinDocumentos, VehiculoYaEnRuta, ViajeNoCoincide, ViajeNoEncontrado,
        ViajeYaCerrado,
    };

    match error {
        PlacaVacia => "La placa del vehículo es obligatoria".into(),
        EncargadoVacio => "El nombre del encargado es obligatorio".into(),
        NumeroDocumentoVacio => "El número de documento es obligatorio".into(),
        VehiculoYaEnRuta => "Este vehículo ya tiene una salida de ruta activa".into(),
        SinDocumentos => "La salida debe declarar al menos un documento".into(),
        DocumentoRequiereAutorizacion => {
            "El documento no es de hoy -- confirme que cuenta con el correo de autorización".into()
        }
        RutaNoEncontrada => "El número de ruta no existe en el catálogo".into(),
        RutaInactiva => "El número de ruta está dado de baja".into(),
        SalidaNoActiva => "La salida de ruta ya no está activa".into(),
        RetornoAnteriorASalida => "El retorno no puede ser anterior a la salida".into(),
        ViajeNoEncontrado => "El viaje que se quiere continuar no existe".into(),
        ViajeYaCerrado => "El viaje que se quiere continuar ya está cerrado".into(),
        ViajeNoCoincide => {
            "El vehículo o el encargado no coinciden con los del viaje que se quiere continuar"
                .into()
        }
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        OperadorNoAutorizado => {
            "La sesión que registra el movimiento no existe o está inactiva".into()
        }
        RutaServiceError::Database(error) => {
            log::error!("ruta: {error}");
            "No se pudo registrar el movimiento de la ruta".into()
        }
    }
}

pub fn mensaje_ruta_catalogo(error: RutaCatalogoServiceError) -> String {
    use RutaCatalogoServiceError::{
        NumeroDuplicado, NumeroInvalido, OperacionNoAutorizada, RangoInvalido, RutaConSalidaActiva,
        RutaNoEncontrada,
    };

    match error {
        NumeroInvalido => "El número de ruta debe ser mayor a cero".into(),
        NumeroDuplicado => "Ya existe una ruta con ese número".into(),
        RangoInvalido => "El rango de números no es válido".into(),
        RutaNoEncontrada => "La ruta ya no existe".into(),
        RutaConSalidaActiva => "La ruta tiene una salida activa en este momento".into(),
        OperacionNoAutorizada => "Su sesión no está autorizada para esta operación".into(),
        RutaCatalogoServiceError::Database(error) => {
            log::error!("catálogo de rutas: {error}");
            "No se pudo guardar la ruta".into()
        }
    }
}

pub fn mensaje_gafete_provisional(error: GafeteProvisionalServiceError) -> String {
    use GafeteProvisionalServiceError::{
        Database, EncargadoInactivo, EncargadoNoEncontrado, EncargadoYaTienePrestamoActivo,
        GafeteYaPrestado, NumeroInvalido, OperacionNoAutorizada, PrestamoNoActivo,
    };

    match error {
        NumeroInvalido => "El número de gafete debe ser mayor a cero".into(),
        EncargadoNoEncontrado => "El encargado no existe en el catálogo".into(),
        EncargadoInactivo => "El encargado está dado de baja en el catálogo".into(),
        EncargadoYaTienePrestamoActivo => {
            "Este encargado ya tiene un gafete provisional prestado".into()
        }
        GafeteYaPrestado => "Ese número de gafete ya está prestado a otra persona".into(),
        PrestamoNoActivo => "El préstamo no está activo".into(),
        OperacionNoAutorizada => {
            "La sesión actual no está autorizada para realizar esta operación".into()
        }
        Database(error) => {
            log::error!("gafete provisional: {error}");
            "No se pudo guardar el préstamo de gafete".into()
        }
    }
}

pub fn mensaje_empresa_proveedor(error: EmpresaProveedorServiceError) -> String {
    use EmpresaProveedorServiceError::{
        EmpresaNoEncontrada, NombreDuplicado, NombreEmpresaVacio, OperacionNoAutorizada,
    };

    match error {
        NombreEmpresaVacio => "El nombre de la empresa es obligatorio".into(),
        NombreDuplicado => "El nombre de la empresa ya existe".into(),
        EmpresaNoEncontrada => "Empresa proveedora no encontrada".into(),
        OperacionNoAutorizada => {
            "La sesión actual no está autorizada para realizar esta operación".into()
        }
        EmpresaProveedorServiceError::Database(error) => {
            log::error!("empresa proveedora: {error}");
            "No se pudo guardar la empresa".into()
        }
    }
}

pub fn mensaje_ingreso_proveedor(error: IngresoProveedorServiceError) -> String {
    use IngresoProveedorServiceError::{
        CedulaVacia, EmpresaInactiva, EmpresaNoEncontrada, GafeteNoDisponible, GafeteNoRegistrado,
        GafeteOcupado, IngresoActivo, NombreVacio, OperadorNoAutorizado, RegistroNoActivo,
        RelojRetrocedido, SalidaAnteriorAIngreso,
    };

    match error {
        CedulaVacia => "La cédula es obligatoria".into(),
        NombreVacio => "El nombre es obligatorio".into(),
        EmpresaNoEncontrada => "Empresa proveedora no encontrada".into(),
        EmpresaInactiva => "La empresa proveedora está dada de baja".into(),
        IngresoActivo => "Esta cédula ya tiene un ingreso de proveedor activo".into(),
        GafeteOcupado => "El gafete ya está asignado a otro ingreso de proveedor".into(),
        GafeteNoRegistrado => "El gafete no está registrado en el catálogo".into(),
        GafeteNoDisponible(_) => "El gafete no está disponible".into(),
        RegistroNoActivo => "El ingreso de proveedor no está activo".into(),
        SalidaAnteriorAIngreso => "La salida no puede ser anterior al ingreso".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        OperadorNoAutorizado => {
            "La sesión que registra el movimiento no existe o está inactiva".into()
        }
        IngresoProveedorServiceError::Database(error) => {
            log::error!("ingreso de proveedor: {error}");
            "No se pudo registrar el movimiento".into()
        }
    }
}

/// `RespuestaInesperada` trae el cuerpo crudo de la respuesta del receptor
/// (puede incluir detalles internos de Postgres/PostgREST) -- nunca pasa a
/// pantalla, mismo criterio que el resto de este módulo con los errores de
/// `SQLite`.
#[cfg(feature = "nube")]
pub fn mensaje_nube(error: crate::nube::NubeError) -> String {
    use crate::nube::NubeError;

    match error {
        NubeError::CredencialesInvalidas => {
            "El secreto de este dispositivo fue rechazado o revocado".into()
        }
        NubeError::DispositivoSuspendido => {
            "Este dispositivo fue suspendido -- contactá a un administrador".into()
        }
        NubeError::VersionDesactualizada => {
            "Esta versión de la app ya no es compatible -- actualizá para seguir sincronizando"
                .into()
        }
        NubeError::Red(error) => {
            log::warn!("nube: {error}");
            "No se pudo conectar con la nube, intentá de nuevo".into()
        }
    }
}

#[cfg(feature = "nube")]
pub fn mensaje_sincronizacion(error: crate::nube::SincronizacionError) -> String {
    use crate::nube::SincronizacionError;

    match error {
        SincronizacionError::BaseLocal(error) => {
            log::error!("sincronización: {error}");
            "No se pudo leer la base de datos local".into()
        }
        SincronizacionError::Red(error) => mensaje_nube(error),
        SincronizacionError::RespuestaInesperada { status, cuerpo } => {
            log::error!("sincronización: respuesta inesperada del receptor ({status}): {cuerpo}");
            "El receptor rechazó el pedido, intentá de nuevo más tarde".into()
        }
        SincronizacionError::FechaInvalida(error) => {
            log::error!("sincronización: fecha inválida del receptor: {error}");
            "El receptor mandó una fecha que no se pudo interpretar, intentá de nuevo más tarde"
                .into()
        }
    }
}

#[cfg(feature = "nube")]
pub fn mensaje_gestion_nube(error: crate::application::GestionNubeError) -> String {
    use crate::application::GestionNubeError;

    match error {
        GestionNubeError::OperacionNoAutorizada => {
            "Sólo una sesión ROOT activa puede gestionar la nube".into()
        }
        GestionNubeError::UsoNoAutorizado => {
            "Su sesión no está autorizada para usar la nube".into()
        }
        GestionNubeError::Sqlite(error) => {
            log::error!("gestión de nube: {error}");
            "No se pudo leer la base de datos local".into()
        }
        GestionNubeError::Usuario(error) => {
            log::error!("gestión de nube: {error}");
            "No se pudo leer la base de datos local".into()
        }
        GestionNubeError::SinSecreto => {
            "Todavía no se guardó el secreto de este dispositivo".into()
        }
        GestionNubeError::Io(error) => {
            log::error!("gestión de nube: {error}");
            "No se pudo guardar el secreto localmente".into()
        }
        GestionNubeError::Autenticacion(error) => mensaje_nube(error),
        GestionNubeError::Sincronizacion(error) => mensaje_sincronizacion(error),
        GestionNubeError::YaConfigurado => {
            "Este dispositivo ya tiene usuarios locales -- no hace falta el arranque inicial".into()
        }
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
        assert_eq!(
            mensaje_autenticacion(error),
            "No se pudo iniciar sesión, intentá de nuevo"
        );
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

    #[test]
    fn los_errores_tecnicos_de_rutas_no_exponen_detalles() {
        let vehiculo = VehiculoRutaServiceError::Database(DatabaseError::FechaCorrupta(
            "detalle interno".into(),
        ));
        let encargado = EncargadoRutaServiceError::Database(DatabaseError::FechaCorrupta(
            "detalle interno".into(),
        ));

        assert_eq!(
            mensaje_vehiculo_ruta(vehiculo),
            "No se pudo guardar el vehículo"
        );
        assert_eq!(
            mensaje_encargado_ruta(encargado),
            "No se pudo guardar el encargado"
        );
    }

    #[test]
    fn los_mensajes_de_ruta_conservan_su_motivo() {
        assert_eq!(
            mensaje_ruta(RutaServiceError::VehiculoYaEnRuta),
            "Este vehículo ya tiene una salida de ruta activa"
        );
        assert_eq!(
            mensaje_ruta(RutaServiceError::RelojRetrocedido),
            "Revise la fecha y hora del equipo antes de continuar"
        );
        assert_eq!(
            mensaje_ruta(RutaServiceError::OperadorNoAutorizado),
            "La sesión que registra el movimiento no existe o está inactiva"
        );
    }

    #[test]
    fn los_mensajes_de_proveedores_conservan_su_motivo() {
        assert_eq!(
            mensaje_empresa_proveedor(EmpresaProveedorServiceError::NombreDuplicado),
            "El nombre de la empresa ya existe"
        );
        assert_eq!(
            mensaje_ingreso_proveedor(IngresoProveedorServiceError::GafeteOcupado),
            "El gafete ya está asignado a otro ingreso de proveedor"
        );
        assert_eq!(
            mensaje_ingreso_proveedor(IngresoProveedorServiceError::OperadorNoAutorizado),
            "La sesión que registra el movimiento no existe o está inactiva"
        );
    }

    #[test]
    fn los_errores_tecnicos_de_proveedores_no_exponen_detalles() {
        let empresa = EmpresaProveedorServiceError::Database(DatabaseError::FechaCorrupta(
            "detalle interno".into(),
        ));
        let ingreso = IngresoProveedorServiceError::Database(DatabaseError::FechaCorrupta(
            "detalle interno".into(),
        ));

        assert_eq!(
            mensaje_empresa_proveedor(empresa),
            "No se pudo guardar la empresa"
        );
        assert_eq!(
            mensaje_ingreso_proveedor(ingreso),
            "No se pudo registrar el movimiento"
        );
    }

    #[cfg(feature = "nube")]
    #[test]
    fn el_cuerpo_crudo_de_una_respuesta_inesperada_no_llega_a_pantalla() {
        let error = crate::nube::SincronizacionError::RespuestaInesperada {
            status: 500,
            cuerpo: "detalle interno de postgrest".into(),
        };
        assert_eq!(
            mensaje_sincronizacion(error),
            "El receptor rechazó el pedido, intentá de nuevo más tarde"
        );
    }

    /// Regresión: `GestionarNube` (exclusivo ROOT) y `UsarNube` (cualquier
    /// rol) compartían la misma variante de error con un único mensaje
    /// redactado sólo para el caso ROOT -- confundía a quien depuraba un
    /// fallo de `UsarNube` haciéndole creer que era una función exclusiva
    /// de ROOT (ver `application::nube::GestionNubeError`).
    #[cfg(feature = "nube")]
    #[test]
    fn la_autorizacion_de_gestion_y_de_uso_de_la_nube_no_comparten_mensaje() {
        use crate::application::GestionNubeError;

        assert_eq!(
            mensaje_gestion_nube(GestionNubeError::OperacionNoAutorizada),
            "Sólo una sesión ROOT activa puede gestionar la nube"
        );
        assert_eq!(
            mensaje_gestion_nube(GestionNubeError::UsoNoAutorizado),
            "Su sesión no está autorizada para usar la nube"
        );
    }

    /// Mismo criterio que `el_cuerpo_crudo_de_una_respuesta_inesperada_no_llega_a_pantalla`,
    /// pero a través de la fachada que usa el puente móvil
    /// (`mobile/rust-core/src/lib.rs`, `From<GestionNubeError> for
    /// NucleoError`) -- confirma que ese camino tampoco filtra el cuerpo
    /// crudo de la respuesta del receptor.
    #[cfg(feature = "nube")]
    #[test]
    fn mensaje_gestion_nube_tampoco_filtra_el_cuerpo_crudo_de_sincronizacion() {
        use crate::application::GestionNubeError;

        let error = GestionNubeError::Sincronizacion(
            crate::nube::SincronizacionError::RespuestaInesperada {
                status: 500,
                cuerpo: "detalle interno de postgrest".into(),
            },
        );
        assert_eq!(
            mensaje_gestion_nube(error),
            "El receptor rechazó el pedido, intentá de nuevo más tarde"
        );
    }

    /// `DispositivoSuspendido`/`VersionDesactualizada` cada uno con su propio
    /// mensaje -- no deben caer los dos en el mismo texto genérico (mismo
    /// motivo que el test de arriba para gestión/uso), porque uno lo resuelve
    /// un admin y el otro se resuelve actualizando la app.
    #[cfg(feature = "nube")]
    #[test]
    fn dispositivo_suspendido_y_version_desactualizada_no_comparten_mensaje() {
        use crate::nube::NubeError;

        let suspendido = mensaje_nube(NubeError::DispositivoSuspendido);
        let desactualizada = mensaje_nube(NubeError::VersionDesactualizada);

        assert_ne!(suspendido, desactualizada);
        assert!(suspendido.contains("administrador"));
        assert!(desactualizada.contains("actualizá") || desactualizada.contains("actualiza"));
    }
}
