//! Cruza la entrada parseada con `AppCore` y deriva el [`ContextState`].
//!
//! Todo es síncrono a propósito: SQLite local responde en microsegundos y el
//! patrón vigente del proyecto ya usa `AppCore` desde el hilo del event loop
//! (no es `Clone` ni `Sync`). Meter async o hilos aquí sólo añadiría canales y
//! estados intermedios sin ganancia medible.

use crate::application::AppCore;
use crate::database::queries::Igualdad;
use crate::database::queries::contratistas::{ContratistaResumen, FiltroContratistas};
use crate::database::queries::empresas::FiltroEmpresas;
use crate::database::queries::ingresos::FiltroIngresosActivos;
use crate::database::queries::usuarios::FiltroUsuarios;
use crate::domain::autorizacion::Operacion;
use crate::models::medio_ingreso::MedioIngreso;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::registro_ingreso_service::IngresoActivoResumen;

use super::estado::ContextState;
use super::parser::{Comando, Entrada, GafeteParse, MedioParse};

/// Mínimo de caracteres de consulta para disparar la búsqueda en vivo. Con 1
/// letra la lista es ruido; con 2 ya recorta bien (los queries usan LIKE bajo
/// ese umbral y FTS5 a partir de 3 — ambos caminos los resuelve `BusquedaTexto`).
pub const MIN_CONSULTA: usize = 2;

/// Cuántas coincidencias se cargan por página. Antes de tener PageUp/PageDown
/// (ver `nuevo_offset_coincidencias`) el número tenía que caber sin scroll en
/// una terminal chica; ahora que hay paginado real, se sube a la misma
/// magnitud que usa Historial (`LIMITE_HISTORIAL_PREDETERMINADO`, 50) — una
/// terminal grande no debe quedarse mostrando 9 filas con la mitad de la
/// pantalla vacía cuando hay espacio de sobra; una chica sigue recortando la
/// vista igual que ya hace la tabla de Historial (`render.rs` no reserva
/// scroll propio, es `ratatui` el que recorta lo que no entra en el área).
pub const LIMITE_COINCIDENCIAS: usize = 50;

/// `*` como consulta pide "todos" en vez de un nombre — sin esto no había
/// forma de listar contratistas/empresas/usuarios completos desde el
/// buscador sin escribir al menos `MIN_CONSULTA` letras de un nombre real
/// (reportado en runtime real: "cómo veo todas las empresas"). Mismo
/// comodín en los tres buscadores, sin necesitar una pantalla nueva:
/// `/activos` ya listaba todo con la consulta vacía porque no tiene el
/// umbral de `MIN_CONSULTA` (no hay riesgo de escanear la tabla completa
/// con cada tecla); acá sí lo hay, así que hace falta un gesto explícito.
pub fn es_comodin_todos(consulta: &str) -> bool {
    consulta.trim() == "*"
}

/// El filtro de texto a mandarle a la consulta: `None` con el comodín
/// (sin filtrar nada, trae todos) o con la consulta vacía; el texto tal
/// cual en cualquier otro caso.
fn texto_de_consulta(consulta: &str) -> Option<String> {
    (!es_comodin_todos(consulta) && !consulta.is_empty()).then(|| consulta.to_string())
}

/// PageUp/PageDown sobre `Coincidencias`/`CoincidenciasEmpresas`/
/// `CoincidenciasUsuarios` (mismo patrón que `paginar` de
/// `historial_controller.rs`): `delta > 0` avanza sólo si `hay_mas` (la
/// página siguiente ya se confirmó al cargar la actual — total real para
/// contratistas, el truco del elemento de más para empresas/usuarios);
/// `delta < 0` retrocede si no estamos ya en la primera página. `None`
/// cuando no hay a dónde moverse, para que el llamador no dispare una
/// consulta idéntica a la que ya está en pantalla.
pub fn nuevo_offset_coincidencias(offset: usize, hay_mas: bool, delta: isize) -> Option<usize> {
    if delta > 0 {
        hay_mas.then_some(offset + LIMITE_COINCIDENCIAS)
    } else {
        (offset > 0).then(|| offset.saturating_sub(LIMITE_COINCIDENCIAS))
    }
}

