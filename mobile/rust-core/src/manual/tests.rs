//! Mismos casos que `desktop/src/ayuda/manual.test.ts` (el lector de
//! TypeScript), más el aplanado para Kotlin y el manual real del móvil.

use super::buscar::{buscar, normalizar};
use super::markdown::{
    Bloque, ElementoLista, EnLinea, ancla, leer_en_linea, leer_markdown, texto_plano,
};
use super::{
    BloqueManual, MANUAL, armar, buscar_en_manual, contenido, enlaces, indice_manual,
    resolver_enlace, tema_manual,
};

fn titulo(nivel: usize, texto: &str, ancla: &str) -> Bloque {
    Bloque::Titulo {
        nivel,
        texto: texto.to_string(),
        ancla: ancla.to_string(),
    }
}

#[test]
fn une_las_lineas_de_un_parrafo_y_lee_titulos_con_su_ancla() {
    let bloques = leer_markdown(
        "# Registrar una salida\n\n## Para qué sirve\n\nCerrar el ingreso\nde una persona.\n",
    );
    assert_eq!(
        bloques,
        vec![
            titulo(1, "Registrar una salida", "registrar-una-salida"),
            titulo(2, "Para qué sirve", "para-qué-sirve"),
            Bloque::Parrafo {
                texto: "Cerrar el ingreso de una persona.".to_string()
            },
        ]
    );
}

#[test]
fn anclas_como_las_de_github() {
    assert_eq!(
        ancla("\"Tiene un ingreso activo acá Y en otro sitio\""),
        "tiene-un-ingreso-activo-acá-y-en-otro-sitio"
    );
    assert_eq!(
        ancla("Pasos — por gafete (una o varias)"),
        "pasos--por-gafete-una-o-varias"
    );
    let anclas: Vec<String> = leer_markdown("## Pasos\n\n## Pasos\n")
        .into_iter()
        .map(|b| match b {
            Bloque::Titulo { ancla, .. } => ancla,
            _ => String::new(),
        })
        .collect();
    assert_eq!(anclas, vec!["pasos", "pasos-1"]);
}

#[test]
fn listas_numeradas_con_vinetas_anidadas_y_texto_en_la_linea_de_abajo() {
    let bloques = leer_markdown(
        "1. Pulse **Ctrl+S**.\n2. Elija una opción:\n   - Si la persona **ya salió**, registre\n     su salida.\n   - Si sigue adentro, no haga nada.\n3. Listo, se ve\n   el mensaje.",
    );
    let elemento = |texto: &str| ElementoLista {
        texto: texto.to_string(),
        hijos: vec![],
    };
    assert_eq!(
        bloques,
        vec![Bloque::Lista {
            ordenada: true,
            inicio: 1,
            elementos: vec![
                elemento("Pulse **Ctrl+S**."),
                ElementoLista {
                    texto: "Elija una opción:".to_string(),
                    hijos: vec![Bloque::Lista {
                        ordenada: false,
                        inicio: 1,
                        elementos: vec![
                            elemento("Si la persona **ya salió**, registre su salida."),
                            elemento("Si sigue adentro, no haga nada."),
                        ],
                    }],
                },
                elemento("Listo, se ve el mensaje."),
            ],
        }]
    );
}

#[test]
fn una_linea_en_blanco_entre_elementos_no_parte_la_lista() {
    let bloques = leer_markdown("- uno\n\n- dos\n\nDespués.\n");
    assert_eq!(bloques.len(), 2);
    assert!(matches!(&bloques[0], Bloque::Lista { elementos, .. } if elementos.len() == 2));
    assert_eq!(
        bloques[1],
        Bloque::Parrafo {
            texto: "Después.".to_string()
        }
    );
}

#[test]
fn tablas_sin_separador_y_con_barras_escapadas() {
    let bloques = leer_markdown(
        "| Mensaje | Qué hacer |\n|---|---|\n| \"Sin ingreso activo\" | Revise el número. |\n| a \\| b | c |\n",
    );
    assert_eq!(
        bloques,
        vec![Bloque::Tabla {
            encabezado: vec!["Mensaje".to_string(), "Qué hacer".to_string()],
            filas: vec![
                vec![
                    "\"Sin ingreso activo\"".to_string(),
                    "Revise el número.".to_string()
                ],
                vec!["a | b".to_string(), "c".to_string()],
            ],
        }]
    );
}

#[test]
fn en_linea_negrita_con_enlace_codigo_y_asteriscos_sueltos() {
    assert_eq!(
        leer_en_linea("Pulse **[Salida](registrar-salida.md)** o `Esc`."),
        vec![
            EnLinea::Texto("Pulse ".to_string()),
            EnLinea::Negrita(vec![EnLinea::Enlace {
                destino: "registrar-salida.md".to_string(),
                hijos: vec![EnLinea::Texto("Salida".to_string())],
            }]),
            EnLinea::Texto(" o ".to_string()),
            EnLinea::Codigo("Esc".to_string()),
            EnLinea::Texto(".".to_string()),
        ]
    );
    assert_eq!(texto_plano(&leer_en_linea("2 * 3 ** y")), "2 * 3 ** y");
}

