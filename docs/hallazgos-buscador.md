# Hallazgos del análisis del buscador (2026-08-19)

Análisis de extremo a extremo del buscador de texto libre y del filtro `clave:valor`,
en 4 ángulos en paralelo (multi-agente): UX/consistencia entre pantallas, consultas SQL,
velocidad/eficiencia, y manejo de acentos/eñes. Cada hallazgo fue verificado por un
segundo agente independiente leyendo el código real antes de confirmarse — no se reporta
nada que no se haya comprobado abriendo el archivo citado.

**Estado (2026-08-19): 17/18 hallazgos confirmados, 0 reparados.** Queda pendiente decidir
prioridad y reparar. 1 hallazgo propuesto se descartó por evidencia (ver abajo).

El buscador es compartido: `src/database/search.rs` (`BusquedaTexto::preparar`) clasifica
cualquier texto en 3 modos — vacío (sin filtro), `< 3` caracteres (`LIKE '%texto%' COLLATE
NOCASE`), `>= 3` caracteres (FTS5 trigram con `remove_diacritics 1`). Lo usan los 4 módulos
de consulta (`contratistas`, `empresas`, `usuarios`, `ingresos` — este último cubre Activos,
Historial y Salida Rápida). Aparte del texto libre, varias pantallas soportan sintaxis
`clave:valor` (`empresa:`, `tipo:`, `praind:`, `gafete:`, etc.) resuelta en Rust, no en SQL,
vía `ui_kit/query_lang.rs` + una función `aplicar_clave` por pantalla.

## Acentos y eñes

- [ ] **`empresa:alvarez` no encuentra "Álvarez Ingeniería".** `src/tui/contratistas/state.rs:52-56`
  (idéntico en `activos/state.rs:35-39` y `historial/filtros.rs:134-138`). **Severidad alta.**
  El filtro `empresa:` compara `e.nombre.to_lowercase().contains(&buscado)` en Rust puro —
  `to_lowercase()` solo pliega mayúsculas Unicode, no quita tildes, así que `"álvarez".contains("alvarez")`
  es `false` carácter a carácter (`á` U+00E1 nunca es `a` U+0061). El buscador de texto libre
  de la misma pantalla sí resuelve esto bien (FTS con `remove_diacritics=1`) — la
  inconsistencia es solo del filtro estructurado `empresa:`, copiado igual en 3 archivos.
  Sugerencia: normalizar diacríticos en ambos lados antes de comparar, con una función
  compartida en vez de triplicar la lógica.

- [ ] **Búsquedas de 1-2 caracteres no pliegan tildes ni Ñ.** `src/database/search.rs:22-30`
  (consumido con `COLLATE NOCASE` en `contratistas.rs:98-100,182-183`, `empresas.rs:56-71`,
  `usuarios.rs:59-85`, `ingresos.rs:326-328,364-365,380-381`). **Severidad media.**
  `COLLATE NOCASE` en SQLite solo pliega ASCII A-Z; buscar "os" (2 caracteres) no encuentra
  "Óscar", pero "osc" (3+, ya en modo FTS) sí. Alcance acotado: nombres completos casi
  siempre superan 3 caracteres y las cédulas son numéricas, así que el disparo más plausible
  es una búsqueda corta por apellido parcial — infrecuente pero real.

- [x] **FTS ya funde ñ/n a propósito — no es un bug.** `src/database/schema.rs:373-386,540-543`.
  `remove_diacritics=1` en las 4 tablas FTS5 hace que "nino" encuentre "Niño" y viceversa
  (confirmado también por el test existente `contratistas_busca_subcadenas_sin_distinguir_tildes_o_mayusculas`).
  Es una decisión razonable de tolerancia a errores de tipeo para nombres reales, no un
  descuido — vale la pena dejar un comentario en `schema.rs` aclarando que es intencional,
  para que nadie lo "corrija" pensando que es un error.

## Consultas / SQL

- [ ] **Historial no encuentra por número de gafete en texto libre; Activos sí.**
  `src/database/queries/ingresos.rs` — `HISTORIAL_FROM_WHERE` (356-382) vs `ACTIVOS_SQL`
  (312-346). **Severidad media.** `ACTIVOS_SQL` compara `:numero_exacto` contra
  `gafete_numero` en ambos modos (LIKE y FTS); `HISTORIAL_FROM_WHERE` nunca lo hace — solo
  funciona con la clave explícita `gafete:26`. Un usuario que aprende en Activos que teclear
  "26" encuentra al portador del gafete 26 se lleva sorpresa en Historial.