/// Rango de gafetes que se ofrecen en el autocompletado de `G:` — el inventario
/// físico de la portería no pasa de unas pocas decenas.
pub const GAFETE_SUGERIDO_MIN: i64 = 1;
pub const GAFETE_SUGERIDO_MAX: i64 = 50;

/// Qué crea `/nuevo <sujeto>` — argumento posicional, no `--modificador`:
/// `/nuevo` es un comando global (no actúa sobre un resultado de búsqueda,
/// DEC-021), así que no le corresponde esa gramática; funciona igual que
/// `/editar <nombre>`, donde la consulta ya es el sujeto de la acción.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SujetoNuevo {
    Contratista,
    Empresa,
    Usuario,
}

/// Vacío = Contratista (compatibilidad: `/nuevo` a secas seguía creando lo
/// mismo que antes de que existieran los otros dos sujetos). `em`/`emp` para
/// empresa, nunca `e` a secas — ya es el alias de `/editar` y aunque el
/// parser no los confunde (son namespaces distintos: nombre de comando vs.
/// valor de un argumento), el operador sí podría.
fn sujeto_nuevo(consulta: &str) -> Option<SujetoNuevo> {
    match consulta.trim().to_lowercase().as_str() {
        "" | "contratista" | "c" => Some(SujetoNuevo::Contratista),
        "empresa" | "em" | "emp" => Some(SujetoNuevo::Empresa),
        "usuario" | "u" => Some(SujetoNuevo::Usuario),
        _ => None,
    }
}

/// Deriva el contexto completo a partir del parseo. Punto único de consulta
/// "mientras se teclea": se llama tras cada cambio del input.
pub fn resolver(core: &AppCore, entrada: &Entrada, sesion: &UsuarioSesion) -> ContextState {
    match entrada {
        Entrada::Inicio => ContextState::Inicio {
            total_dentro: contar_activos(core),
        },
        Entrada::BusquedaLibre { consulta } => resolver_busqueda_contratistas(core, consulta),
        Entrada::Desconocido { nombre } => ContextState::MensajeError {
            mensaje: format!("Comando no reconocido: /{nombre} — escriba /ayuda"),
        },
        Entrada::Comando {
            comando,
            consulta,
            gafete,
            medio,
        } => {
            if let Some(GafeteParse::Invalido(valor)) = gafete {
                return ContextState::MensajeError {
                    mensaje: format!("El gafete debe ser un número: G:{valor}"),
                };
            }
            if let Some(MedioParse::Invalido(valor)) = medio {
                return ContextState::MensajeError {
                    mensaje: format!("Medio no reconocido: M:{valor} (use caminando o vehiculo)"),
                };
            }
            let gafete_numero = match gafete {
                Some(GafeteParse::Valido(numero)) => Some(*numero),
                _ => None,
            };
            // `M:` no participa en la resolución del listado: lo conserva el
            // parseo y lo consume `preparar_resumen_ingreso` al confirmar.
            match comando {
                Comando::Ingreso => resolver_busqueda_contratistas(core, consulta),
                Comando::Salida => resolver_salida(core, consulta, gafete_numero),
                Comando::Gafete => ContextState::AbrirSalidaGafete {
                    texto: consulta.clone(),
                },
                Comando::Activos => resolver_activos(core, consulta),
                Comando::Nuevo => match sujeto_nuevo(consulta) {
                    Some(SujetoNuevo::Contratista) => ContextState::NuevoContratista,
                    Some(SujetoNuevo::Empresa) => ContextState::NuevoEmpresa,
                    Some(SujetoNuevo::Usuario) => ContextState::NuevoUsuario,
                    None => ContextState::MensajeError {
                        mensaje: format!(
                            "Sujeto no reconocido: /nuevo contratista|empresa|usuario \
                             (o /n c|em|u) — \"{consulta}\" no es ninguno"
                        ),
                    },
                },
                Comando::Editar => match sujeto_editar(consulta) {
                    (SujetoEditar::Contratista, resto) => {
                        resolver_busqueda_contratistas(core, &resto)
                    }
                    (SujetoEditar::Empresa, resto) => resolver_busqueda_empresas(core, &resto),
                    (SujetoEditar::Usuario, resto) => {
                        resolver_busqueda_usuarios(core, &resto, sesion)
                    }
                },
                Comando::Historial => {
                    if consulta.is_empty() {
                        ContextState::AbrirHistorial
                    } else {
                        ContextState::MensajeError {
                            mensaje: "El historial no toma argumentos: escriba /historial a secas"
                                .to_string(),
                        }
                    }
                }
                Comando::Ayuda => ContextState::Ayuda,
                Comando::CerrarSesion => ContextState::ConfirmarCerrarSesion,
            }
        }
    }
}