#[test]
fn resuelve_enlaces_relativos_con_ancla_y_descarta_los_de_afuera() {
    assert_eq!(
        resolver_enlace("movil/a.md", "b.md"),
        Some(("movil/b.md".to_string(), None))
    );
    assert_eq!(
        resolver_enlace("movil/a.md", "../escritorio/c.md#paso-2"),
        Some(("escritorio/c.md".to_string(), Some("paso-2".to_string())))
    );
    assert_eq!(
        resolver_enlace("movil/a.md", "#mensajes"),
        Some(("movil/a.md".to_string(), Some("mensajes".to_string())))
    );
    assert_eq!(resolver_enlace("movil/a.md", "https://example.com"), None);
    assert_eq!(resolver_enlace("movil/a.md", "imagen.png"), None);
}

const README: &str = "# Manual\n## Índice — Escritorio\n- [Otro](escritorio/x.md)\n## Índice — Móvil (teléfono)\n### Ingresos\n- ✅ [Ingreso](movil/ingreso.md)\n### Fuera del manual\n- **Rutas**: no.\n## Mejoras\n1. [Ingreso](movil/ingreso.md) otra vez.\n";

#[test]
fn indice_en_el_orden_del_readme_y_lo_que_falta_en_otros() {
    let manual = armar(&[
        ("README.md", README),
        ("movil/ingreso.md", "# Registrar un ingreso\n\nTexto."),
        ("movil/suelto.md", "# Proceso sin índice"),
        ("escritorio/x.md", "# Del escritorio"),
    ]);
    let indice: Vec<(String, Vec<String>)> = manual
        .indice
        .iter()
        .map(|g| {
            (
                g.titulo.clone(),
                g.temas.iter().map(|t| t.ruta.clone()).collect(),
            )
        })
        .collect();
    assert_eq!(
        indice,
        vec![
            ("Ingresos".to_string(), vec!["movil/ingreso.md".to_string()]),
            ("Otros".to_string(), vec!["movil/suelto.md".to_string()]),
        ]
    );
    // Sólo los procesos del móvil.
    assert!(manual.proceso("escritorio/x.md").is_none());
}

#[test]
fn aplana_listas_con_marca_y_sangria_y_resuelve_los_enlaces() {
    let manual = armar(&[
        ("README.md", README),
        (
            "movil/ingreso.md",
            "# Ingreso\n\n1. Toque **[Salida](salida.md#pasos)**.\n   - Detalle.\n\nVer [afuera](../escritorio/x.md).\n",
        ),
        ("movil/salida.md", "# Salida\n\n## Pasos\n"),
    ]);
    let tema = contenido(&manual, "movil/ingreso.md").expect("existe");
    let BloqueManual::Elemento {
        sangria,
        marca,
        tramos,
    } = &tema.bloques[1]
    else {
        panic!("se esperaba un elemento: {:?}", tema.bloques[1]);
    };
    assert_eq!((*sangria, marca.as_str()), (0, "1."));
    let enlace = tramos
        .iter()
        .find(|t| t.texto == "Salida")
        .expect("tramo del enlace");
    assert!(enlace.negrita);
    assert_eq!(enlace.enlace.as_deref(), Some("movil/salida.md"));
    assert_eq!(enlace.ancla_enlace.as_deref(), Some("pasos"));
    assert!(matches!(
        &tema.bloques[2],
        BloqueManual::Elemento { sangria: 1, marca, .. } if marca == "•"
    ));
    // Un tema que no está en este manual queda como texto.
    let BloqueManual::Parrafo { tramos, .. } = &tema.bloques[3] else {
        panic!("se esperaba un párrafo");
    };
    assert!(tramos.iter().all(|t| t.enlace.is_none()));
}

#[test]
fn busca_sin_tildes_ni_mayusculas_y_premia_el_mensaje_entero() {
    let manual = armar(&[
        ("README.md", ""),
        (
            "movil/salida.md",
            "# Registrar una salida\n\n## Mensajes que pueden aparecer\n\n| Mensaje | Qué hacer |\n|---|---|\n| \"Sin ingreso activo\" | Revise el número del gafete. |\n",
        ),
        (
            "movil/gafete.md",
            "# Gafete perdido\n\n## Pasos\n\nMarque el gafete como perdido. El contratista queda sin ingreso.\n",
        ),
        (
            "movil/praind.md",
            "# PRAIND vencido\n\nEl contratista no puede entrar.",
        ),
    ]);
    let rutas = |consulta: &str| -> Vec<String> {
        buscar(&manual.procesos, consulta, 20)
            .into_iter()
            .map(|r| r.ruta)
            .collect()
    };
    assert_eq!(
        normalizar("¿Cómo REGISTRO la Salida?"),
        "como registro la salida"
    );
    assert_eq!(rutas("gafet"), vec!["movil/gafete.md", "movil/salida.md"]);
    assert_eq!(rutas("práind"), vec!["movil/praind.md"]);
    assert_eq!(rutas("el gafete perdido"), vec!["movil/gafete.md"]);
    // "marco" no está (el manual dice "Marque"): gana el que tiene más.
    assert_eq!(
        rutas("cómo marco un gafete perdido"),
        vec!["movil/gafete.md"]
    );
    assert!(rutas("tractor amarillo").is_empty());
    assert!(rutas("gafete tractor amarillo").is_empty());
    assert!(rutas("a").is_empty());

    let primero = buscar(&manual.procesos, "Sin ingreso activo", 20).remove(0);
    assert_eq!(primero.ruta, "movil/salida.md");
    assert_eq!(
        primero.seccion.as_deref(),
        Some("Mensajes que pueden aparecer")
    );
    assert_eq!(
        primero.ancla.as_deref(),
        Some("mensajes-que-pueden-aparecer")
    );
    assert_eq!(
        primero.fragmento,
        "\"Sin ingreso activo\" — Revise el número del gafete."
    );
}

