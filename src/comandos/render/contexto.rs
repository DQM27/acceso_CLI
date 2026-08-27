//! Despacha el área de contexto según el `ContextState` vigente — el
//! "cuerpo" que muta bajo el prompt siempre visible — y agrupa las
//! pantallas estáticas de confirmación (Inicio, cerrar sesión, alta de
//! contratista/empresa/usuario, abrir Historial/gafete).

use ratatui::text::{Line, Span};

use crate::comandos::columnas::{ColumnaActivos, ColumnaBusqueda, SelectorColumnas};
use crate::comandos::estado::ContextState;

use super::activos::{
    lineas_coincidencias_activos, lineas_ficha, lineas_resumen_ingreso, lineas_resumen_salida,
    lineas_tabla_activos,
};
use super::auditoria::lineas_tabla_auditoria;
use super::ayuda::lineas_ayuda;
use super::busqueda::{
    lineas_coincidencias, lineas_coincidencias_empresas, lineas_coincidencias_usuarios,
};
use super::estilos::{acento, estilo_error, muted};
use super::util::cantidad_personas;

/// Las cinco listas navegables (`Coincidencias*`/`TablaActivos`) empiezan
/// con 2 líneas fijas de encabezado (fila de columnas + divisor, o título +
/// línea en blanco para Empresas/Usuarios) antes de la primera fila de
/// ítem — de ahí sale el índice absoluto de la línea resaltada dentro del
/// `Vec` que devuelve cada `lineas_coincidencias*`/`lineas_tabla_activos`.
/// Si algún día cambia el preámbulo de alguna de esas funciones, hay que
/// actualizar también este número (no hay forma de derivarlo automáticamente
/// sin que cada función devuelva su propio índice).
const FILAS_ANTES_DE_ITEMS: usize = 2;

/// Índice absoluto (dentro del `Vec<Line>` ya armado) de la fila resaltada
/// — `None` cuando no hay lista (o está vacía, nada que mantener visible).
/// Lo usa `render()` para calcular el scroll y que ↓ nunca empuje la
/// selección fuera del área visible (antes se perdía de vista sin más aviso
/// que "la flecha ya no se mueve", con listas que ahora pueden traer hasta
/// `LIMITE_COINCIDENCIAS` = 50 filas).
fn indice_seleccion_lista(hay_items: bool, seleccion: usize) -> Option<usize> {
    hay_items.then_some(FILAS_ANTES_DE_ITEMS + seleccion)
}

/// `scroll.y` del `Paragraph` del área de contexto: el mínimo necesario
/// para que la línea `seleccionada` (índice absoluto) quede dentro de las
/// últimas `alto` filas visibles. Sin selección (`None`, pantallas sin
/// lista navegable) no hay nada que perseguir, arranca en 0 como siempre.
pub(super) fn scroll_hacia_seleccion(seleccionada: Option<usize>, alto: u16) -> u16 {
    let Some(seleccionada) = seleccionada else {
        return 0;
    };
    let alto = alto as usize;
    if alto == 0 || seleccionada < alto {
        return 0;
    }
    (seleccionada - alto + 1) as u16
}

pub(super) fn lineas_contexto(
    contexto: &ContextState,
    ancho: u16,
    columnas_busqueda: &SelectorColumnas<ColumnaBusqueda>,
    columnas_activos: &SelectorColumnas<ColumnaActivos>,
) -> (Vec<Line<'static>>, Option<usize>) {
    match contexto {
        ContextState::Inicio { total_dentro } => (lineas_inicio(*total_dentro), None),
        ContextState::Coincidencias {
            consulta,
            items,
            seleccion,
            offset,
            total,
        } => (
            lineas_coincidencias(
                consulta,
                items,
                *seleccion,
                *offset,
                *total,
                ancho,
                columnas_busqueda,
            ),
            indice_seleccion_lista(!items.is_empty(), *seleccion),
        ),
        ContextState::CoincidenciasActivos {
            descripcion,
            items,
            seleccion,
        } => (
            lineas_coincidencias_activos(descripcion, items, *seleccion, ancho, columnas_activos),
            indice_seleccion_lista(!items.is_empty(), *seleccion),
        ),
        ContextState::CoincidenciasEmpresas {
            consulta,
            items,
            seleccion,
            offset,
            hay_mas,
        } => (
            lineas_coincidencias_empresas(consulta, items, *seleccion, *offset, *hay_mas),
            indice_seleccion_lista(!items.is_empty(), *seleccion),
        ),
        ContextState::CoincidenciasUsuarios {
            consulta,
            items,
            seleccion,
            offset,
            hay_mas,
        } => (
            lineas_coincidencias_usuarios(consulta, items, *seleccion, *offset, *hay_mas),
            indice_seleccion_lista(!items.is_empty(), *seleccion),
        ),
        ContextState::ResumenIngreso { .. } => (lineas_resumen_ingreso(contexto), None),
        ContextState::ResumenSalida { activo } => (lineas_resumen_salida(activo), None),
        ContextState::TablaActivos {
            items,
            total,
            seleccion,
        } => (
            lineas_tabla_activos(items, *total, *seleccion, ancho, columnas_activos),
            indice_seleccion_lista(!items.is_empty(), *seleccion),
        ),
        ContextState::FichaContratista { resumen } => (lineas_ficha(resumen), None),
        ContextState::TablaAuditoria {
            items,
            seleccion,
            total,
            ..
        } => (
            lineas_tabla_auditoria(items, *total, *seleccion, ancho),
            indice_seleccion_lista(!items.is_empty(), *seleccion),
        ),
        ContextState::ConfirmarCerrarSesion => (lineas_cerrar_sesion(), None),
        ContextState::ConfirmarCambioPassword => (lineas_cambio_password(), None),
        ContextState::ConfirmarModoClasico => (lineas_modo_clasico(), None),
        ContextState::NuevoContratista => (lineas_nuevo_contratista(), None),
        ContextState::NuevoEmpresa => (lineas_nuevo_empresa(), None),
        ContextState::NuevoUsuario => (lineas_nuevo_usuario(), None),
        ContextState::AbrirHistorial => (lineas_abrir_historial(), None),
        ContextState::AbrirSalidaGafete { texto } => (lineas_abrir_salida_gafete(texto), None),
        ContextState::Ayuda => (lineas_ayuda(), None),
        ContextState::MensajeError { mensaje } => (
            vec![
                Line::from(""),
                Line::from(Span::styled(format!("✗ {mensaje}"), estilo_error())),
            ],
            None,
        ),
    }
}