/// Con un contratista ya elegido, arma la tarjeta de validación previa al
/// ingreso: `preparar_ingreso` (solo lectura) más la disponibilidad del gafete
/// pedido. Si el contratista no requiere gafete, el `G:` del operador se
/// descarta igual que hace el servicio al persistir.
pub fn preparar_resumen_ingreso(
    core: &AppCore,
    contratista_id: i64,
    gafete: Option<i64>,
    medio: MedioIngreso,
) -> ContextState {
    let preparacion = match core.preparar_ingreso(contratista_id) {
        Ok(preparacion) => preparacion,
        Err(error) => {
            return ContextState::MensajeError {
                mensaje: format!("No se pudo preparar el ingreso: {error}"),
            };
        }
    };
    let gafete = if preparacion.requiere_gafete {
        gafete
    } else {
        None
    };
    let gafete_ocupante = gafete.and_then(|numero| buscar_activo_por_gafete(core, numero));
    ContextState::ResumenIngreso {
        preparacion,
        gafete,
        medio,
        gafete_ocupante,
    }
}

/// Ficha de una búsqueda de texto libre con el contratista ya elegido: el
/// resumen de la búsqueda ya trae todos los campos que muestra la ficha, no
/// hace falta otra consulta.
pub fn ficha_desde_resumen(resumen: ContratistaResumen) -> ContextState {
    ContextState::FichaContratista { resumen }
}

/// Qué edita `/editar <sujeto> <consulta>` — igual idea que `SujetoNuevo`
/// (DEC-045), pero acá siempre queda texto después del sujeto (la
/// búsqueda), así que sólo el primer token se interpreta como sujeto, nunca
/// la consulta completa. Mismos alias que `/nuevo` salvo "c" para
/// contratista: con `/nuevo` no había ambigüedad posible (no admite
/// consulta), acá "c" seguido de una búsqueda de una sola palabra sí
/// generaría una — se prefiere dejar "contratista" (default, sin prefijo)
/// como el único camino a ese sujeto (DEC-052).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SujetoEditar {
    Contratista,
    Empresa,
    Usuario,
}

/// Separa el primer token de `consulta` y lo interpreta como sujeto si
/// coincide con un alias reconocido; si no, todo el texto es la búsqueda de
/// un contratista (comportamiento previo a los sujetos, sin cambios).
fn sujeto_editar(consulta: &str) -> (SujetoEditar, String) {
    let recortado = consulta.trim_start();
    let mut partes = recortado.splitn(2, char::is_whitespace);
    let primero = partes.next().unwrap_or_default();
    match primero.to_lowercase().as_str() {
        "empresa" | "em" | "emp" => (
            SujetoEditar::Empresa,
            partes.next().unwrap_or_default().trim().to_string(),
        ),
        "usuario" | "u" => (
            SujetoEditar::Usuario,
            partes.next().unwrap_or_default().trim().to_string(),
        ),
        _ => (SujetoEditar::Contratista, consulta.to_string()),
    }
}

