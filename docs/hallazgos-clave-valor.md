# Hallazgos del análisis de clave:valor (2026-08-19)

Análisis de extremo a extremo de la sintaxis `clave:valor` (`empresa:`, `tipo:`, `praind:`,
`gafete:`, etc., estilo GitHub/Gmail) en las 3 pantallas que la implementan (Contratistas,
Activos, Historial), en 4 ángulos en paralelo (multi-agente): UX/consistencia, consultas SQL,
velocidad/eficiencia, y acentos/eñes. Cada hallazgo fue verificado por un segundo agente
independiente leyendo el código real antes de confirmarse.

**Estado (2026-08-20): 16/18 hallazgos confirmados, 2 reparados (1 parcialmente).** 2
hallazgos propuestos se descartaron por evidencia (ver abajo).

El análisis léxico (comillas, negación con guion, separar clave de texto libre) lo hace el
crate externo `query-parser`; cada pantalla tiene su propia función `aplicar_clave` que
interpreta las claves reconocidas y llama a `resolver_terminos` (`src/tui/ui_kit/query_lang.rs`).

## UX / consistencia entre pantallas

- [x] **La negación (`-clave:valor`) solo funciona para `tipo` y `estado`.** Todas las demás
  claves reconocidas (`empresa`, `praind`, `ruta`, `acceso`, `gafete`, `medio`, `desde`,
  `hasta`, `ingreso`, `salida`) tenían el guard `!term.negated`, así que un término negado se
  rechazaba en silencio y caía a texto libre reconstruido con el guion (`-empresa:brisas`),
  que casi nunca matchea nada. **Severidad alta.** **Parcialmente reparado (2026-08-20):**
  `ruta`/`acceso` (Contratistas) y `medio` (Activos) ya soportan negación —
  `src/tui/contratistas/state.rs::aplicar_clave` invierte el booleano (`b != term.negated`);
  `src/tui/activos/state.rs::aplicar_clave` usa `medio_opuesto` (sólo hay 2 variantes de
  `MedioIngreso`, así que negar una da la otra; el patrón exhaustivo deja de compilar si se
  agrega una tercera). **Sigue pendiente** para `empresa`, `gafete`, `desde`, `hasta`,
  `ingreso`, `salida` (Contratistas/Activos/Historial) y `praind` (Contratistas): esas claves
  filtran por igualdad/rango contra SQL (`= :valor`, `LIKE '%valor%'`), no por un booleano o
  enum cerrado ya resuelto en Rust — soportar `-clave:valor` ahí exige agregar `NOT`/`!=` en
  las mismas consultas (`contratistas.rs`, `queries/ingresos.rs`) que ya están marcadas para
  reescribirse por el hallazgo de rendimiento "WHERE con flags dinámicos" en
  `docs/hallazgos-buscador.md` — se decidió no tocar ese SQL dos veces por separado, sino
  resolver ambos juntos cuando se aborde esa reescritura.

- [ ] **Clave con typo o no soportada en la pantalla se busca como texto libre sin ningún
  aviso.** `escribir "empresaa:brisas"` (o `estado:` en Contratistas/Activos, donde esa clave
  no existe) cae al mismo fallback silencioso que el hallazgo anterior — casi siempre 0
  resultados, indistinguible de una búsqueda legítima sin coincidencias.
  `src/tui/ui_kit/query_lang.rs:82-98`; `contratistas/state.rs:114`; `activos/state.rs:83`;
  `historial/filtros.rs:218`. **Severidad media.**

- [ ] **En Historial, la etiqueta de búsqueda oculta la negación de `tipo`/`estado`.** Cuando
  el filtro viene de `-tipo:swat`, el estado interno ya guarda el complemento positivo
  (`[Praind, InHouse, PorCorreo]`), así que el resumen muestra `"tipo: PRAIND o IN HOUSE o POR
  CORREO"` en vez de `"tipo: no SWAT"` — se pierde toda traza de que hubo negación.
  `src/tui/historial/render.rs:118-120`, `historial/filtros.rs:147-162`. **Severidad media.**
  (Contratistas/Activos no arman resumen alguno, así que ahí no aplica este problema puntual.)