- [ ] **`gafete:abc` (no numérico) se comporta distinto en Activos e Historial.**
  `src/tui/activos/state.rs:69-75` vs `src/tui/historial/filtros.rs:198-201,66-74`.
  **Severidad media.** En Activos, un valor no numérico se ignora en silencio (el término
  queda como texto libre). En Historial, `construir()` devuelve
  `Err("Ingrese un número de gafete válido")` y bloquea la búsqueda. Mismo input, dos
  experiencias.

- [ ] **El "total" de Activos ignora el filtro aplicado.** `src/database/queries/ingresos.rs:172-177`
  vs `191-211` (`ACTIVOS_SQL`). **Severidad baja.** El `COUNT(*)` de `listar_activos` es una
  cadena fija (`WHERE fecha_hora_salida IS NULL`) sin ninguno de los parámetros de búsqueda;
  `items` sí está filtrado. Contraste con Contratistas/Historial, donde el `COUNT` reutiliza
  literalmente el mismo `WHERE` que el `SELECT`. Hoy es coherente con lo que muestra la UI
  ("`N` DE `M` DENTRO" = filtrados de totales), pero no está documentado como decisión
  deliberada — riesgo de que un cambio futuro lo "unifique" sin querer y rompa esa semántica.

- [x] **Sin riesgo de inyección SQL.** Los 4 módulos de consulta usan bind params
  (`named_params!`/`params!`) en todo texto de usuario; los únicos `format!` ensamblan SQL
  constante o arman el valor de un patrón LIKE que luego se bindea. Todos los `MATCH` FTS
  usan el placeholder ligado a `consulta_fts`, ya escapado por `BusquedaTexto::preparar`.

## Velocidad y eficiencia

- [ ] **El WHERE con "flags" dinámicos impide que SQLite use los índices existentes.**
  `src/database/queries/contratistas.rs:92-124` (`CONTRATISTAS_FROM_WHERE`);
  `src/database/queries/ingresos.rs:312-346` (`ACTIVOS_SQL`) y `356-382`
  (`HISTORIAL_FROM_WHERE`). **Severidad alta — confirmado con `EXPLAIN QUERY PLAN` real, no
  solo teoría.** El patrón `(:modo_busqueda = 0 OR (:modo_busqueda = 1 AND ...) OR (:modo_busqueda
  = 2 AND ...))` + `(:empresa_id IS NULL OR c.empresa_id = :empresa_id)` en una sola consulta
  preparada no permite que SQLite decida en tiempo de `prepare` qué rama aplica (depende del
  valor del parámetro en runtime), así que termina escaneando todas las filas candidatas sin
  usar `idx_contratistas_empresa` ni `idx_registro_ingresos_empresa`/`idx_registro_ingresos_fecha_ingreso`
  aunque existan. Contraste: `empresas.rs`/`usuarios.rs` sí arman SQL distinto por rama
  (`match busqueda.modo { ... }`) y por tanto sí pueden aprovechar índice/FTS directamente.

- [ ] **Historial (y Contratistas) ejecutan la misma consulta compleja dos veces por cada
  tecla.** `src/database/queries/ingresos.rs:247-274` (`count_sql`/`select_sql`, ambos sobre
  `HISTORIAL_FROM_WHERE`); mismo patrón en `contratistas.rs:172-185`. **Severidad alta.**
  Cada búsqueda evalúa el mismo predicado de 9+ condiciones dos veces completas sobre
  `registro_ingresos`, tabla append-only sin límite de crecimiento. `empresas.rs`/`usuarios.rs`
  no calculan total en absoluto (una sola consulta) — inconsistencia de enfoque entre
  pantallas, además del costo.

- [x] **El full scan en búsquedas `< 3` caracteres es inevitable por diseño, no un error.**
  El tokenizer trigram de FTS5 requiere mínimo 3 caracteres tanto para `MATCH` como para
  acelerar `LIKE`/`GLOB` — es una limitación de SQLite, no configurable. Bajar el umbral de
  `BusquedaTexto` no serviría de nada. El único ángulo de mejora real sería aprovechar el
  índice `UNIQUE` de `cedula` para coincidencias exactas cortas, cosa que hoy no se hace.