/// Página 0 (nueva búsqueda: cambió el texto) de contratistas/empresas/
/// usuarios comparte offset 0 — `pagina_*` (pública, la usa
/// `operando.rs::paginar_coincidencias` con el mismo `consulta` y un offset
/// distinto para PageUp/PageDown, sin reconsultar desde cero).
fn resolver_busqueda_empresas(core: &AppCore, consulta: &str) -> ContextState {
    pagina_empresas(core, consulta, 0)
}

pub fn pagina_empresas(core: &AppCore, consulta: &str, offset: usize) -> ContextState {
    let (items, hay_mas) = if es_comodin_todos(consulta) || consulta.chars().count() >= MIN_CONSULTA
    {
        let filtro = FiltroEmpresas {
            texto: texto_de_consulta(consulta),
            // Un elemento de más para saber si hay página siguiente
            // (`buscar_empresas` no devuelve un total aparte) — se descarta
            // antes de guardar `items`.
            limite: LIMITE_COINCIDENCIAS + 1,
            offset,
            ..FiltroEmpresas::default()
        };
        match core.buscar_empresas(&filtro) {
            Ok(mut items) => {
                let hay_mas = items.len() > LIMITE_COINCIDENCIAS;
                items.truncate(LIMITE_COINCIDENCIAS);
                (items, hay_mas)
            }
            Err(_) => {
                return ContextState::MensajeError {
                    mensaje: "No se pudo consultar las empresas".to_string(),
                };
            }
        }
    } else {
        (Vec::new(), false)
    };
    ContextState::CoincidenciasEmpresas {
        consulta: consulta.to_string(),
        items,
        seleccion: 0,
        offset,
        hay_mas,
    }
}

/// A diferencia de contratistas/empresas, buscar usuarios exige permiso
/// (`Operacion::GestionarUsuarios`) — mismo gate que `abrir_formulario_nuevo_usuario`,
/// aplicado acá para que el operador sin permiso ni siquiera vea una lista
/// vacía sospechosa, sino el mensaje explícito.
fn resolver_busqueda_usuarios(
    core: &AppCore,
    consulta: &str,
    sesion: &UsuarioSesion,
) -> ContextState {
    pagina_usuarios(core, consulta, 0, sesion)
}

pub fn pagina_usuarios(
    core: &AppCore,
    consulta: &str,
    offset: usize,
    sesion: &UsuarioSesion,
) -> ContextState {
    if !sesion.rol.puede(Operacion::GestionarUsuarios) {
        return ContextState::MensajeError {
            mensaje: "No tiene permiso para gestionar usuarios".to_string(),
        };
    }
    let (items, hay_mas) = if es_comodin_todos(consulta) || consulta.chars().count() >= MIN_CONSULTA
    {
        let filtro = FiltroUsuarios {
            texto: texto_de_consulta(consulta),
            // Mismo truco de "un elemento de más" que `pagina_empresas`.
            limite: LIMITE_COINCIDENCIAS + 1,
            offset,
            ..FiltroUsuarios::default()
        };
        match core.buscar_usuarios(sesion, &filtro) {
            Ok(mut items) => {
                let hay_mas = items.len() > LIMITE_COINCIDENCIAS;
                items.truncate(LIMITE_COINCIDENCIAS);
                (items, hay_mas)
            }
            Err(_) => {
                return ContextState::MensajeError {
                    mensaje: "No se pudo consultar los usuarios".to_string(),
                };
            }
        }
    } else {
        (Vec::new(), false)
    };
    ContextState::CoincidenciasUsuarios {
        consulta: consulta.to_string(),
        items,
        seleccion: 0,
        offset,
        hay_mas,
    }
}

fn resolver_busqueda_contratistas(core: &AppCore, consulta: &str) -> ContextState {
    pagina_contratistas(core, consulta, 0)
}