- [ ] **La ayuda F1 no documenta sintaxis de valores, solo nombres de clave.** Ninguna de las
  3 pantallas menciona en `ayuda_extra` los valores aceptados (`praind:vence|vencido|sin`,
  `ruta:si|no`, `medio:caminando|vehiculo`, `estado:activos|cerrados|todos`), ni las listas por
  coma, ni la negación con guion. `contratistas/render.rs:87`; `activos/render.rs:74`;
  `historial/render.rs:49`. **Severidad baja.**

- [x] **`empresa`/`tipo` tienen exactamente la misma lógica en las 3 pantallas — no hay
  divergencia adicional oculta.** Verificado sin diferencias sutiles más allá de lo ya
  documentado. Usuarios/Empresas podrían beneficiarse de `clave:valor` (`rol:administrador`,
  `activo:si`) pero el dominio es chico y el beneficio marginal; Nuevo Ingreso/Salida Rápida
  no lo necesitan por ser flujos de captura, no de exploración de listas. No es un bug.

## Consultas / SQL

- [x] **`ContratistasState.hoy` se calcula una sola vez al arrancar la app y nunca se
  refresca.** Alimenta tanto `praind:vence`/`praind:vencido`/`praind:sin` como el coloreado de
  vencimiento en pantalla; el único setter (`set_hoy`) solo lo llamaban los tests.
  `src/tui/contratistas/state.rs:334` (`impl Default`), `341-343` (`set_hoy`); nunca invocado
  desde `app.rs`. **Severidad alta.** **Reparado (2026-08-20):** `ContratistasState::tick`
  ahora refresca `self.hoy = ahora_costa_rica().date_naive()` en cada vuelta (antes de
  revisar el debounce), igual que el resto del estado vivo — una sesión que cruza medianoche
  ya no queda congelada en la fecha de arranque.

- [x] **Historial no tiene una clave `praind:` — la comparación pedida no aplica.** Solo existe
  `tipo:praind` como valor de la clave `tipo`. Historial calcula su propio `hoy` una vez, pero
  solo lo usa para el rango de fechas por defecto, no para vencimientos relativos, así que no
  tiene el mismo bug.

- [x] **Sin riesgo de inyección SQL en los valores resueltos de clave:valor.** Todos los
  valores (`empresa_id`, `t0..t3`, `praind_hoy`, `gafete_numero`, etc.) llegan como parámetros
  bindeados (`named_params!`); los únicos `format!` interpolan constantes fijas del propio
  módulo, no datos de usuario.

## Velocidad y eficiencia

- [x] **`resolver_terminos`/`aplicar_clave`/`query_parser::parse` solo corren tras el debounce
  de 250ms, nunca por cada tecla.** Verificado en las 3 pantallas — el manejador de tecla
  normal solo hace `handle_key` + marcar debounce; el parseo real solo se dispara desde
  `tick()` o acciones discretas. Diseño ya correcto, sin hallazgo.

- [x] **Re-parsear la consulta completa por búsqueda no es un problema real.** Cadena corta
  (típicamente <100 caracteres), a lo sumo ~4 veces/segundo durante tecleo continuo — órdenes
  de magnitud por debajo del costo de la consulta SQL posterior.

- [x] **La resolución de `empresa:` (O(n) en memoria) corre una sola vez por búsqueda, sin
  duplicación.** No hay una segunda pasada de "validar" y luego "aplicar" en ninguna pantalla.

- [x] **Combinar varias claves a la vez no empeora el plan SQL.** El WHERE es texto estático
  sin importar cuántas claves estén activas — el costo ya está fijado por el problema general
  de flags dinámicos (documentado en `docs/hallazgos-buscador.md`), no se agrava aquí.