- [ ] **`empresa:` resuelve el nombre recorriendo todas las empresas en memoria, en cada
  tecla.** `src/tui/{activos,contratistas}/state.rs` y `historial/filtros.rs` (líneas ~35-59
  según archivo). **Severidad baja.** Es `O(n)` con una asignación de `String` nueva por
  empresa comparada, sin índice — razonable mientras el catálogo sea chico (V1 solo
  contratistas/empresas propias), pero no escalaría bien si creciera mucho.

- [ ] **Los límites de paginación son inconsistentes entre módulos.**
  `contratistas.rs:9-10`, `empresas.rs:5-6`, `usuarios.rs:7-8` (100/500) vs `ingresos.rs:11-12`
  (50/200 para historial) vs `ingresos.rs:17` (1000 fijo para Activos, sin offset real).
  **Severidad baja.** Cuatro constantes distintas con el mismo propósito y nombres casi
  iguales; Activos es intencionalmente sin paginación (comentario propio en el código lo
  confirma), pero vale unificar criterio en los demás.

- [x] **El debounce de 250ms está bien implementado donde existe.** Verificado en las 5
  pantallas que lo tienen (Activos, Contratistas, Empresas, Historial, Usuarios): el patrón
  `marcar()`/`listo()` consume el estado una sola vez por marca, sin duplicar consultas. Ver
  el hallazgo de UX de abajo para las 2 pantallas que NO lo tienen.

## UX / consistencia entre pantallas

- [ ] **Nuevo Ingreso y Salida Rápida no tienen debounce — cada tecla golpea la base de
  datos.** `src/tui/nuevo_ingreso/state.rs:179-188` y `src/tui/salida_rapida/state.rs:128-138`.
  **Severidad media.** A diferencia de las otras 5 pantallas (que marcan el debounce y solo
  emiten la búsqueda real desde `tick()` tras 250ms sin teclear), estas dos disparan
  `AccionNuevoIngreso::Buscar`/`AccionSalidaRapida::Buscar` de inmediato en cada tecla, y
  `app.rs` ejecuta la consulta de forma síncrona ahí mismo. Escribir rápido dispara una
  consulta por letra en vez de agrupar.

- [ ] **En Salida Rápida, un solo Esc cierra todo el overlay y descarta lo escrito.**
  `src/tui/salida_rapida/state.rs:109-112`. **Severidad baja.** El resto de pantallas usan
  dos etapas (`Esc` con filtro no vacío → solo limpia; `Esc` con filtro vacío → sale/vuelve).
  Salida Rápida no distingue: un Esc con texto escrito cierra todo de una, rompiendo la
  convención que el usuario ya aprendió en el resto de la app.

- [ ] **Salida Rápida nunca puede avisar "resultados ocultos".**
  `src/tui/app.rs:1450-1463` y `src/tui/salida_rapida/state.rs` (sin campo `total`).
  **Severidad baja.** `procesar_accion_salida_rapida` descarta `pagina.total` y solo guarda
  `items`. Nuevo Ingreso, sobre una consulta comparable, sí conserva el total y expone
  `resultados_ocultos()` para avisar "N de M — afine la búsqueda". Si algún día hay más de
  1000 ingresos activos coincidentes, Salida Rápida no avisaría que hay más.

## Descartado

- **"Sin test que cubra tildes/eñes en el modo LIKE corto"** — refutado: `tests/busqueda_fts.rs`
  ya tiene un caso con `"ía"` (2 caracteres, con tilde, modo LIKE) contra "María Peña". El
  hallazgo original citó el rango de líneas equivocado y pasó por alto ese test.

## Respuesta a "¿esto es el máximo rendimiento o se puede mejorar?"

Se puede mejorar, y los dos hallazgos de severidad alta en rendimiento (WHERE con flags
dinámicos sin índice, y la consulta duplicada por tecla en Historial/Contratistas) son los
de mayor impacto real, confirmados con `EXPLAIN QUERY PLAN` — no es solo teoría. El patrón
correcto ya existe en el propio código (`empresas.rs`/`usuarios.rs` arman SQL distinto por
modo en vez de flags dinámicos) — sería cuestión de aplicar el mismo criterio a
`contratistas.rs`/`ingresos.rs`.