pub fn pagina_contratistas(core: &AppCore, consulta: &str, offset: usize) -> ContextState {
    let (items, total) = if es_comodin_todos(consulta) || consulta.chars().count() >= MIN_CONSULTA {
        let filtro = FiltroContratistas {
            texto: texto_de_consulta(consulta),
            limite: LIMITE_COINCIDENCIAS,
            offset,
            ..FiltroContratistas::default()
        };
        match core.buscar_contratistas(&filtro) {
            // `pagina.total` ya es el conteo real sin límite (query aparte
            // en `buscar_contratistas`) — a diferencia de empresas/usuarios
            // no hace falta el truco del elemento de más.
            Ok(pagina) => (pagina.items, pagina.total),
            Err(_) => {
                return ContextState::MensajeError {
                    mensaje: "No se pudo consultar los contratistas".to_string(),
                };
            }
        }
    } else {
        (Vec::new(), 0)
    };
    ContextState::Coincidencias {
        consulta: consulta.to_string(),
        items,
        seleccion: 0,
        offset,
        total,
    }
}

fn resolver_salida(core: &AppCore, consulta: &str, gafete: Option<i64>) -> ContextState {
    let descripcion = match (consulta.is_empty(), gafete) {
        (false, _) => format!("\"{consulta}\""),
        (true, Some(numero)) => format!("gafete {numero}"),
        (true, None) => String::new(),
    };
    // Sin criterio todavía (ni texto suficiente ni gafete): lista vacía — el
    // render muestra la pista de uso en vez de un "sin resultados" engañoso.
    if consulta.chars().count() < MIN_CONSULTA && gafete.is_none() {
        return ContextState::CoincidenciasActivos {
            descripcion: String::new(),
            items: Vec::new(),
            seleccion: 0,
        };
    }
    let filtro = FiltroIngresosActivos {
        texto: (!consulta.is_empty()).then(|| consulta.to_string()),
        gafete_numero: gafete.map(Igualdad::Incluye),
        ..FiltroIngresosActivos::default()
    };
    let items = match core.listar_ingresos_activos(&filtro) {
        Ok(lista) => lista.items,
        Err(_) => {
            return ContextState::MensajeError {
                mensaje: "No se pudo consultar los ingresos activos".to_string(),
            };
        }
    };
    // Una sola coincidencia salta directo a la tarjeta de confirmación — con
    // varias, lista para elegir con ↑↓; con ninguna, la lista vacía muestra
    // "No hay ingreso activo para …".
    if let [unico] = items.as_slice() {
        return ContextState::ResumenSalida {
            activo: unico.clone(),
        };
    }
    ContextState::CoincidenciasActivos {
        descripcion,
        items,
        seleccion: 0,
    }
}

fn resolver_activos(core: &AppCore, consulta: &str) -> ContextState {
    let filtro = FiltroIngresosActivos {
        texto: (!consulta.is_empty()).then(|| consulta.to_string()),
        ..FiltroIngresosActivos::default()
    };
    match core.listar_ingresos_activos(&filtro) {
        Ok(lista) => ContextState::TablaActivos {
            items: lista.items,
            total: lista.total,
            seleccion: 0,
        },
        Err(_) => ContextState::MensajeError {
            mensaje: "No se pudo consultar los ingresos activos".to_string(),
        },
    }
}

fn contar_activos(core: &AppCore) -> usize {
    core.listar_ingresos_activos(&FiltroIngresosActivos::default())
        .map(|lista| lista.total)
        .unwrap_or(0)
}

fn buscar_activo_por_gafete(core: &AppCore, numero: i64) -> Option<IngresoActivoResumen> {
    let filtro = FiltroIngresosActivos {
        gafete_numero: Some(Igualdad::Incluye(numero)),
        ..FiltroIngresosActivos::default()
    };
    core.listar_ingresos_activos(&filtro)
        .ok()
        .and_then(|lista| lista.items.into_iter().next())
}

/// Gafetes del rango sugerido que no aparecen en ningún ingreso activo.
fn gafetes_libres(core: &AppCore) -> Vec<i64> {
    let ocupados: Vec<i64> = core
        .listar_ingresos_activos(&FiltroIngresosActivos::default())
        .map(|lista| {
            lista
                .items
                .iter()
                .filter_map(|item| item.gafete_numero)
                .collect()
        })
        .unwrap_or_default();
    (GAFETE_SUGERIDO_MIN..=GAFETE_SUGERIDO_MAX)
        .filter(|numero| !ocupados.contains(numero))
        .collect()
}

