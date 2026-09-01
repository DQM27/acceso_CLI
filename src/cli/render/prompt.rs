//! La fila de input siempre visible: la paleta de comandos desplegable
//! (`render_prompt`) y el resaltado de sintaxis clave:valor/comando dentro
//! de la línea (`render_prompt_linea`, `segmentar_comando`, `segmentar_claves`).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::cli::estado::AppState;
use crate::cli::formulario::{ModoFormulario, Subfase};
use crate::cli::formulario_empresa::ModoFormularioEmpresa;
use crate::cli::formulario_usuario::{ModoFormularioUsuario, SubfaseUsuario};
use crate::cli::parser::{Comando, Entrada};

use super::estilos::{acento, estilo_fundido, estilo_seleccion, glifo_feedback_color, muted};

/// Ancho de la columna de nombre en la paleta de comandos — cubre el más
/// largo (`cerrarsesion`, 12 caracteres) más un espacio de separación. Con
/// el ancho fijo anterior (8) los nombres largos ("historial",
/// "cerrarsesion") quedaban pegados a la descripción sin espacio entre
/// medio — reportado en runtime real ("líneas que no están completas").
const ANCHO_NOMBRE_PALETA: usize = 13;

fn descripcion_comando(comando: Comando) -> &'static str {
    match comando {
        Comando::Ingreso => "registrar ingreso — /ingreso <nombre> G:<n> M:<medio>",
        Comando::Salida => "registrar salida — /salida <nombre> o /salida G:<n>",
        Comando::Gafete => "salida rápida por gafete — uno o varios, separados por coma",
        Comando::Activos => "quién está dentro ahora",
        Comando::Nuevo => "dar de alta — contratista (default), empresa o usuario",
        Comando::Editar => "editar — contratista (default), empresa o usuario",
        Comando::Historial => "explorar movimientos — filtro empresa/tipo/fecha…",
        Comando::Auditoria => "cambios auditados de contratistas (admin/root)",
        Comando::Ayuda => "sintaxis completa y ejemplos",
        Comando::Clave => "cambiar mi propia contraseña",
        Comando::Clasico => "reiniciar en la TUI clásica (queda como default)",
        Comando::CerrarSesion => "cerrar sesión y volver al login",
    }
}

/// Sin paleta: el prompt nunca desaparece, vive dentro de un recuadro de
/// línea fina, sin caja ni borde — texto plano sobre el fondo, mismo
/// criterio que `fzf` sin `--border` (la mayoría de sus usuarios ni lo
/// activa): ninguna Surface necesita una caja para leerse, cada una ya se
/// identifica con su propia etiqueta en mayúscula (`HISTORIAL ›`, `NUEVA
/// EMPRESA ›`…) o, en la paleta, con la lista misma. Cambia de etiqueta
/// según la fase (cédula, contraseña enmascarada, o el `>` de comandos),
/// siempre con el cursor visible.
///
/// La fila de input queda anclada **abajo** (no arriba) y lo que haya
/// encima (el desplegable de la paleta) crece hacia arriba desde ahí —
/// mismo layout que `fzf --layout=reverse-list`. `render()` calcula el alto
/// para que quepan todas las filas, y como el área de contexto de arriba
/// absorbe ese crecimiento (`Constraint::Min`), la fila de input se queda
/// fija en la misma posición sin importar cuántas coincidencias haya —
/// antes, con el input arriba, cada tecla que cambiaba el alto de la lista
/// hacía saltar el propio `>` de lugar.
pub(super) fn render_prompt(
    frame: &mut Frame,
    area: Rect,
    app: &AppState,
    paleta: Option<&[Comando]>,
) {
    let Some(comandos) = paleta else {
        let fila_input = Rect::new(area.x, area.y, area.width, 1);
        render_prompt_linea(frame, fila_input, app);
        return;
    };
    if area.height < 2 {
        let fila_input = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        );
        render_prompt_linea(frame, fila_input, app);
        return;
    }

    let fila_input = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
    render_prompt_linea(frame, fila_input, app);

    let filas_disponibles = fila_input.y.saturating_sub(area.y);
    let fila_comandos = Rect::new(area.x, area.y, area.width, filas_disponibles);
    // Fila resaltada = la que Tab/Enter completarían (`app.seleccion_paleta`,
    // movida con ↑↓ — ver `operando.rs`). Se acota por si la lista se achicó
    // al seguir escribiendo y el índice quedó desactualizado.
    let seleccionada = app.seleccion_paleta.min(comandos.len().saturating_sub(1));
    let lineas: Vec<Line<'static>> = comandos
        .iter()
        .enumerate()
        .map(|(indice, comando)| {
            let marcador = if indice == seleccionada { "› " } else { "  " };
            let nombre = format!("{marcador}/{:<ANCHO_NOMBRE_PALETA$}", comando.nombre());
            let descripcion = descripcion_comando(*comando).to_string();
            if indice == seleccionada {
                // Antes era un solo `Span` con `estilo_seleccion()` y
                // perdía toda jerarquía justo en la fila que el operador
                // tiene marcada — la que más importa distinguir. Ponerle
                // color propio a cada mitad (en vez de negrita) daría dos
                // fondos distintos dentro de la misma barra resaltada (el
                // reversed intercambia fg/bg de cada `Span` por separado);
                // negrita en el nombre mantiene una sola barra de color
                // sólida y sigue marcando la diferencia.
                Line::from(vec![
                    Span::styled(nombre, estilo_seleccion().add_modifier(Modifier::BOLD)),
                    Span::styled(descripcion, estilo_seleccion()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(nombre, acento()),
                    Span::styled(descripcion, muted()),
                ])
            }
        })
        .collect();
    // Sin fundido: la paleta aparece de una — el fundido en cascada por fila
    // que había antes (DEC-062) estorbaba en el uso real, reportado en
    // runtime.
    frame.render_widget(Paragraph::new(lineas), fila_comandos);
}

