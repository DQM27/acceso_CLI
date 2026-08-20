# Hallazgos del análisis del buscador (2026-08-19)

Análisis de extremo a extremo del buscador de texto libre y del filtro `clave:valor`,
en 4 ángulos en paralelo (multi-agente): UX/consistencia entre pantallas, consultas SQL,
velocidad/eficiencia, y manejo de acentos/eñes. Cada hallazgo fue verificado por un
segundo agente independiente leyendo el código real antes de confirmarse — no se reporta
nada que no se haya comprobado abriendo el archivo citado.

**Estado (2026-08-20): 17/18 hallazgos confirmados, 17/17 resueltos** (12 reparados con
cambio de código, 5 confirmados intencionales/aceptables y documentados sin cambiar
comportamiento — ver el detalle de cada uno abajo, marcado explícitamente). 1 hallazgo
propuesto se descartó por evidencia (ver abajo).

El buscador es compartido: `src/database/search.rs` (`BusquedaTexto::preparar`) clasifica
cualquier texto en 3 modos — vacío (sin filtro), `< 3` caracteres (`LIKE '%texto%' COLLATE
NOCASE`), `>= 3` caracteres (FTS5 trigram con `remove_diacritics 1`). Lo usan los 4 módulos
de consulta (`contratistas`, `empresas`, `usuarios`, `ingresos` — este último cubre Activos,
Historial y Salida Rápida). Aparte del texto libre, varias pantallas soportan sintaxis
`clave:valor` (`empresa:`, `tipo:`, `praind:`, `gafete:`, etc.) resuelta en Rust, no en SQL,
vía `ui_kit/query_lang.rs` + una función `aplicar_clave` por pantalla.

## Acentos y eñes

- [x] **`empresa:alvarez` no encuentra "Álvarez Ingeniería".** `src/tui/contratistas/state.rs:52-56`
  (idéntico en `activos/state.rs:35-39` y `historial/filtros.rs:134-138`). **Severidad alta.**
  El filtro `empresa:` comparaba `e.nombre.to_lowercase().contains(&buscado)` en Rust puro —
  `to_lowercase()` solo pliega mayúsculas Unicode, no quita tildes, así que `"álvarez".contains("alvarez")`
  daba `false` carácter a carácter (`á` U+00E1 nunca es `a` U+0061). El buscador de texto libre
  de la misma pantalla sí resolvía esto bien (FTS con `remove_diacritics=1`) — la
  inconsistencia era solo del filtro estructurado `empresa:`, copiado igual en 3 archivos.
  **Reparado (2026-08-20):** nueva función compartida `ui_kit::plegar_diacriticos`
  (`src/tui/ui_kit/texto.rs`) que pliega vocales acentuadas, diéresis y `ñ→n`/`ç→c` (mismo
  criterio que ya usa FTS5 con `remove_diacritics=1`, confirmado intencional más abajo); se
  aplica a ambos lados de la comparación (`empresa.nombre` y el valor buscado) en los 3
  archivos, en vez de triplicar la lógica.

- [x] **Búsquedas de 1-2 caracteres no pliegan tildes ni Ñ.** `src/database/search.rs`
  (consumido con `COLLATE NOCASE` en `contratistas.rs`, `empresas.rs`, `usuarios.rs`,
  `ingresos.rs`). **Severidad media.** `COLLATE NOCASE` en SQLite solo plegaba ASCII A-Z;
  buscar "os" (2 caracteres) no encontraba "Óscar", pero "osc" (3+, ya en modo FTS) sí.
  **Reparado (2026-08-20):** nueva función SQL `PLEGAR(texto)` (registrada en
  `database::schema::initialize_database`, implementación compartida en
  `crate::texto::plegar_para_busqueda`, la misma base que ya usaba
  `ui_kit::plegar_diacriticos` para el filtro `empresa:`) — los 4 módulos ahora comparan
  `PLEGAR(columna) LIKE PLEGAR(:patron)` en vez de `columna LIKE :patron COLLATE NOCASE`.
  **Antes de implementar se comprobó experimentalmente** (no se asumió) que una `COLLATE`
  personalizada (`sqlite3_create_collation`) **no** es suficiente: SQLite la aplica a
  comparaciones de igualdad pero no al operador `LIKE`, que sólo respeta
  `PRAGMA case_sensitive_like`/su propio plegado ASCII interno — una función SQL sí participa
  porque el plegado ocurre antes de la comparación, no durante. Requirió habilitar la
  feature `functions` de `rusqlite` en `Cargo.toml`. Cubierto por
  `busqueda_corta_pliega_tildes` en los 4 archivos de test de consultas (`contratista_queries`,
  `empresa_queries`, `usuario_queries`, `ingreso_queries` ×2 — Activos e Historial), cada uno
  con un caso donde la subcadena ASCII buscada no aparece literalmente en el texto acentuado
  (p. ej. "al" no es subcadena literal de "Álvarez").