/// Pistas de la línea de sugerencias, según lo que hay tecleado. No alteran el
/// estado: son el "qué puedo escribir ahora" del autocompletado contextual.
pub fn calcular_sugerencias(core: &AppCore, texto: &str, entrada: &Entrada) -> Vec<String> {
    // Tecleando el nombre del comando: sugerir los que empiecen igual.
    if texto.starts_with('/') && !texto.contains(' ') {
        let prefijo = texto[1..].to_lowercase();
        let coinciden: Vec<String> = Comando::TODOS
            .iter()
            .filter(|comando| comando.nombre().starts_with(&prefijo))
            .map(|comando| format!("/{}", comando.nombre()))
            .collect();
        if !coinciden.is_empty() {
            return vec![format!("comandos: {}", coinciden.join("  "))];
        }
    }

    // Completando un parámetro a medio escribir.
    if let Some(ultimo) = texto.split_whitespace().last() {
        let minusculas = ultimo.to_lowercase();
        if let Some(digitos) = minusculas.strip_prefix("g:") {
            let libres = gafetes_libres(core);
            let sugeridos: Vec<String> = libres
                .iter()
                .filter(|numero| numero.to_string().starts_with(digitos))
                .take(8)
                .map(|numero| numero.to_string())
                .collect();
            if sugeridos.is_empty() {
                return vec!["sin gafetes libres con ese prefijo (1-50)".to_string()];
            }
            return vec![format!("gafetes libres: {}", sugeridos.join("  "))];
        }
        if let Some(letras) = minusculas.strip_prefix("m:") {
            let opciones: Vec<&str> = ["caminando", "vehiculo"]
                .into_iter()
                .filter(|opcion| opcion.starts_with(letras))
                .collect();
            if !opciones.is_empty() {
                return vec![format!("medio: {}", opciones.join("  "))];
            }
        }
    }

    match entrada {
        Entrada::Inicio => vec!["escriba / para comandos — /ayuda explica la sintaxis".into()],
        Entrada::Comando {
            comando: Comando::Ingreso,
            gafete,
            medio,
            ..
        } => {
            let mut pistas = vec!["↑↓ elegir · Enter confirmar".to_string()];
            let mut parametros = Vec::new();
            if gafete.is_none() {
                parametros.push("G:<número> gafete");
            }
            if medio.is_none() {
                parametros.push("M:caminando|vehiculo");
            }
            if !parametros.is_empty() {
                pistas.push(parametros.join(" · "));
            }
            pistas
        }
        Entrada::Comando {
            comando: Comando::Salida,
            ..
        } => vec!["nombre o G:<número> del gafete · ↑↓ elegir · Enter confirmar".into()],
        Entrada::Comando {
            comando: Comando::Gafete,
            ..
        } => vec!["número(s) de gafete, separados por coma · Enter registra la salida".into()],
        Entrada::Comando {
            comando: Comando::Activos,
            ..
        } => vec!["↑↓ elegir · Enter registrar salida · Esc limpiar".into()],
        Entrada::Comando {
            comando: Comando::Nuevo,
            ..
        } => vec!["Enter abre el formulario de alta · Esc limpiar".into()],
        Entrada::Comando {
            comando: Comando::Editar,
            consulta,
            ..
        } => match sujeto_editar(consulta).0 {
            SujetoEditar::Contratista => {
                vec!["nombre o cédula del contratista · ↑↓ elegir · Enter abrir edición".into()]
            }
            SujetoEditar::Empresa => {
                vec!["nombre de la empresa · ↑↓ elegir · Enter abrir edición".into()]
            }
            SujetoEditar::Usuario => {
                vec!["nombre o cédula del usuario · ↑↓ elegir · Enter abrir edición".into()]
            }
        },
        _ => vec!["Enter confirmar · Esc limpiar · Ctrl+C salir".into()],
    }
}