- [x] **No hay asignaciones evitables en el camino caliente por tecla.** Las listas de
  empresas se pasan siempre por referencia; los únicos clones (`FiltrosHistorial::clone()`,
  `valores(): Vec<String>`) son de datos pequeños y ocurren solo una vez por búsqueda real,
  gateados por el debounce.

## Acentos y eñes

- [x] **Los enums cerrados (`tipo`, `praind`, `ruta`, `acceso`, `medio`, `estado`) ya
  contemplan tildes como literales explícitos** (`"próximo"`, `"sí"`, `"vehículo"`, `"salió"`),
  así que escribir con o sin acento da el mismo resultado. No es un bug.

- [ ] **`ingreso:`/`salida:` en Historial no normalizan tildes en ningún lado (ni Rust ni
  SQL) — mecanismo distinto y aparte del ya conocido de `empresa:`.** El valor crudo pasa
  intacto a `LIKE '%valor%' COLLATE NOCASE`; `COLLATE NOCASE` solo pliega ASCII A-Z, no
  diacríticos. `historial/filtros.rs:210-217`; `src/database/queries/ingresos.rs:232-233,380-381`.
  **Severidad media.** Caso de prueba: usuario "María José" — `salida:jose` sí matchea (sin
  tilde en ambos lados), pero `salida:josé` o `ingreso:maria` no matchean si el nombre guardado
  tiene/no tiene tilde en la posición contraria.

- [x] **Las listas (`clave:a,b,c`) no tienen el mismo gap que `empresa:` porque solo existen
  para `tipo`/`estado` (enums cerrados sin acentos reales de entrada).** Las claves de texto
  libre (`empresa`, `ingreso`, `salida`) están bloqueadas explícitamente para listas
  (`valores.len() == 1`), así que no hay ruta de código de listas que compare texto libre con
  tildes.

## Descartado

- **"`desde:`/`hasta:` sobrescriben un formulario de fecha ajustado por otro control, sin
  aviso visual."** Refutado: no existe tal formulario/control alterno para fecha — el único
  mecanismo para fijar `desde`/`hasta` es la misma sintaxis `clave:valor` (tecleada o vía el
  atajo del heatmap, que arma la misma sintaxis internamente). El detalle técnico de "acepta
  cualquier texto sin validar formato hasta `construir()`, con mensaje genérico" es real, pero
  la premisa de impacto ("pierde un valor que el usuario fijó por otra vía") no aplica a este
  código.

- **"`tipo:a,b,c,d,e` con más de 4 valores nunca pierde un valor distinto, solo recorta
  duplicados."** Refutado con contraejemplo: `tipo:praind,praind,praind,praind,swat` (4
  duplicados de PRAIND al inicio + SWAT al final) sí pierde SWAT en silencio, porque el `zip`
  de tamaño fijo 4 corta por posición/índice, no por unicidad — no hay `dedup` en `valores()`.
  Bug real de bajo impacto (filtro de búsqueda en UI, no corrupción de datos), pendiente de
  reportar aparte si se decide corregir.

## Respuesta a "¿estoy obteniendo el máximo rendimiento o puede mejorar?"

**No aplica un veredicto de rendimiento aparte para clave:valor.** Este análisis es de
consistencia/UX del sistema de claves, no de rendimiento — los dos hallazgos de rendimiento
real (falta de índices por flags dinámicos, consulta duplicada por tecla) ya están
documentados en `docs/hallazgos-buscador.md` y son los mismos para cualquier búsqueda,
tenga o no `clave:valor`; no se repiten aquí. El propio sistema clave:valor (parseo léxico,
resolución de `empresa:` en memoria, listas) no le agrega costo relevante — todo lo revisado
en esa dimensión salió confirmado como "no es un problema" (ver arriba).