- [x] **FTS ya funde ñ/n a propósito — no es un bug.** `src/database/schema.rs:373-386,540-543`.
  `remove_diacritics=1` en las 4 tablas FTS5 hace que "nino" encuentre "Niño" y viceversa
  (confirmado también por el test existente `contratistas_busca_subcadenas_sin_distinguir_tildes_o_mayusculas`).
  Es una decisión razonable de tolerancia a errores de tipeo para nombres reales, no un
  descuido — vale la pena dejar un comentario en `schema.rs` aclarando que es intencional,
  para que nadie lo "corrija" pensando que es un error.

## Consultas / SQL

- [x] **Historial no encuentra por número de gafete en texto libre; Activos sí.**
  `src/database/queries/ingresos.rs`. **Severidad media.** `ACTIVOS_SQL` comparaba
  `:numero_exacto` contra `gafete_numero` en ambos modos (LIKE y FTS); el `WHERE` de
  Historial nunca lo hacía — solo funcionaba con la clave explícita `gafete:26`. **Reparado
  (2026-08-20)** como parte de la reescritura de abajo: `construir_where_historial` ahora
  arma la misma unión explícita por gafete exacto que ya tenía Activos (`registro_ingresos_fts`
  no indexa `gafete_numero`, así que ninguno de los dos modos puede encontrarlo sólo con
  `MATCH`/`LIKE`). Cubierto por
  `historial_encuentra_por_gafete_en_texto_libre_modo_corto_y_fts` (`tests/ingreso_queries.rs`),
  con un caso para cada modo (2 y 3+ caracteres).

- [x] **`gafete:abc` (no numérico) se comporta distinto en Activos e Historial.**
  `src/tui/activos/state.rs` vs `src/tui/historial/filtros.rs`. **Severidad media.** En
  Activos, un valor no numérico se ignoraba en silencio (el término quedaba como texto
  libre). En Historial, `aplicar_clave` aceptaba cualquier valor y lo escribía en
  `f.gafete`; `construir()` recién lo validaba después y devolvía
  `Err("Ingrese un número de gafete válido")`, bloqueando toda la búsqueda — mismo input,
  dos experiencias. **Reparado (2026-08-20):** `aplicar_clave` en `historial/filtros.rs`
  ahora exige que el valor parsee como número antes de aceptar la clave (mismo criterio que
  Activos); un valor no numérico cae a texto libre en silencio, igual en las dos pantallas.
  El mensaje `"Ingrese un número de gafete válido"` de `construir()` queda intacto para
  cuando el operador escribe directo en el campo del panel clásico — ahí sí es el
  comportamiento correcto, es un campo de formulario, no una búsqueda rápida. Cubierto por
  `parsear_consulta_gafete_no_numerico_cae_a_texto_libre` (`src/tui/historial/tests.rs`).

- [x] **El "total" de Activos ignora el filtro aplicado.** `src/database/queries/ingresos.rs`.
  **Severidad baja.** El `COUNT(*)` de `listar_activos` es una cadena fija (`WHERE
  fecha_hora_salida IS NULL`) sin ninguno de los parámetros de búsqueda; `items` sí está
  filtrado. Contraste con Contratistas/Historial, donde el `COUNT` reutiliza literalmente el
  mismo `WHERE` que el `SELECT`. **Confirmado intencional, no se cambia (2026-08-20):** es
  coherente con lo que muestra la UI ("`N` DE `M` DENTRO" = filtrados de totales sin filtrar)
  — se dejó un comentario explícito en el código junto al `COUNT(*)` para que un cambio
  futuro no lo "unifique" con Contratistas/Historial sin querer.

- [x] **Sin riesgo de inyección SQL.** Los 4 módulos de consulta usan bind params
  (`named_params!`/`params!`) en todo texto de usuario; los únicos `format!` ensamblan SQL
  constante o arman el valor de un patrón LIKE que luego se bindea. Todos los `MATCH` FTS
  usan el placeholder ligado a `consulta_fts`, ya escapado por `BusquedaTexto::preparar`.

## Velocidad y eficiencia