#[test]
fn recorta_los_fragmentos_largos_cerca_de_la_palabra() {
    let largo = format!(
        "{} gafete {}",
        "relleno ".repeat(40),
        "más relleno ".repeat(40)
    );
    let manual = armar(&[
        ("README.md", ""),
        ("movil/x.md", &format!("# X\n\n{largo}\n")),
    ]);
    let fragmento = buscar(&manual.procesos, "gafete", 1).remove(0).fragmento;
    assert!(
        fragmento.starts_with('…') && fragmento.ends_with('…'),
        "{fragmento}"
    );
    assert!(fragmento.contains("gafete"));
    assert!(fragmento.chars().count() <= 182);
}

// ---- El manual real del móvil, tal como viaja en el .so ----

fn todos_los_enlaces(bloques: &[Bloque]) -> Vec<String> {
    let mut destinos = Vec::new();
    for bloque in bloques {
        match bloque {
            Bloque::Titulo { texto, .. } | Bloque::Parrafo { texto } => {
                destinos.extend(enlaces(&leer_en_linea(texto)));
            }
            Bloque::Lista { elementos, .. } => {
                for elemento in elementos {
                    destinos.extend(enlaces(&leer_en_linea(&elemento.texto)));
                    destinos.extend(todos_los_enlaces(&elemento.hijos));
                }
            }
            Bloque::Tabla { encabezado, filas } => {
                for celda in encabezado.iter().chain(filas.iter().flatten()) {
                    destinos.extend(enlaces(&leer_en_linea(celda)));
                }
            }
        }
    }
    destinos
}

fn anclas(bloques: &[BloqueManual]) -> Vec<&str> {
    bloques
        .iter()
        .filter_map(|b| match b {
            BloqueManual::Titulo { ancla, .. } => Some(ancla.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn el_manual_del_movil_viene_completo_y_en_el_indice() {
    assert!(MANUAL.procesos.len() > 10, "{}", MANUAL.procesos.len());
    for proceso in &MANUAL.procesos {
        assert!(
            matches!(
                proceso.bloques.first(),
                Some(Bloque::Titulo { nivel: 1, .. })
            ),
            "{} no empieza con su título",
            proceso.ruta
        );
    }
    let indice = indice_manual();
    assert!(
        indice.iter().all(|g| g.titulo != "Otros"),
        "hay procesos fuera del índice del README"
    );
    let en_indice: usize = indice.iter().map(|g| g.temas.len()).sum();
    assert_eq!(en_indice, MANUAL.procesos.len());
}

#[test]
fn ningun_enlace_del_manual_del_movil_esta_roto() {
    let mut rotos = Vec::new();
    for proceso in &MANUAL.procesos {
        for destino in todos_los_enlaces(&proceso.bloques) {
            let Some((ruta, ancla)) = resolver_enlace(&proceso.ruta, &destino) else {
                rotos.push(format!("{} → {destino}", proceso.ruta));
                continue;
            };
            match tema_manual(ruta) {
                None => rotos.push(format!("{} → {destino}", proceso.ruta)),
                Some(objetivo) => {
                    if let Some(ancla) = ancla
                        && !anclas(&objetivo.bloques).contains(&ancla.as_str())
                    {
                        rotos.push(format!(
                            "{} → {destino} (no existe la sección)",
                            proceso.ruta
                        ));
                    }
                }
            }
        }
    }
    assert!(rotos.is_empty(), "enlaces rotos: {rotos:#?}");
}

#[test]
fn los_mensajes_de_la_app_se_encuentran_pegandolos() {
    let primero =
        buscar_en_manual("Este encargado ya tiene un gafete provisional prestado".to_string())
            .into_iter()
            .next()
            .expect("algún resultado");
    assert_eq!(primero.ruta, "movil/gafete-kof.md");
    assert_eq!(
        primero.seccion.as_deref(),
        Some("Mensajes que pueden aparecer")
    );
    assert!(tema_manual("movil/no-existe.md".to_string()).is_none());
}
