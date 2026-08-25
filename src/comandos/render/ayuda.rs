//! Pantalla de `/ayuda`: sintaxis completa agrupada por categoría
//! (frecuentes/gestión/historial/sistema/sintaxis y atajos) — progressive
//! disclosure, misma guía que las CLIs de referencia (clig.dev/bettercli.org).

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::estilos::{acento, muted};

/// Ancho fijo de la columna de sintaxis — igual en todas las secciones para
/// que las descripciones queden alineadas de punta a punta de la pantalla,
/// no sólo dentro de cada bloque.
const ANCHO_SINTAXIS_AYUDA: usize = 34;

/// Un encabezado de sección + sus filas `(sintaxis, descripción)`. Agrupar
/// por categoría (en vez de una lista plana de 18 filas, como era antes)
/// sigue la misma guía que ya usan las CLIs de referencia — agrupar por
/// categoría lógica y dejar la sintaxis avanzada aparte de los comandos en
/// sí ("progressive disclosure", clig.dev / bettercli.org) — y de paso
/// refleja en la propia ayuda la distinción frecuente/ocasional que ya
/// rige el resto del diseño (§5.1), en vez de esconderla en una lista sin
/// jerarquía.
fn seccion_ayuda(lineas: &mut Vec<Line<'static>>, titulo: &str, filas: &[(&str, &str)]) {
    // El encabezado va en negrita simple (sin color) — un peso más que la
    // sintaxis de cada fila (acento) y dos más que su descripción (muted),
    // para que la jerarquía se lea de un vistazo: título > comando >
    // explicación.
    lineas.push(Line::from(Span::styled(
        titulo.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for (sintaxis, descripcion) in filas {
        lineas.push(Line::from(vec![
            Span::styled(format!("  {sintaxis:<ANCHO_SINTAXIS_AYUDA$}"), acento()),
            Span::styled(descripcion.to_string(), muted()),
        ]));
    }
    lineas.push(Line::from(""));
}

pub(super) fn lineas_ayuda() -> Vec<Line<'static>> {
    let mut lineas = vec![
        Line::from(Span::styled(
            "AYUDA — sintaxis de comandos",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    seccion_ayuda(
        &mut lineas,
        "FRECUENTES",
        &[
            ("/ingreso <nombre> G:<n> M:<medio>", "registrar un ingreso"),
            ("/ingreso 119430546 G:12", "también por cédula"),
            ("/salida <nombre>", "registrar salida por nombre"),
            ("/salida G:27", "registrar salida por gafete"),
            (
                "/gafete 2, 25, 85",
                "salida rápida de uno o varios gafetes (alias /g)",
            ),
            ("/activos", "tabla de personas dentro, ↑↓ Enter da salida"),
            ("texto sin /", "búsqueda de contratistas por cédula/nombre"),
        ],
    );

    seccion_ayuda(
        &mut lineas,
        "GESTIÓN",
        &[
            ("/nuevo", "dar de alta un contratista (default)"),
            ("/nuevo empresa", "dar de alta una empresa (alias /n em)"),
            (
                "/nuevo usuario",
                "dar de alta un usuario (alias /n u, requiere permiso)",
            ),
            ("/editar <nombre>", "editar un contratista (default)"),
            (
                "/editar empresa <nombre>",
                "editar una empresa (alias /e em)",
            ),
            (
                "/editar usuario <cédula|nombre>",
                "editar un usuario (alias /e u, requiere permiso)",
            ),
        ],
    );

    seccion_ayuda(
        &mut lineas,
        "HISTORIAL",
        &[
            ("/historial", "explorar movimientos (alias /h)"),
            (
                "empresa:x tipo:a,b -salida:ana",
                "filtro clave:valor · listas con coma · negable con -",
            ),
            ("F5 con resultados", "exportar el filtro completo a XLSX"),
        ],
    );

    seccion_ayuda(
        &mut lineas,
        "SISTEMA",
        &[
            ("/ayuda", "esta ayuda"),
            ("/cerrarsesion", "cerrar sesión y volver al login"),
        ],
    );

    seccion_ayuda(
        &mut lineas,
        "SINTAXIS Y ATAJOS",
        &[
            (
                "<nombre> --i G:<n> M:<medio>",
                "atajo: mismo resultado que /ingreso, /salida o /editar",
            ),
            (
                "G: gafete · M: caminando|vehiculo",
                "un solo valor cada uno, sin lista ni negación (eso es sólo de /historial)",
            ),
            ("Alias", "/i /s /g /a /n /e /h /cs"),
            ("F4 sobre una tabla", "elegir qué columnas mostrar"),
            ("Tab", "completa comandos, gafetes libres y medios"),
            ("Esc · Ctrl+C", "limpia el input · sale de la app"),
            ("Ctrl+Q", "atajo de /cerrarsesion — Enter la confirma"),
        ],
    );

    // La última sección ya deja una línea en blanco de más (el mismo
    // separador entre bloques): se recorta para no dejar aire de sobra al
    // final de la pantalla.
    lineas.pop();
    lineas
}