/// Claves de `/ingreso`/`/salida`/`--modificador` (DEC-021: sólo se
/// interpretan con un comando de ítem activo, nunca sobre texto libre suelto
/// — `resaltado_parametros` ya respeta eso mirando el `Entrada` resuelto).
const CLAVES_PARAMETRO: [&str; 2] = ["g", "m"];
/// Claves de `/historial` (DEC-031) — mismo vocabulario que `historial.rs`.
const CLAVES_HISTORIAL: [&str; 8] = [
    "empresa", "tipo", "estado", "gafete", "ingreso", "salida", "desde", "hasta",
];

/// Blanco: mismo criterio de "esto ya no es texto libre" en cualquier
/// input clave:valor de la app (DEC-0XX) — un color propio, distinto del
/// acento (cian, ya reservado para "foco/Surface") y de `muted()` (texto
/// secundario), para no competir con esos dos significados ya establecidos.
fn estilo_clave() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Qué reconocer como estructura (blanco+negrita) en el valor del prompt —
/// dos gramáticas distintas: Historial es siempre `clave:valor`; el prompt
/// de comandos sin Surface además tiene el propio nombre del comando y, en
/// `/nuevo`/`/editar`, la palabra-sujeto (`empresa`/`usuario`/`contratista`),
/// ninguna de las dos con `:` — de ahí que no compartan un solo mecanismo.
enum EstiloValor {
    Claves(&'static [&'static str]),
    Comando,
}

fn segmentar(texto: &str, estilo: &EstiloValor) -> Vec<(String, bool)> {
    match estilo {
        EstiloValor::Claves(claves) => segmentar_claves(texto, claves),
        EstiloValor::Comando => segmentar_comando(texto),
    }
}

/// Mismos alias que `resolver::sujeto_nuevo` (DEC-045) — duplicados a
/// propósito: ese resolutor decide qué se crea, éste sólo decide qué
/// palabra pintar en blanco; si la lista de alias cambia allá, hay que
/// repetir el cambio acá.
fn sujeto_nuevo_valido(consulta: &str) -> bool {
    matches!(
        consulta.trim().to_lowercase().as_str(),
        "contratista" | "c" | "empresa" | "em" | "emp" | "usuario" | "u"
    )
}

/// Mismos alias que `resolver::sujeto_editar` (DEC-052) — sólo el primer
/// token de la consulta cuenta como sujeto, el resto ya es la búsqueda.
fn sujeto_editar_es_palabra_clave(consulta: &str) -> bool {
    let primero = consulta.split_whitespace().next().unwrap_or_default();
    matches!(
        primero.to_lowercase().as_str(),
        "empresa" | "em" | "emp" | "usuario" | "u"
    )
}