/// Autocompletado predecible de Tab. Devuelve el nuevo texto del input, o
/// `None` cuando no hay nada razonable que completar — nunca completa nada que
/// implique una acción (eso siempre lo confirma Enter).
pub fn autocompletar(core: &AppCore, texto: &str) -> Option<String> {
    // 1) Nombre de comando a medio escribir: "/ing" → "/ingreso ".
    if texto.starts_with('/') && !texto.contains(' ') {
        let prefijo = texto[1..].to_lowercase();
        let nombre = Comando::TODOS
            .into_iter()
            .map(|comando| comando.nombre())
            .find(|nombre| nombre.starts_with(&prefijo) && *nombre != prefijo)?;
        return Some(format!("/{nombre} "));
    }

    let recortado_fin = texto.trim_end();
    let ultimo = recortado_fin.split_whitespace().last().unwrap_or_default();
    let minusculas = ultimo.to_lowercase();

    // 2) "G:<prefijo numérico>" (o "G:" vacío) → primer gafete libre que
    //    empiece con ese prefijo.
    if let Some(digitos) = minusculas.strip_prefix("g:") {
        let libre = gafetes_libres(core)
            .into_iter()
            .find(|numero| numero.to_string().starts_with(digitos))?;
        return Some(reemplazar_ultimo_token(texto, &format!("G:{libre} ")));
    }

    // 3) "M:<prefijo>" → caminando/vehiculo.
    if let Some(letras) = minusculas.strip_prefix("m:") {
        let opcion = ["caminando", "vehiculo"]
            .into_iter()
            .find(|opcion| opcion.starts_with(letras) && *opcion != letras)?;
        return Some(reemplazar_ultimo_token(texto, &format!("M:{opcion} ")));
    }

    // 4) Tras "/ingreso <nombre> " ofrecer el primer parámetro que falte.
    if let Entrada::Comando {
        comando: Comando::Ingreso,
        gafete,
        medio,
        ..
    } = super::parser::parsear(texto)
        && texto.ends_with(' ')
    {
        if gafete.is_none() {
            return Some(format!("{texto}G:"));
        }
        if medio.is_none() {
            return Some(format!("{texto}M:"));
        }
    }

    None
}

/// Sustituye el último token del texto conservando lo anterior (incluido el
/// espacio que lo separaba).
fn reemplazar_ultimo_token(texto: &str, nuevo: &str) -> String {
    let recortado = texto.trim_end();
    match recortado.rfind(char::is_whitespace) {
        Some(posicion) => format!("{} {nuevo}", &recortado[..posicion]),
        None => nuevo.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comodin_todos_reconoce_asterisco_con_espacios_alrededor() {
        assert!(es_comodin_todos("*"));
        assert!(es_comodin_todos("  *  "));
        assert!(!es_comodin_todos("*ana"));
        assert!(!es_comodin_todos("ana"));
        assert!(!es_comodin_todos(""));
    }

    #[test]
    fn texto_de_consulta_es_none_para_comodin_y_vacio() {
        assert_eq!(texto_de_consulta("*"), None);
        assert_eq!(texto_de_consulta(""), None);
        assert_eq!(texto_de_consulta("ana"), Some("ana".to_string()));
    }

    #[test]
    fn nuevo_offset_avanza_solo_si_hay_mas() {
        assert_eq!(
            nuevo_offset_coincidencias(0, true, 1),
            Some(LIMITE_COINCIDENCIAS)
        );
        assert_eq!(nuevo_offset_coincidencias(0, false, 1), None);
    }

    #[test]
    fn nuevo_offset_retrocede_solo_si_no_esta_en_la_primera_pagina() {
        assert_eq!(
            nuevo_offset_coincidencias(LIMITE_COINCIDENCIAS, true, -1),
            Some(0)
        );
        assert_eq!(nuevo_offset_coincidencias(0, true, -1), None);
    }

    #[test]
    fn nuevo_offset_retroceder_nunca_baja_de_cero() {
        // Offset menor que una página completa (p. ej. tras un total que no
        // es múltiplo exacto del límite): satura en 0, no resta de más.
        assert_eq!(nuevo_offset_coincidencias(3, true, -1), Some(0));
    }
}