- [x] **El WHERE con "flags" dinámicos impide que SQLite use los índices existentes.**
  `src/database/queries/contratistas.rs`, `src/database/queries/ingresos.rs` (Activos e
  Historial). **Severidad alta — confirmado con `EXPLAIN QUERY PLAN` real, no solo teoría.**
  El patrón `(:modo_busqueda = 0 OR (:modo_busqueda = 1 AND ...) OR (:modo_busqueda = 2 AND
  ...))` + `(:empresa_id IS NULL OR c.empresa_id = :empresa_id)` en una sola consulta
  preparada no permitía que SQLite decidiera en tiempo de `prepare` qué rama aplica (depende
  del valor del parámetro en runtime), así que terminaba escaneando todas las filas
  candidatas sin usar `idx_contratistas_empresa` ni
  `idx_registro_ingresos_empresa`/`idx_registro_ingresos_fecha_ingreso` aunque existieran.
  **Reparado (2026-08-20):** los 3 módulos (`contratistas::construir_where`,
  `ingresos::construir_where_activos`, `ingresos::construir_where_historial`) arman el
  `WHERE` sólo con las condiciones realmente activas — igual que ya hacían
  `empresas.rs`/`usuarios.rs` para el modo de búsqueda, extendido aquí a cada filtro
  estructurado (`empresa_id`, `tipo`, `praind`, `personal_ruta`, `tiene_acceso`, `gafete`,
  `medio`, `estado`, `usuario_ingreso`/`usuario_salida`). Regresión cubierta con
  `EXPLAIN QUERY PLAN` real en tests unitarios nuevos:
  `contratistas::tests::filtrar_por_empresa_usa_el_indice`,
  `ingresos::tests::activos_filtrar_por_empresa_usa_el_indice`,
  `ingresos::tests::historial_rango_de_fechas_usa_el_indice` — los tres confirman el nombre
  del índice presente en el plan, no sólo que la consulta "funciona".

- [x] **Historial (y Contratistas) ejecutan la misma consulta compleja dos veces por cada
  tecla.** `src/database/queries/ingresos.rs` (`count_sql`/`select_sql`); mismo patrón en
  `contratistas.rs`. **Severidad alta.** Cada búsqueda evaluaba el mismo predicado de 9+
  condiciones dos veces completas sobre `registro_ingresos`, tabla append-only sin límite de
  crecimiento. **Mitigado, no fusionado (2026-08-20):** el fix de arriba (WHERE indexado en
  vez de full scan) ya reduce el costo real de cada una de las dos ejecuciones de un scan
  completo a una búsqueda por índice — el grueso del costo que motivaba la severidad alta.
  Se evaluó fusionar `COUNT`+`SELECT` en una sola consulta con `COUNT(*) OVER()`
  (ventana), pero se descartó a propósito: si un `offset` queda por encima del total
  filtrado (p. ej. el total se redujo entre una página y la siguiente), la ventana no
  devuelve ninguna fila y el total dejaría de verse — exactamente el tipo de "total
  silenciosamente incorrecto" que esta app evita en otros lados (Contratistas/Historial ya
  reportan el total real, no uno recortado). Mantener dos ejecuciones — ahora ambas
  indexadas — es la opción que no arriesga esa regresión de corrección a cambio de un ahorro
  marginal en una base local de un solo usuario.

- [x] **El full scan en búsquedas `< 3` caracteres es inevitable por diseño, no un error.**
  El tokenizer trigram de FTS5 requiere mínimo 3 caracteres tanto para `MATCH` como para
  acelerar `LIKE`/`GLOB` — es una limitación de SQLite, no configurable. Bajar el umbral de
  `BusquedaTexto` no serviría de nada. El único ángulo de mejora real sería aprovechar el
  índice `UNIQUE` de `cedula` para coincidencias exactas cortas, cosa que hoy no se hace.

- [x] **`empresa:` resuelve el nombre recorriendo todas las empresas en memoria, en cada
  tecla.** `src/tui/{activos,contratistas}/state.rs` y `historial/filtros.rs`. **Severidad
  baja.** Es `O(n)` con una asignación de `String` nueva por empresa comparada (más cara aún
  ahora que también pliega diacríticos), sin índice. **Revisado, no se cambia (2026-08-20):**
  sigue siendo razonable mientras el catálogo sea chico — V1 sólo administra las empresas
  propias del cliente, no un directorio externo que pueda crecer sin control — y ya corre
  detrás del debounce de 250ms, no en cada tecla cruda. Construir un índice en memoria
  (`HashMap` por nombre plegado) sería la solución si el catálogo creciera mucho, pero
  agregar esa complejidad hoy no tiene un problema real que resolver detrás; se deja
  documentado para revisar si algún día cambia la escala del catálogo de empresas.

- [x] **Los límites de paginación son inconsistentes entre módulos.**
  `contratistas.rs`, `empresas.rs`, `usuarios.rs` (100/500) vs `ingresos.rs` (50/200 para
  historial) vs `ingresos.rs` (1000 fijo para Activos, sin offset real). **Severidad baja.**
  Cuatro constantes distintas con el mismo propósito y nombres casi iguales; Contratistas,
  Empresas y Usuarios tenían literalmente el mismo par de valores (100/500) definido tres
  veces. **Reparado (2026-08-20):** unificadas en
  `database::queries::{LIMITE_LISTADO_PREDETERMINADO, LIMITE_LISTADO_MAXIMO}`, importadas
  con alias en los 3 módulos (mismo nombre local `LIMITE_PREDETERMINADO`/`LIMITE_MAXIMO`, un
  solo lugar define el valor). Historial y Activos mantienen sus propias constantes con
  rustdoc explicando por qué son distintas a propósito (Historial pagina en ventanas más
  chicas por ser una tabla append-only de crecimiento indefinido; Activos no pagina — es un
  tope de seguridad, comentario que ya existía).