/// Resalta, en el prompt de comandos sin Surface: el nombre del comando ya
/// reconocido (líder `/algo` o `--modificador` sobre un comando de ítem,
/// DEC-021) y, si aplica, la palabra-sujeto de `/nuevo`/`/editar` — más
/// `G:`/`M:` con el mismo mecanismo de `segmentar_claves`. Con texto que no
/// resolvió a ningún comando (búsqueda libre, comando desconocido, vacío),
/// nada se resalta: no hay estructura que señalar todavía.
fn segmentar_comando(texto: &str) -> Vec<(String, bool)> {
    let Entrada::Comando {
        comando, consulta, ..
    } = crate::cli::parser::parsear(texto)
    else {
        return vec![(texto.to_string(), false)];
    };
    // `None` en vez de `false` para Ingreso/Salida/etc.: ahí la primera
    // palabra tras el líder es la búsqueda (o un `G:`/`M:`), no una posición
    // de sujeto — sin esta distinción, un `G:27` justo después del líder
    // caía en la rama de "sujeto" (con `false`) y nunca llegaba a
    // `clave_de_token`, perdiendo el resaltado de parámetro.
    let sujeto_tras_lider: Option<bool> = match comando {
        Comando::Nuevo => Some(sujeto_nuevo_valido(&consulta)),
        Comando::Editar => Some(sujeto_editar_es_palabra_clave(&consulta)),
        _ => None,
    };

    let mut segmentos = Vec::new();
    let mut resto = texto;
    let mut lider_visto = false;
    let mut primero_tras_lider = false;
    while !resto.is_empty() {
        let espacios = resto.len() - resto.trim_start().len();
        if espacios > 0 {
            segmentos.push((resto[..espacios].to_string(), false));
            resto = &resto[espacios..];
            continue;
        }
        let fin_palabra = resto.find(char::is_whitespace).unwrap_or(resto.len());
        let palabra = &resto[..fin_palabra];
        let es_lider = !lider_visto
            && (palabra
                .strip_prefix('/')
                .is_some_and(|nombre| Comando::desde_texto(nombre) == Some(comando))
                || palabra
                    .strip_prefix("--")
                    .is_some_and(|nombre| Comando::desde_texto(nombre) == Some(comando)));
        if es_lider {
            lider_visto = true;
            primero_tras_lider = sujeto_tras_lider.is_some();
            segmentos.push((palabra.to_string(), true));
        } else if primero_tras_lider {
            primero_tras_lider = false;
            segmentos.push((palabra.to_string(), sujeto_tras_lider.unwrap_or(false)));
        } else if let Some(fin_clave) = clave_de_token(palabra, &CLAVES_PARAMETRO) {
            segmentos.push((palabra[..fin_clave].to_uppercase(), true));
            if fin_clave < palabra.len() {
                segmentos.push((palabra[fin_clave..].to_string(), false));
            }
        } else {
            segmentos.push((palabra.to_string(), false));
        }
        resto = &resto[fin_palabra..];
    }
    segmentos
}

/// Si `token` (con o sin `-` de negación) es `clave:...` para alguna de
/// `claves` (sin distinguir mayúsculas), el largo de esa porción inicial
/// (negación + clave + `:`) — el resto (el valor) no se toca.
fn clave_de_token(token: &str, claves: &[&str]) -> Option<usize> {
    let (largo_negacion, cuerpo) = match token.strip_prefix('-') {
        Some(resto) => (1, resto),
        None => (0, token),
    };
    let (clave, _) = cuerpo.split_once(':')?;
    if clave.is_empty() {
        return None;
    }
    claves
        .iter()
        .any(|c| c.eq_ignore_ascii_case(clave))
        .then_some(largo_negacion + clave.len() + 1)
}

