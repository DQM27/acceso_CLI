//! Resultado de `resolver()` — dato puro, sin ninguna dependencia de
//! terminal (ver el mismo criterio en `parser.rs`/`resolver.rs`). Vivía
//! dentro de `comandos/estado.rs` junto a `AppState` (que sí depende de
//! `tui_input`); se separó para que la GUI (`application/comandos.rs`)
//! pueda usar el lenguaje de comandos sin arrastrar `ratatui`/`crossterm`/
//! `tui-input` como dependencias compiladas.

use crate::database::queries::auditoria_contratistas::CambioContratistaAuditado;
use crate::database::queries::contratistas::ContratistaResumen;
use crate::database::queries::empresas::EmpresaResumen;
use crate::database::queries::usuarios::UsuarioResumen;
use crate::models::medio_ingreso::MedioIngreso;
use crate::services::registro_ingreso_service::{IngresoActivoResumen, PreparacionIngreso};

/// Lo que ocupa el área contextual en este momento. Se reconstruye entero cada
/// vez que cambia el input; la selección (`seleccion`) es el único estado que
/// sobrevive entre reconstrucciones, y lo hace dentro de la propia variante.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ContextState {
    /// Input vacío: título, conteo de personas dentro y comandos disponibles.
    Inicio {
        total_dentro: usize,
    },
    /// Coincidencias de contratistas para `/ingreso` o una búsqueda de texto
    /// libre. Con la consulta demasiado corta o sin resultados, `items` queda
    /// vacío y el render muestra la pista correspondiente a partir de
    /// `consulta`. `offset`/`total` habilitan PageUp/PageDown (mismo patrón
    /// que `paginar` de Historial): `total` es el conteo real que ya
    /// devuelve `buscar_contratistas`, sin límite.
    Coincidencias {
        consulta: String,
        items: Vec<ContratistaResumen>,
        seleccion: usize,
        offset: usize,
        total: usize,
    },
    /// Coincidencias de empresas para `/editar empresa <consulta>` (DEC-052)
    /// — mismo criterio que `Coincidencias`, sin selector de columnas (F4):
    /// `EmpresaResumen` tiene sólo 3 campos, una lista simple alcanza.
    /// `buscar_empresas` no devuelve un conteo total (a diferencia de
    /// contratistas): `hay_mas` viene de pedir un elemento de más
    /// (`LIMITE_COINCIDENCIAS + 1`) y comprobar si sobró, sin tocar la
    /// capa de base de datos para agregar un `COUNT` aparte.
    CoincidenciasEmpresas {
        consulta: String,
        items: Vec<EmpresaResumen>,
        seleccion: usize,
        offset: usize,
        hay_mas: bool,
    },
    /// Coincidencias de usuarios para `/editar usuario <consulta>`
    /// (DEC-052) — sólo llega acá quien tiene `Operacion::GestionarUsuarios`
    /// (`resolver_busqueda_usuarios` corta antes con `MensajeError`). Mismo
    /// truco de `hay_mas` que `CoincidenciasEmpresas`.
    CoincidenciasUsuarios {
        consulta: String,
        items: Vec<UsuarioResumen>,
        seleccion: usize,
        offset: usize,
        hay_mas: bool,
    },
    /// Coincidencias de ingresos activos para `/salida`. `descripcion` es la
    /// consulta ya formateada para el mensaje "No hay ingreso activo para …"
    /// (p. ej. `"carlos"` o `gafete 27`).
    CoincidenciasActivos {
        descripcion: String,
        items: Vec<IngresoActivoResumen>,
        seleccion: usize,
    },
    /// Tarjeta de validación previa al ingreso. `gafete_ocupante` es el
    /// ingreso activo que hoy tiene el gafete pedido, si existe.
    ResumenIngreso {
        preparacion: PreparacionIngreso,
        gafete: Option<i64>,
        medio: MedioIngreso,
        gafete_ocupante: Option<IngresoActivoResumen>,
    },
    /// Tarjeta de confirmación de salida sobre un ingreso activo concreto.
    ResumenSalida {
        activo: IngresoActivoResumen,
    },
    /// `/activos`: tabla de personas dentro ahora mismo. Navegable con ↑↓ —
    /// Enter sobre una fila lleva a `ResumenSalida` (DEC-056), mismo camino
    /// que `/salida` ya usa desde `CoincidenciasActivos`.
    TablaActivos {
        items: Vec<IngresoActivoResumen>,
        total: usize,
        seleccion: usize,
    },
    /// `/auditoria`: cambios auditados de contratistas — sólo Administrador
    /// y Root (`Operacion::VerAuditoria`, verificado tanto acá como en
    /// `AppCore::buscar_auditoria_contratistas`). Sin filtro ni exportación,
    /// a diferencia de Historial: sólo lectura paginada con PageUp/PageDown,
    /// mismo `total` real que ya trae `PaginaAuditoriaContratistas`.
    TablaAuditoria {
        items: Vec<CambioContratistaAuditado>,
        seleccion: usize,
        offset: usize,
        total: usize,
    },
    /// Búsqueda de texto libre resuelta a un contratista concreto.
    FichaContratista {
        resumen: ContratistaResumen,
    },
    /// `/cerrarsesion`: tarjeta de confirmación — Enter cierra la sesión y
    /// vuelve al login, Esc cancela.
    ConfirmarCerrarSesion,
    /// `/clave`: tarjeta de entrada — Enter abre la Surface de cambio de
    /// contraseña propia (primero pide la actual, luego la nueva).
    ConfirmarCambioPassword,
    /// `/clasico`: tarjeta de confirmación — Enter guarda la preferencia de
    /// interfaz y reinicia la aplicación en la TUI clásica; Esc cancela.
    ConfirmarModoClasico,
    /// `/nuevo` (o `/nuevo contratista`/`/n c`): tarjeta de entrada al alta
    /// — Enter abre el formulario de contratista, Esc cancela.
    NuevoContratista,
    /// `/nuevo empresa` (`/n em`): tarjeta de entrada al alta de empresa.
    NuevoEmpresa,
    /// `/nuevo usuario` (`/n u`): tarjeta de entrada al alta de usuario.
    NuevoUsuario,
    /// `/historial`: tarjeta de entrada — Enter abre la Surface de
    /// Historial (§5.2/DEC-023/024), Esc cancela.
    AbrirHistorial,
    /// `/gafete` (`/g`): tarjeta de entrada al modo de salida por gafete
    /// (DEC-057) — `texto` es lo que ya se escribió después del comando
    /// (posiblemente vacío); si al confirmar con Enter no está vacío, se
    /// procesa de una vez en el mismo paso en vez de abrir la Surface en
    /// blanco (`/gafete 2, 25` + Enter saca a los tres de un tiro).
    AbrirSalidaGafete {
        texto: String,
    },
    Ayuda,
    /// Comando desconocido, parámetro inválido o error de consulta: se muestra
    /// el mensaje con `✗` y la sugerencia de `/ayuda` cuando aplica.
    MensajeError {
        mensaje: String,
    },
}