// El nombre de la app ya lo dice el título de la ventana (`BRISAS CLI` en
// la barra) — repetirlo acá era ruido, no identidad: esta pantalla es la
// única señal de vida real cuando el input está vacío.
fn lineas_inicio(total_dentro: usize) -> Vec<Line<'static>> {
    vec![Line::from(format!(
        "{} actualmente dentro",
        cantidad_personas(total_dentro)
    ))]
}

fn lineas_cerrar_sesion() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("CERRAR SESIÓN", muted())),
        Line::from(""),
        Line::from("La sesión actual se cerrará y volverá a la pantalla de autenticación."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para cerrar sesión · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_cambio_password() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("CAMBIAR CONTRASEÑA", muted())),
        Line::from(""),
        Line::from("Digite su contraseña."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para continuar · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_modo_clasico() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("MODO CLÁSICO", muted())),
        Line::from(""),
        Line::from("La aplicación se reiniciará en la TUI clásica, que quedará"),
        Line::from("como interfaz por defecto."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para reiniciar · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_nuevo_contratista() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("NUEVO CONTRATISTA", muted())),
        Line::from(""),
        Line::from("Se abrirá el formulario de alta: cédula, nombre, empresa, tipo,"),
        Line::from("fecha PRAIND, personal de ruta y acceso."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir el formulario · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_nuevo_empresa() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("NUEVA EMPRESA", muted())),
        Line::from(""),
        Line::from("Se abrirá el alta de empresa: sólo el nombre."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir el formulario · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_nuevo_usuario() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("NUEVO USUARIO", muted())),
        Line::from(""),
        Line::from("Se abrirá el formulario de alta: cédula, nombre, rol y contraseña."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir el formulario · Esc para cancelar",
            acento(),
        )),
    ]
}

fn lineas_abrir_historial() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("HISTORIAL", muted())),
        Line::from(""),
        Line::from("Se abrirá el explorador de movimientos: filtro clave:valor"),
        Line::from("(empresa/tipo/estado/gafete/ingreso/salida/desde/hasta)."),
        Line::from(""),
        Line::from(Span::styled(
            "ENTER para abrir · Esc para cancelar",
            acento(),
        )),
    ]
}

/// `texto` es lo que ya se escribió después de `/gafete` — si no está
/// vacío, Enter lo procesa de una vez (DEC-057), así que la tarjeta lo
/// anticipa en vez de mostrar el mismo texto genérico siempre.
fn lineas_abrir_salida_gafete(texto: &str) -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled("SALIDA POR GAFETE", muted())),
        Line::from(""),
        Line::from("Se abrirá el modo de salida rápida: sólo números de gafete,"),
        Line::from("uno o varios separados por coma. Queda abierto para el siguiente."),
        Line::from(""),
    ];
    if texto.trim().is_empty() {
        lineas.push(Line::from(Span::styled(
            "ENTER para abrir · Esc para cancelar",
            acento(),
        )));
    } else {
        lineas.push(Line::from(Span::styled(
            format!("ENTER registra la salida de: {}", texto.trim()),
            acento(),
        )));
    }
    lineas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_no_se_mueve_sin_seleccion_o_si_ya_entra_en_pantalla() {
        assert_eq!(scroll_hacia_seleccion(None, 20), 0);
        // Fila 5 (índice) cabe de sobra en un área de 20 filas.
        assert_eq!(scroll_hacia_seleccion(Some(5), 20), 0);
        // Justo la última fila visible (índice 19 en un área de 20): sigue
        // sin hacer falta scroll.
        assert_eq!(scroll_hacia_seleccion(Some(19), 20), 0);
    }

    #[test]
    fn scroll_sigue_la_seleccion_cuando_se_va_de_la_pantalla() {
        // Índice 25 en un área de 20 filas: hay que correr 6 para que la
        // fila 25 sea la última visible (filas 6..=25).
        assert_eq!(scroll_hacia_seleccion(Some(25), 20), 6);
    }

    #[test]
    fn scroll_con_area_de_alto_cero_no_paniquea() {
        assert_eq!(scroll_hacia_seleccion(Some(3), 0), 0);
    }

    #[test]
    fn indice_seleccion_lista_none_sin_items() {
        assert_eq!(indice_seleccion_lista(false, 0), None);
        assert_eq!(
            indice_seleccion_lista(true, 3),
            Some(FILAS_ANTES_DE_ITEMS + 3)
        );
    }
}