/// Divide `texto` en segmentos `(texto, es_clave)` en el mismo orden y con
/// el mismo largo total que el original (para no correr el cálculo del
/// cursor, que trabaja en caracteres): cada `clave:` reconocida se separa y
/// se pasa a mayúscula para mostrarse en blanco; el resto — espacios,
/// valores, texto libre — queda intacto. No valida el valor (eso lo sigue
/// haciendo `parser::clasificar_token`/`historial::aplicar_clave`): esto es
/// sólo la señal visual de "esto ya no es texto libre".
fn segmentar_claves(texto: &str, claves: &[&str]) -> Vec<(String, bool)> {
    let mut segmentos = Vec::new();
    let mut resto = texto;
    while !resto.is_empty() {
        let espacios = resto.len() - resto.trim_start().len();
        if espacios > 0 {
            segmentos.push((resto[..espacios].to_string(), false));
            resto = &resto[espacios..];
            continue;
        }
        let fin_palabra = resto.find(char::is_whitespace).unwrap_or(resto.len());
        let palabra = &resto[..fin_palabra];
        match clave_de_token(palabra, claves) {
            Some(fin_clave) => {
                segmentos.push((palabra[..fin_clave].to_uppercase(), true));
                if fin_clave < palabra.len() {
                    segmentos.push((palabra[fin_clave..].to_string(), false));
                }
            }
            None => segmentos.push((palabra.to_string(), false)),
        }
        resto = &resto[fin_palabra..];
    }
    segmentos
}

/// Arma los `Span` del valor del prompt a partir de los segmentos de
/// `segmentar_claves` (o un único segmento sin resaltar si `segmentos` es
/// `None`), insertando la celda resaltada del cursor en `columna` cuando
/// corresponde — mismo cursor propio de siempre (nunca el del terminal),
/// ahora consciente de que puede caer en medio de un segmento coloreado.
fn spans_valor(
    visible: &str,
    segmentos: Option<Vec<(String, bool)>>,
    columna_cursor: Option<usize>,
    mostrar_cursor: bool,
) -> Vec<Span<'static>> {
    let piezas = segmentos.unwrap_or_else(|| vec![(visible.to_string(), false)]);
    let estilizar = |texto: String, es_clave: bool| -> Span<'static> {
        if es_clave {
            Span::styled(texto, estilo_clave())
        } else {
            Span::raw(texto)
        }
    };
    let Some(columna) = columna_cursor else {
        return piezas
            .into_iter()
            .map(|(texto, es_clave)| estilizar(texto, es_clave))
            .collect();
    };
    let mut spans = Vec::new();
    let mut offset = 0usize;
    let mut insertado = false;
    for (texto, es_clave) in piezas {
        let largo = texto.chars().count();
        if insertado || columna >= offset + largo {
            spans.push(estilizar(texto, es_clave));
            offset += largo;
            continue;
        }
        let local = columna - offset;
        let indice_byte = texto
            .char_indices()
            .nth(local)
            .map_or(texto.len(), |(i, _)| i);
        let (antes, resto) = texto.split_at(indice_byte);
        let mut caracteres = resto.chars();
        let bajo_cursor = caracteres.next().map(String::from).unwrap_or_default();
        let despues: String = caracteres.collect();
        if !antes.is_empty() {
            spans.push(estilizar(antes.to_string(), es_clave));
        }
        // Mitad "apagada" del parpadeo: el carácter se ve normal, sin la
        // celda resaltada — mismo criterio que cualquier cursor de terminal,
        // sólo que el nuestro nunca desaparece del todo (ver `blink_on`).
        spans.push(if mostrar_cursor {
            Span::styled(
                bajo_cursor,
                Style::default().add_modifier(Modifier::REVERSED),
            )
        } else {
            estilizar(bajo_cursor, es_clave)
        });
        if !despues.is_empty() {
            spans.push(estilizar(despues, es_clave));
        }
        insertado = true;
        offset += largo;
    }
    if !insertado && mostrar_cursor {
        spans.push(Span::styled(
            " ".to_string(),
            Style::default().add_modifier(Modifier::REVERSED),
        ));
    }
    spans
}