- [x] **El debounce de 250ms está bien implementado donde existe.** Verificado en las 5
  pantallas que lo tienen (Activos, Contratistas, Empresas, Historial, Usuarios): el patrón
  `marcar()`/`listo()` consume el estado una sola vez por marca, sin duplicar consultas. Ver
  el hallazgo de UX de abajo para las 2 pantallas que NO lo tienen.

## UX / consistencia entre pantallas

- [x] **Nuevo Ingreso y Salida Rápida no tienen debounce — cada tecla golpea la base de
  datos.** `src/tui/nuevo_ingreso/state.rs`, `src/tui/salida_rapida/state.rs`. **Severidad
  media.** A diferencia de las otras 5 pantallas (que marcan el debounce y solo emiten la
  búsqueda real desde `tick()` tras 250ms sin teclear), estas dos disparaban
  `AccionNuevoIngreso::Buscar`/`AccionSalidaRapida::Buscar` de inmediato en cada tecla, y
  `app.rs` ejecutaba la consulta de forma síncrona ahí mismo. **Reparado (2026-08-20):**
  mismo patrón `Debounce` + `tick()` que ya usan Historial/Contratistas/Activos/Empresas/
  Usuarios; `App::run` las agrega a la ronda de `tick()` de cada vuelta del bucle (y al
  helper de tests `agotar_debounce_busquedas`). Cubierto por
  `inicia_vacio_y_busqueda_emite_acciones_tras_el_debounce`
  (`src/tui/nuevo_ingreso/tests.rs`) y
  `escribir_filtra_por_gafete_o_nombre_en_un_solo_campo_tras_el_debounce`
  (`src/tui/salida_rapida/tests.rs`).

- [x] **En Salida Rápida, un solo Esc cierra todo el overlay y descarta lo escrito.**
  `src/tui/salida_rapida/state.rs`. **Severidad baja.** El resto de pantallas usan dos
  etapas (`Esc` con filtro no vacío → solo limpia; `Esc` con filtro vacío → sale/vuelve).
  Salida Rápida no distinguía: un Esc con texto escrito cerraba todo de una. **Reparado
  (2026-08-20):** mismas dos etapas que el resto de la app. Cubierto por
  `esc_con_filtro_escrito_solo_limpia_sin_cerrar` (`src/tui/salida_rapida/tests.rs`).

- [x] **Salida Rápida nunca puede avisar "resultados ocultos".**
  `src/tui/app.rs`, `src/tui/salida_rapida/state.rs` (sin campo `total`). **Severidad baja.**
  `procesar_accion_salida_rapida` descartaba `pagina.total` y sólo guardaba `items`. Nuevo
  Ingreso, sobre una consulta comparable, sí conservaba el total y exponía
  `resultados_ocultos()`. **Reparado (2026-08-20):** `SalidaRapidaState` gana un campo
  `total` y el mismo `resultados_ocultos()` que `NuevoIngresoState`; `app.rs` ya no descarta
  `pagina.total` con `.map(|pagina| pagina.items)`; el hint "N de M — afine la búsqueda" se
  agrega a la etiqueta del campo de búsqueda del overlay.

## Descartado

- **"Sin test que cubra tildes/eñes en el modo LIKE corto"** — refutado: `tests/busqueda_fts.rs`
  ya tiene un caso con `"ía"` (2 caracteres, con tilde, modo LIKE) contra "María Peña". El
  hallazgo original citó el rango de líneas equivocado y pasó por alto ese test.

## Respuesta a "¿esto es el máximo rendimiento o se puede mejorar?"

**Actualizado (2026-08-20):** los dos hallazgos de severidad alta en rendimiento ya están
reparados/mitigados (ver detalle arriba) — `contratistas.rs`/`ingresos.rs` ahora arman el
mismo tipo de SQL por rama que ya usaban `empresas.rs`/`usuarios.rs`, en vez de flags
dinámicos, y las 3 consultas usan sus índices reales (confirmado con `EXPLAIN QUERY PLAN` en
tests, no sólo teoría). Quedan pendientes los hallazgos de severidad media/baja restantes de
este documento (tildes en búsquedas cortas, `empresa:` en memoria, límites de paginación
inconsistentes, y las 3 pantallas de UX de Salida Rápida/Nuevo Ingreso).