/// Sólo la línea de texto del input (etiqueta + valor + cursor), sin marco —
/// el marco lo pone `render_prompt`, que reutiliza esto tanto con paleta
/// como sin ella. Sólo se llama en fase `Operando`: el login tiene su propia
/// composición sin caja (`render_login`), nunca pasa por acá.
fn render_prompt_linea(frame: &mut Frame, area: Rect, app: &AppState) {
    // `editable` sólo decide DE DÓNDE sale la posición del cursor (del
    // `Input` real mientras se escribe, o del final del texto congelado
    // cuando no) — ya no si el cursor se ve. Antes, con una Surface
    // mostrando resultado (Historial ya aplicado, un campo no-texto de un
    // formulario…) el cursor desaparecía del todo y no quedaba ninguna
    // señal de que la línea seguía activa (reportado en runtime real:
    // "no se sabe dónde se puede escribir"). Como cualquier tecla de texto
    // vuelve a poner esa Surface en edición (ver los controladores), seguía
    // siendo verdad que "se puede escribir ahí".
    let (etiqueta, valor, editable, cursor_chars, resaltado) = if let Some(destino) = app
        .historial
        .as_ref()
        .and_then(|h| h.exportacion_destino.as_ref())
    {
        // F5: el input edita la ruta de exportación — propio, no
        // `app.input`, para no pisar el filtro congelado detrás.
        (
            "destino xlsx › ".to_string(),
            destino.value().to_string(),
            true,
            destino.visual_cursor(),
            None,
        )
    } else if let Some(historial) = &app.historial {
        // Editando: el input escribe el filtro clave:valor. Mostrando
        // resultado (Enter ya aplicó, DEC-024): el texto queda congelado
        // hasta Esc, sin cursor — no se edita mientras se navega. En los dos
        // casos las claves reconocidas (`empresa:`, `tipo:`…) se resaltan
        // igual, tecleando o ya aplicadas.
        let editable = historial.resultado.is_none();
        (
            "HISTORIAL › ".to_string(),
            app.input.value().to_string(),
            editable,
            if editable {
                app.input.visual_cursor()
            } else {
                app.input.value().chars().count()
            },
            Some(EstiloValor::Claves(&CLAVES_HISTORIAL)),
        )
    } else if let Some(empresa) = &app.formulario_empresa {
        let etiqueta = match empresa.modo {
            ModoFormularioEmpresa::Nuevo => "NUEVA EMPRESA › ".to_string(),
            ModoFormularioEmpresa::Editar { .. } => "EDITAR EMPRESA › ".to_string(),
        };
        (
            etiqueta,
            app.input.value().to_string(),
            true,
            app.input.visual_cursor(),
            None,
        )
    } else if let Some(fu) = &app.formulario_usuario {
        // La etiqueta identifica la Surface (mismo nombre en Editando y
        // Resumen) en vez de cambiar por campo — antes mutaba a "cédula › ",
        // "nombre › "… en cada campo, duplicando (con retraso, lejos del
        // foco real) la misma señal que ya da el glifo por fila arriba, y
        // encima le hacía perder de vista en qué formulario estaba.
        let etiqueta = match fu.modo {
            ModoFormularioUsuario::Nuevo => "NUEVO USUARIO › ".to_string(),
            ModoFormularioUsuario::Editar { .. } => "EDITAR USUARIO › ".to_string(),
        };
        let editable = matches!(fu.subfase, SubfaseUsuario::Editando) && fu.campo.es_texto();
        // Password/Confirmar se enmascaran también acá — no sólo en el
        // área de contenido — nunca se ve la contraseña en texto plano.
        let valor = if editable && fu.campo.es_secreto() {
            "•".repeat(app.input.value().chars().count())
        } else {
            app.input.value().to_string()
        };
        (
            etiqueta,
            valor,
            editable,
            if editable {
                app.input.visual_cursor()
            } else {
                app.input.value().chars().count()
            },
            None,
        )
    } else if app.formulario_password.is_some() {
        // Los tres campos (Actual/Nueva/Confirmar) son secretos siempre —
        // a diferencia de `formulario_usuario` no hace falta mirar cuál
        // está activo, acá nunca hay un campo de texto plano que mostrar.
        (
            "CAMBIAR CONTRASEÑA › ".to_string(),
            "•".repeat(app.input.value().chars().count()),
            true,
            app.input.visual_cursor(),
            None,
        )
    } else if app.salida_gafete.is_some() {
        (
            "GAFETE › ".to_string(),
            app.input.value().to_string(),
            true,
            app.input.visual_cursor(),
            None,
        )
    } else {
        match &app.formulario {
            // La etiqueta identifica la Surface (mismo nombre en cualquier
            // subfase — Editando, EligiendoEmpresa o Resumen) en vez de
            // cambiar por campo: antes mutaba a "cédula › ", "empresa › ",
            // "confirmar › "… y esa señal, lejos del campo real (arriba,
            // marcado por su propio glifo), confundía más de lo que
            // aclaraba — el operador termina sin saber en qué formulario
            // está. Editando vs. Resumen (antes distinguidos por el color
            // del borde, ya sin recuadro) los sigue diferenciando el área de
            // contexto: `Subfase::Resumen` es la única que muestra la
            // tarjeta de revisión completa antes de guardar.
            Some(formulario) => {
                let etiqueta = match formulario.modo {
                    ModoFormulario::Nuevo => "NUEVO CONTRATISTA › ".to_string(),
                    ModoFormulario::Editar { .. } => "EDITAR CONTRATISTA › ".to_string(),
                };
                let editable = matches!(formulario.subfase, Subfase::EligiendoEmpresa { .. })
                    || formulario.campo.es_texto();
                (
                    etiqueta,
                    app.input.value().to_string(),
                    editable,
                    if editable {
                        app.input.visual_cursor()
                    } else {
                        app.input.value().chars().count()
                    },
                    None,
                )
            }
            None => (
                "> ".to_string(),
                app.input.value().to_string(),
                true,
                app.input.visual_cursor(),
                Some(EstiloValor::Comando),
            ),
        }
    };

    // El `> ` de la línea de comandos (ninguna Surface abierta) muta al
    // símbolo del feedback vigente mientras dure — DEC-060. Mismo largo
    // (2 caracteres) que "> ", así que no afecta el cálculo de ancho de
    // abajo. Sólo en ese caso puntual: los demás prompts (`gafete › `,
    // `historial › `…) ya llevan su propia etiqueta descriptiva, sin un
    // símbolo suelto que reemplazar.
    let opacidad_glifo = app.presentacion.opacidad("prompt_glifo");
    let (etiqueta, estilo_etiqueta) = match (etiqueta.as_str(), app.feedback_vigente()) {
        ("> ", Some(feedback)) => {
            let (simbolo, color) = glifo_feedback_color(feedback.nivel);
            (
                format!("{simbolo} "),
                estilo_fundido(color, opacidad_glifo, Modifier::empty()),
            )
        }
        _ => (etiqueta, acento()),
    };

    let ancho_etiqueta = etiqueta.chars().count() as u16;
    let viewport = area.width.saturating_sub(ancho_etiqueta + 1) as usize;
    // El scroll se calcula sobre el `Input` que de verdad gobierna el
    // cursor en este frame — con la exportación abierta es
    // `exportacion_destino`, no `app.input` (que sigue congelado detrás).
    let scroll = match (
        editable,
        app.historial
            .as_ref()
            .and_then(|h| h.exportacion_destino.as_ref()),
    ) {
        (false, _) => 0,
        (true, Some(destino)) => destino.visual_scroll(viewport),
        (true, None) => app.input.visual_scroll(viewport),
    };
    let visible: String = valor.chars().skip(scroll).take(viewport).collect();
    let segmentos = resaltado.map(|estilo| segmentar(&visible, &estilo));

    // Cursor propio (celda resaltada), nunca el bloque real del terminal —
    // el cursor real de cada emulador de terminal parpadea y se reposiciona
    // con su propio timing, fuera de nuestro control, y se veía
    // aparecer/desaparecer de forma inconsistente entre terminales. Acá el
    // cursor puede estar a mitad del texto (←/→/Home/End de `tui_input`),
    // así que se resalta el carácter bajo el cursor en vez de insertar un
    // "_" que correría el resto. Siempre presente (nunca `None`) — con o
    // sin Surface, editable o congelado — y parpadea con `blink_on` en vez
    // de desaparecer del todo, para que la línea nunca se lea como "inerte".
    let columna_cursor = Some(cursor_chars.saturating_sub(scroll).min(viewport));
    let mut spans = vec![Span::styled(etiqueta, estilo_etiqueta)];
    spans.extend(spans_valor(
        &visible,
        segmentos,
        columna_cursor,
        super::blink_on(app),
    ));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clave_de_token_reconoce_clave_conocida_sin_distinguir_mayusculas() {
        assert_eq!(clave_de_token("EMPRESA:bac", &CLAVES_HISTORIAL), Some(8));
        assert_eq!(clave_de_token("empresa:bac", &CLAVES_HISTORIAL), Some(8));
    }

    #[test]
    fn clave_de_token_respeta_la_negacion() {
        assert_eq!(
            clave_de_token("-estado:cerrados", &CLAVES_HISTORIAL),
            Some(8)
        );
    }

    #[test]
    fn clave_de_token_ignora_clave_desconocida_o_sin_dos_puntos() {
        assert_eq!(clave_de_token("xyz:algo", &CLAVES_HISTORIAL), None);
        assert_eq!(clave_de_token("texto libre", &CLAVES_HISTORIAL), None);
    }

    #[test]
    fn segmentar_claves_mayuscula_solo_la_clave_y_conserva_el_valor() {
        let segmentos = segmentar_claves("empresa:bac", &CLAVES_HISTORIAL);
        assert_eq!(
            segmentos,
            vec![("EMPRESA:".to_string(), true), ("bac".to_string(), false),]
        );
    }

    #[test]
    fn segmentar_claves_preserva_espacios_y_texto_libre() {
        let segmentos = segmentar_claves("Ana g:25 Perez", &CLAVES_PARAMETRO);
        assert_eq!(
            segmentos,
            vec![
                ("Ana".to_string(), false),
                (" ".to_string(), false),
                ("G:".to_string(), true),
                ("25".to_string(), false),
                (" ".to_string(), false),
                ("Perez".to_string(), false),
            ]
        );
    }

    #[test]
    fn segmentar_claves_mismo_largo_total_que_el_original() {
        let texto = "-tipo:praind,swat desde:01/08/2026 texto suelto";
        let segmentos = segmentar_claves(texto, &CLAVES_HISTORIAL);
        let largo: usize = segmentos.iter().map(|(s, _)| s.chars().count()).sum();
        assert_eq!(largo, texto.chars().count());
    }

    #[test]
    fn segmentar_comando_resalta_lider_y_parametro() {
        let segmentos = segmentar_comando("/ingreso Carlos G:27");
        assert_eq!(
            segmentos,
            vec![
                ("/ingreso".to_string(), true),
                (" ".to_string(), false),
                ("Carlos".to_string(), false),
                (" ".to_string(), false),
                ("G:".to_string(), true),
                ("27".to_string(), false),
            ]
        );
    }

    #[test]
    fn segmentar_comando_resalta_el_modificador_no_solo_el_lider() {
        let segmentos = segmentar_comando("Ana --i G:27");
        assert_eq!(
            segmentos,
            vec![
                ("Ana".to_string(), false),
                (" ".to_string(), false),
                ("--i".to_string(), true),
                (" ".to_string(), false),
                ("G:".to_string(), true),
                ("27".to_string(), false),
            ]
        );
    }

    #[test]
    fn segmentar_comando_no_resalta_parametro_sobre_texto_libre_sin_modificador() {
        // DEC-021: sin comando de ítem activo, G: es literal.
        let segmentos = segmentar_comando("Ana G:27");
        assert_eq!(segmentos, vec![("Ana G:27".to_string(), false)]);
    }

    #[test]
    fn segmentar_comando_resalta_sujeto_valido_de_nuevo_y_editar() {
        assert_eq!(
            segmentar_comando("/nuevo empresa"),
            vec![
                ("/nuevo".to_string(), true),
                (" ".to_string(), false),
                ("empresa".to_string(), true),
            ]
        );
        assert_eq!(
            segmentar_comando("/editar usuario Ana"),
            vec![
                ("/editar".to_string(), true),
                (" ".to_string(), false),
                ("usuario".to_string(), true),
                (" ".to_string(), false),
                ("Ana".to_string(), false),
            ]
        );
    }

    #[test]
    fn segmentar_comando_no_resalta_nombre_de_contratista_en_editar() {
        // Sin sujeto (default Contratista): la primera palabra es la
        // búsqueda, no una palabra-clave — no debe resaltarse.
        assert_eq!(
            segmentar_comando("/editar Carlos"),
            vec![
                ("/editar".to_string(), true),
                (" ".to_string(), false),
                ("Carlos".to_string(), false),
            ]
        );
    }
}
