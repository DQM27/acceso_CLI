# Pendientes

Documento único de trabajo pendiente en el repositorio. Reemplaza a las auditorías y
planes previos (`hallazgos-*.md`, `auditoria-*.md`, `plan-*.md` de saneamiento/respaldos/
autorización, `refactor-app-y-errores.md`), cuyo contenido ya cerrado se consolidó aquí o
se descartó. Ese historial sigue disponible en `git log`/`git show` de los commits que los
crearon si hace falta el detalle completo de un hallazgo ya reparado.

`docs/diagrama-logico.md` y `docs/evaluacion-sqlite.md` siguen aparte: son referencia de
arquitectura y configuración, no rastreadores de tareas.

## Regla de este documento

**Al terminar una tarea de aquí, se marca `[x]` en el mismo commit (o el siguiente) que la
resuelve — no se deja para una limpieza posterior.** Si una tarea se descarta en vez de
hacerse, se marca `[x]` igual con una nota de por qué se descartó (no se borra la línea:
que quede el rastro de la decisión).

---

## Refactor en curso — `refactor/app-y-errores`

Contexto: `src/tui/app.rs` era el archivo más grande del repo (2.971 líneas). Fases 1 y 2
completas (pruebas y errores extraídos, quedó en 1.556 líneas). Fases 3-5 pendientes, en
este orden:

- [x] **Fase 3 — Extraer trabajos asincrónicos (2026-08-21).** Movidos a
  `src/tui/app/auth_jobs.rs`: los 4 flujos de Argon2 (login, ROOT inicial, crear
  usuario, cambiar contraseña administrativa/propia), sus tipos (`HiloUsuarioPendiente`,
  `DatosUsuarioPendiente`, `ReceptorHash`/`ReceptorCambioPropio`/`ReceptorAutenticacion`)
  y `finalizar_hilos_pendientes`. Movimiento mecánico — mismo `impl App` en otro archivo,
  sin cambiar lógica; los campos de estado async se quedaron en el `struct App` de
  `app.rs`. `app.rs` pasó de 1.556 a 1.189 líneas. `cargo fmt`, Clippy estricto
  (`-D warnings`) y la suite completa en verde.
- [x] **Fase 4 — Agrupar despachadores por área (2026-08-21).** Movidos a
  `src/tui/app/actions/{accesos,catalogos,admin}.rs`: accesos (Activos, Historial, Nuevo
  Ingreso, Salida Rápida), catálogos (Contratistas, Empresas), administración (Usuarios,
  Auditoría, Respaldos). Métodos marcados `pub(in crate::tui::app)` para que `app.rs`
  (ancestro del submódulo `actions`) siga pudiendo llamarlos — visibilidad de Rust sólo da
  acceso automático a descendientes, no a ancestros. `app.rs` pasó de 1.189 a 620 líneas.
  `cargo fmt`, Clippy estricto y la suite completa en verde.
- [x] **Fase 5 — Separar navegación y runtime: descartada por decisión (2026-08-21).**
  `app.rs` ya quedó en 620 líneas tras las Fases 3-4 — dejó de ser el hotspot que
  justificaba el refactor original (2.971 líneas). Partirlo más en `navigation.rs`/
  `runtime.rs` tendría retorno decreciente: cada pieza restante (loop de render,
  navegación global, sesión) ya es chica y de propósito claro. Se prioriza
  `application.rs` (965 líneas, más señal real de necesitar el corte) en su lugar.

Riesgos a conservar bajo prueba durante el refactor: una vista autenticada no debe quedar
activa sin sesión; F2 debe refrescar Activos/Historial/Nuevo Ingreso según la vista debajo
del overlay; los hilos de Argon2 no deben bloquear el loop ni perder una escritura
validada al cerrar; la restauración sólo devuelve `SalidaApp::Restaurar`; el modo sin
`AppCore` no debe dejar formularios esperando indefinidamente. Cada corte: `cargo fmt` +
suite completa + Clippy estricto.

## Siguiente candidato tras `app.rs`: repartir `AppCore`

- [x] **Repartido (2026-08-21).** `src/application.rs` (965 líneas) se repartió en
  `src/application/{mod,autenticacion,accesos,catalogos,usuarios,respaldos,historial}.rs`
  (61-291 líneas cada uno), API pública sin cambios — `mod.rs` conserva la estructura,
  construcción, `Drop`, y el único helper realmente transversal
  (`verificar_actor_activo`, usado por 4 de los 6 submódulos). Los demás helpers
  privados (`en_transaccion_con_reloj_validado`, `validar_reloj`,
  `verificar_operador_activo`, `verificar_creacion_usuario`, `verificar_gestion_usuario`,
  `establecer_empresa_activa`, `establecer_usuario_activo`, los de respaldos) se movieron
  junto con su único grupo consumidor en vez de quedar en `mod.rs`. `cargo fmt`, Clippy
  estricto y la suite completa en verde.

## Otros archivos grandes (orden sugerido, después de lo anterior)

- [x] **Repartido (2026-08-21).** `src/database/queries/ingresos.rs` (807 líneas) se
  separó en `ingresos/{mod,activos,historial}.rs` (144/313/438 líneas). `mod.rs` conserva
  el trait `IngresosQuery`, `SqliteIngresosQuery` (que delega cada método a su
  submódulo — un `impl Trait` no se puede partir entre archivos) y los conversores de
  fila que ambas consultas comparten (`resultado_desde_fila`, `motivo_desde_fila`,
  `tipo_desde_fila`, `medio_desde_fila`, `fecha_hora_desde_fila`); cada submódulo se
  quedó con su propio `WHERE` dinámico, conversor específico y pruebas de plan de
  consulta (`EXPLAIN QUERY PLAN`). `cargo fmt`, Clippy estricto y la suite completa en
  verde.
- [x] **Repartido (2026-08-21).** `src/tui/contratistas/state.rs` (934 líneas) →
  `query.rs` (lenguaje `clave:valor`, 120 líneas), `form.rs` (validación y construcción
  del formulario, 114 líneas), `state.rs` queda en 737. `FormularioContratista` y sus
  enums (`CampoFormulario`/`ModoFormulario`/`Desplegable`) se quedaron en `state.rs` a
  propósito: `render.rs` lee sus campos privados directamente, y moverlos habría exigido
  `pub(in ...)` en cada campo sólo para separar código sin beneficio real. Sólo se
  extrajeron las funciones libres que no tienen ese acoplamiento
  (`construir`/`convertir_actualizacion`/`mover_campo`/`agregar_fecha`/`tipos`/
  `texto_tipo`, con `tipos`/`texto_tipo` visibles también para `render.rs` vía
  `pub(in crate::tui::contratistas::state)`, ya que ese archivo los usa directo). `cargo
  fmt`, Clippy estricto y la suite completa en verde.
- [x] **Repartido (2026-08-21).** `src/tui/usuarios/state.rs` (870 líneas) → `form.rs`
  (validación y selector de rol, 42 líneas), `password.rs` (regla de validación de
  contraseña compartida por crear-usuario y cambiar-contraseña, 17 líneas), `state.rs`
  queda en 833. Igual que en Contratistas: `Secreto`/`FormularioUsuario`/
  `FormularioPassword` se quedan en `state.rs` porque `render.rs` lee sus campos
  privados directamente — sólo se extrajeron las funciones libres sin ese acoplamiento
  (`ROLES`/`texto_rol`/`si_no` visibles también para `render.rs` vía
  `pub(in crate::tui::usuarios::state)`). `cargo fmt`, Clippy estricto y la suite
  completa en verde.
- `src/database/schema.rs` (804 líneas) y los `render.rs` de 500-600 líneas: revisados, sin
  acción urgente — cohesión aceptable. Reevaluar sólo si agregar una migración o una
  sección visual nueva empieza a doler.

## Agregados de dominio con campos públicos — diferido a propósito hasta V3

- [ ] **`NuevoRegistroIngreso` construible con campos públicos**
  (`src/models/registro_ingreso.rs`). Nada en producción lo construye fuera de
  `RegistroIngresoService::registrar_entrada`, pero nada del tipo lo impide tampoco.
- [ ] **`Contratista` con campos públicos, incluido el derivado `empresa_activa`**
  (`src/models/contratista.rs`). `verificar_acceso` confía en ese booleano sin que el tipo
  lo proteja.

  Corrección de fondo para ambos: mover a agregados de dominio con constructor privado
  (`crear`/`actualizar`/`evaluar_acceso`), para que los servicios orquesten en vez de
  recordar cada invariante. **Decisión ya tomada: no es prioridad hoy.** La app es de
  instancia única y un solo hilo — la protección sería cosmética porque nada en el camino
  real la viola. Retomar cuando se diseñe **concurrencia multi-terminal (V3)**, no antes.

## Respaldo automático

- [x] **Reparado (2026-08-21): el respaldo automático corre a la 01:00 (hora Costa Rica),
  no a cualquier hora del día.** Antes se disparaba apenas la app arrancaba, sin importar
  la hora — si abrías a las 9 AM, el respaldo del día quedaba sellado a las 9 AM. Ahora
  `respaldo_automatico_diario_si_hace_falta` (`application/respaldos.rs`) no hace nada
  antes de la 01:00 Costa Rica. **Bug real encontrado y reparado en el mismo cambio:** el
  chequeo sólo se evaluaba una vez, al arrancar el proceso (`main.rs`) — si la app se
  queda abierta varios días seguidos sin reiniciar (el caso normal, "la app siempre está
  abierta"), el respaldo de un día nuevo nunca se disparaba. Se agregó una revisión
  periódica (cada 60s) dentro del bucle de la TUI (`tui/app.rs::run_internal`) para que
  también corra mientras la app sigue corriendo, no sólo al abrir. Si la app estuvo
  cerrada cuando pasó la 01:00, se sigue capturando apenas se vuelve a abrir (ya
  funcionaba así). Pruebas nuevas en `tests/configuracion_respaldos.rs` cubren el límite
  de hora exacto (antes/después de la 01:00).
- [x] **Retención de 30 días — evaluado, se deja en 7.** La retención actual es por
  *cantidad* de archivos automáticos (`RETENCION_AUTOMATICOS`), no por fecha real — con
  el respaldo corriendo ~1 vez/día, ambas nociones coinciden en el uso normal, pero
  divergen si la app estuvo cerrada varias semanas (el conteo de "últimos N archivos" no
  es lo mismo que "últimos N días calendario" si hay huecos). Decisión del usuario:
  mantener el criterio por cantidad, y **7 ya es suficiente** — no se cambia.
- [ ] **Omitido a propósito: importar un respaldo en una instalación sin base de datos
  detectada.** La base ya es portátil hoy sin cambios de código (`journal_mode = DELETE`,
  un solo archivo `.db`, sin `-wal`/`-shm`) — copiarla a otra máquina con ambas apps
  cerradas ya funciona. Lo que falta es la UX: hoy, si no hay base, la app crea una vacía
  en silencio y va directo a "Configuración Inicial" (crear ROOT), sin ofrecer nunca
  importar un respaldo existente. El usuario pidió omitirlo por ahora ("no quiero entrar
  en cosas tan complejas"). Si se retoma, la restauración ya validada
  (`database::backup::restaurar_respaldo`) es reutilizable casi tal cual — falta la
  pantalla previa al login que detecte "no existe archivo" y ofrezca importar vs. crear
  nueva.
- [x] **Reparado (2026-08-21): el fallo del respaldo automático ya no queda en silencio.**
  Solución en dos fases, según lo pedido: **Fase 1** — si
  `respaldo_automatico_diario_si_hace_falta` falla, el Menú Principal muestra un aviso
  genérico ("Fallo en el sistema de respaldo de la base de datos. Contacte al
  administrador.") en la barra de estado, para cualquier rol — no en Login, porque una
  sesión puede pasar días sin que nadie inicie sesión. **Fase 2** — el motivo exacto (se
  reutilizan los mensajes de `RespaldoError`, p.ej. "disco lleno") sólo se muestra en la
  pantalla Respaldos (sólo Root). No se construyó ningún sistema de log persistente: el
  estado vive en memoria (`MenuPrincipalState::fallo_respaldo_automatico` y
  `RespaldosState::fallo_automatico`), se actualiza en cada revisión periódica (60s, en
  `tui/app.rs::run_internal`) y se reemplaza en el próximo intento (éxito u otro fallo).
  `respaldo_automatico_diario_si_hace_falta` ahora devuelve `EstadoRespaldoAutomatico`
  (`SinCambios`/`Creado`/`Fallo(mensaje)`) en vez de descartar el resultado. La limpieza
  por retención sigue sin reportar sus propios errores por separado (se ignora con `let
  _ =`, como antes) — no se consideró necesario para este alcance.

## Búsqueda `clave:valor` e Historial

- [x] **Reparado (2026-08-21): Historial no avisaba de clave no reconocida.** Se agregó
  `filtros::resumen_consulta` (usa `resolver_terminos_detallado`, igual que Contratistas/
  Activos) y `render.rs::etiqueta_busqueda` ahora lo llama en vez de leer directo de
  `state.filtro_aplicado` — de paso corrige que la etiqueta nunca reflejaba `tipo`/
  `estado`/`ingreso`/`salida` realmente tecleados, porque nada en producción escribía en
  `filtro_aplicado` (sólo los tests lo tocaban a mano). La consulta SQL real no cambió.
- [x] **Reparado (2026-08-21): Historial ocultaba la negación de `tipo`.** Nuevo campo
  `FiltrosHistorial::tipos_negado`; `resumen_consulta` reconstruye la lista original
  (complemento del complemento) y muestra `"tipo: no SWAT"` en vez de `"tipo: PRAIND o IN
  HOUSE o POR CORREO"`. `estado` se dejó tal cual a propósito: negar uno de sus 2 valores
  reales ya produce el otro valor exacto y sin ambigüedad (`-estado:cerrados` → `Activos`
  es la respuesta correcta, no hay información que se pierda al mostrarla en positivo) —
  a diferencia de `tipo`, donde negar 1 de 4 exige leer una lista de 3 para inferirlo.
- [x] **Reparado (2026-08-21): F1 no documentaba sintaxis de valores.** Las 3 pantallas
  (`contratistas`, `activos`, `historial`) ahora listan los valores aceptados por clave
  (`praind:vence|vencido|sin`, `ruta:si|no`, `medio:caminando|vehiculo`,
  `estado:activos|cerrados|todos`, formato de fecha, etc.) y mencionan la negación con
  guion en `ayuda_extra`.
- [x] **Reparado (2026-08-21): `ingreso:`/`salida:` en Historial no plegaban tildes.**
  `database/queries/ingresos/historial.rs` cambió `LIKE ... COLLATE NOCASE` (sólo pliega
  ASCII) por `PLEGAR(...) LIKE PLEGAR(...)` — misma función SQL que ya usan `empresa:`/
  texto libre. "María José" — `salida:jose` y `salida:josé` ahora matchean igual.
  **Efecto secundario encontrado y reparado en el mismo cambio:** `usuario_salida_nombre`
  es la primera columna nullable a la que se le aplica `PLEGAR` — la función nunca había
  manejado `NULL` (siempre se usó antes sólo con columnas `NOT NULL`) y rompía toda la
  consulta con "Invalid function parameter type Null" en vez de simplemente excluir la
  fila, como sí hacía `LIKE` con `NULL` de forma nativa. `registrar_funcion_plegar`
  (`src/database/schema.rs`) ahora toma `Option<String>` y devuelve `None` si la entrada
  es `NULL` — semántica SQL estándar, detectado por
  `tests/ingreso_queries.rs::historial_filtra_por_quien_dio_ingreso_y_quien_dio_salida`
  (ya existente, no hizo falta un test nuevo).

## Catálogo de gafetes (`docs/plan-gafetes.md`)

Plan aprobado el 2026-08-22, implementado el 2026-08-30 — antes `gafete_numero` en
`registro_ingresos` era un `INTEGER` libre sin relación con los gafetes físicos reales,
sin forma de sacar de circulación uno perdido ni de saber quién lo debe.

- [x] **Núcleo (2026-08-30).** `MIGRACION_14` (el plan pedía `MIGRACION_13`, ya ocupada por
  la generalización de auditoría): tablas `gafetes` (estado DISPONIBLE/PERDIDO/DE_BAJA +
  deudor) y `gafetes_incidentes` (historial append-only). `GafeteService` valida las
  transiciones (Disponible ↔ Perdido, Disponible → DeBaja) y el alta individual/por rango
  (tope defensivo de 200, atómica — si un número del rango falla, ninguno queda creado).
  `RegistroIngresoService` gana un tercer genérico (`GafeteRepository`): `registrar_entrada`
  valida el catálogo (no existe/perdido/de baja) antes del chequeo de ocupación existente, y
  `PreparacionIngreso.gafetes_deuda` viaja no bloqueante. Tocó los ~40 call sites de
  `RegistroIngresoService::new(...)` (3 en `application/accesos.rs`, 37 repartidos en 4
  archivos de test) más fixtures de gafete en los tests que registran entradas reales.
  `AppCore::gafetes` (fachada) sin restricción de rol a propósito — decisión explícita del
  usuario, a diferencia de Empresas/Usuarios: cualquier operador con sesión gestiona el
  catálogo completo.
- [x] **TUI (2026-08-30).** Aviso de deuda no bloqueante en Nuevo Ingreso. Pantalla
  `src/tui/gafetes/` completa (maestro-detalle, alta individual/rango con Tab, marcar
  perdido con buscador de contratista deudor, resolver con 1=Pagado/2=Apareció, dar de
  baja). `OpcionMenu::GestionGafetes`, atajo por letra `G` — deliberadamente fuera de la
  barra de pestañas del tema Negro (mismo grupo que Cli/CerrarSesion/Salir, los
  otros accesos por letra) para no tocar el corpus de snapshots visuales de las 9 pantallas
  que sí son pestaña; único snapshot que cambió fue el propio Menú Principal (3 temas),
  regenerado y revisado. Filtro de búsqueda del catálogo simplificado a propósito (número
  exacto o `estado:`/`-estado:`) en vez del motor `clave:valor` completo — el catálogo es
  chico, no lo justifica.
- [x] **GUI (2026-08-30) — decisión explícita al retomar el plan: paridad con la TUI, no
  sólo la validación heredada.** El plan original (pre-GUI) sólo cubría la TUI; al
  retomarlo se decidió construir también la pantalla de gestión en `desktop/`, mismo
  criterio de paridad que ya tienen Contratistas/Empresas/Usuarios/Auditoría. Comandos
  Tauri (`comandos::gafetes`, sin restricción de rol, mismo criterio que el núcleo) +
  `Gafetes.tsx`/`FormularioGafete.tsx`/`GestionGafeteModal.tsx` (el buscador de deudor
  reusa el mecanismo de `ListaFlotante` que ya tenía `NuevoIngresoModal`). Aviso de deuda
  no bloqueante también en el modal de Nuevo Ingreso de la GUI.
- [x] **Corregido de paso: `AppCore::marcar_gafete_perdido`/`resolver_gafete` calculaban
  "ahora" fuera de `AppCore`.** El plan (sección 7) no detallaba este punto a nivel de
  firma; el resto de `AppCore` siempre calcula la fecha/hora con `self.reloj.ahora_utc()`
  adentro, nunca como parámetro del llamador — se corrigió para seguir esa única
  convención en vez de quedar como la excepción.

`cargo fmt`, Clippy estricto (`-D warnings`) y la suite completa en verde en los tres
proyectos (raíz: 491 tests; `desktop/src-tauri`: 20 tests; `desktop/`: `npx tsc --noEmit` +
`npm run build` + 130 tests de Vitest).

- [x] **Historial por gafete (2026-08-30).** `gafetes_incidentes` ya guardaba
  `usuario_id` de quién marcó perdido/resolvió, pero sin lector ni pantalla. Se agregó
  `GafetesIncidentesQuery::historial` (núcleo, sin paginar — un gafete tiene a lo sumo un
  puñado de incidentes) y se mostró en un modal propio (`HistorialGafeteModal.tsx`, tabla
  simple sin AG Grid), separado de `GestionGafeteModal.tsx` (que sólo maneja acciones) —
  columna "Historial" en el catálogo, botón "Detalles". Columna "Resolver" también agregada
  (visible sólo en estado Perdido, abre el mismo modal de gestión) para que el operador no
  dependa de conocer el doble click. Columna "Deudor" renombrada a "Asignado a" en los tres
  lugares donde aparecía (catálogo, modal de gestión, modal de historial) — sólo el texto
  visible, los campos internos (`contratista_deudor_*`) quedan igual.
  - [x] **Réplica en TUI (2026-08-31).** Atajo `H` en Gestión de Gafetes (Normal, sobre el
    gafete seleccionado): `ModoGafetes::Historial` reemplaza el maestro-detalle por una
    tabla de ancho completo (mismo patrón que `auditoria/render.rs`), con las mismas
    columnas que `HistorialGafeteModal.tsx` (fecha, evento, operador, asignado a, motivo).
    Sin estado "cargando" — a diferencia de lo previsto, no hizo falta cablear un tick
    async nuevo: la app es de instancia única y un solo hilo, así que
    `AppCore::historial_gafete` (ya existía, usado hasta ahora sólo por el comando Tauri)
    se llama y resuelve dentro del mismo tick que procesa `AccionGafetes::VerHistorial`,
    igual que el resto de acciones de este dispatcher. `cargo fmt`, Clippy estricto y la
    suite completa (497 tests) en verde.
- [x] **Incidentes de gafetes en Auditoría general (2026-08-30).** El historial por gafete
  (arriba) sigue sin restricción de rol; además se sumaron los mismos incidentes
  (`gafetes_incidentes`, vía `GafetesIncidentesQuery::historial_completo`, nuevo) a la
  pantalla de Auditoría general (`Auditoria.tsx`), gateados por `Operacion::VerAuditoria`
  igual que el resto — mismo patrón que contratistas/empresas: el registro completo se ve
  sin restricción, su auditoría de cambios no. `AppCore::buscar_auditoria_gafetes`
  (`application/catalogos.rs`, junto a `buscar_auditoria`) reusa `ContratistaServiceError`
  a propósito, mismo criterio que el resto de la auditoría genérica. Sin tabla nueva ni
  duplicar el dato — `gafetes_incidentes` sigue siendo la única fuente; el merge de las dos
  listas (`auditoria_cambios` + `gafetes_incidentes`) es puramente de presentación en
  `Auditoria.tsx`, ordenado por fecha.

## Sistema visual / UI

- [x] **Tema Negro y navegación por pestañas (2026-08-22).** Se agregó un tercer tema
  oscuro inspirado en la referencia entregada: fondo carbón, texto claro, acento lavanda
  y selección por video inverso. Las 9 pantallas operativas comparten una barra creada
  con `ratatui::widgets::Tabs` desde `ScreenShell`; Login, Configuración Inicial y el Menú
  Principal quedan fuera, igual que las acciones Cerrar sesión/Salir. La barra reutiliza
  `OpcionMenu::visible_para`, conserva el estado de cada pantalla al alternar y responde a
  `Ctrl+←/→` o `Ctrl+1..9`. En anchos reducidos degrada de nombres completos a nombres
  cortos y finalmente sólo números. Se regeneraron y revisaron los snapshots de los tres
  temas: 180 combinaciones en total. Animaciones, transiciones y mouse quedaron fuera del
  alcance por decisión explícita.
- [x] **Ajustado (2026-08-22): la barra se fusionó al encabezado y la navegación por
  pestañas quedó exclusiva del tema Negro.** Feedback del usuario tras probar la primera
  versión: la barra vivía en su propia fila separada del encabezado por una línea propia
  (fatiga visual, "dos piezas sueltas"), el título de pantalla repetía lo que la pestaña
  resaltada ya decía, y las pestañas no debían convivir con Classic/Brisas — para el
  usuario "tema" siempre significó todo el entorno, no sólo el color. Cambios: (1)
  `ScreenShell` (`ui_kit/shell.rs`) ahora dibuja la pestaña como 3ª línea del mismo bloque
  de encabezado, sin línea divisoria de por medio, a todo lo ancho del viewport (antes
  compartía 1/3 de columna con el reloj y degradaba a sólo números en anchos normales); el
  título de pantalla se omite cuando hay pestañas. (2) Nuevo campo
  `Theme::navegacion_pestanas` (`ui_kit/theme.rs`), `true` sólo en `ThemePreset::Negro` —
  las 9 pantallas gatean `tabs: theme.navegacion_pestanas.then_some(&tabs)` en vez de
  pasarlo siempre. (3) `App::sincronizar_vista_con_tema` (`tui/app.rs`), llamada al iniciar
  sesión y en cada F7: entrar a Negro salta del Menú Principal directo a la primera
  pestaña visible para el rol (el Menú no es alcanzable ahí); salir de Negro vuelve al
  Menú con `menu.seleccion` sincronizada a la última pantalla vista. Los atajos
  `Ctrl+←/→`/`Ctrl+1..9` y el "Volver" de Cambiar contraseña (Esc) quedaron gateados igual.
  Suite completa (289 tests, con 2 casos nuevos para la sincronización) + snapshots
  regenerados y revisados + Clippy estricto + `cargo fmt`, todo en verde.
- [x] **Bug (2026-08-21): el buscador no arrancaba vacío al activarlo.** Al presionar `/`
  en Activos, Contratistas, Empresas, Nuevo Ingreso y Usuarios, el campo se prellenaba con
  el filtro anterior (`TextInput::new(self.filtro.clone())`) en vez de arrancar limpio —
  reportado por el usuario como "entorpece" escribir una búsqueda nueva. Reparado en las 5
  pantallas (`TextInput::default()`); `self.filtro` no se toca hasta que el operador
  escribe algo, así que salir sin escribir (Enter sin cambios) conserva el filtro anterior
  intacto. Historial y Salida Rápida no tenían el bug (su caja es un campo permanente, no
  un modo que se abre/cierra).
- [x] **Reparado (2026-08-21): componentes visuales duplicados.** `render_campo` (alias
  sin valor agregado de `render_form_field`) estaba copiado en 8 `render.rs`; eliminado,
  llaman a `render_form_field` directo. Cálculo de posición del cursor duplicado en 6
  sitios → `ui_kit::{posicionar_cursor, posicionar_cursor_campo}`. Mensaje de "sin
  resultados" y de "nada seleccionado" duplicados en 7 pantallas →
  `ui_kit::{empty_state, panel_vacio}`. Clasificación `✓`/error de la barra de estado,
  idéntica en 4 pantallas → `ui_kit::clasificar_mensaje` (Historial/Respaldos tienen una
  3ª categoría real y se quedan con su propia lógica). **Sin tocar, evaluado y
  descartado por ahora:** un `ConfirmationView` único — unificarlo de verdad exigiría
  promover Activos/Empresas/Usuarios (hoy una línea en la barra de estado) al panel
  completo que ya tiene Salida Rápida, un cambio de UX visible que el usuario prefirió
  dejar para revisar después con calma, no meterlo en este barrido. La máquina de
  estados del modo búsqueda (abrir/cerrar/debounce) también sigue duplicada 5 veces —
  cada pantalla interpreta su propio `clave:valor`, unificarla exige un callback
  conectable, no es un corte mecánico.
- [x] **Reparado (2026-08-21): foco/selección/severidad dependían del color.** Auditado
  a fondo: las pantallas usan el marcador no cromático `▶` para foco y selección vía
  `ui_kit`. Gaps reales encontrados y cerrados: la tabla de Respaldos era la única sin
  ningún marcador de selección (agregado `▶`); la
  fecha PRAIND vencida/por vencer en Contratistas sólo tenía color, sin símbolo de
  respaldo (agregado `!` antes de la fecha, mismo criterio que Activos ya usa para
  accesos con advertencia).
- [x] **Reparado (2026-08-21): cobertura visual sin snapshots aprobados.** Agregada
  `insta` como dev-dependency; `visual_tests.rs` vuelca cada combinación (texto +
  tramos de estilo — color/negrita por fragmento, no sólo el texto, para no dejar pasar
  una regresión que sólo pierde una señal de color) a un snapshot aprobado por
  `insta::assert_snapshot!`. 120 snapshots generados y aprobados
  (`src/tui/snapshots/*.snap`, ~1.5 MB). El reloj de `ScreenShell`
  (`hora_actual_texto()`, hora real del sistema) se enmascara (`··:··`) antes de
  comparar — sin eso el snapshot habría cambiado solo con el reloj, sin ninguna
  regresión visual real, y el test habría empezado a fallar en cualquier corrida
  futura. Para actualizar snapshots tras un cambio visual intencional:
  `INSTA_UPDATE=always cargo test --lib visual_tests`, revisar el diff, commitear.
- [x] **Reparado (2026-08-21): atajos por letra — el único conflicto real.** La
  auditoría redujo el alcance real a un solo choque: `A` significaba "Activar/
  Desactivar" en Empresas/Usuarios pero "recargar listado" en Respaldos. Renombrado a
  `L` en Respaldos. El resto de letras señaladas (`C`, `E`, `N`, `P`, `R`) no chocan
  entre pantallas — cada una se usa en una sola pantalla con su propio significado, no
  hay una referencia global de teclado formal pero tampoco ambigüedad real que resolver.

## SQLite — decisiones abiertas

- [x] **Descartado del todo (2026-09-01): sin cifrado en reposo.** `secure_delete=FAST` ya
  reduce restos recuperables pero no cifra la base — se evaluaron BitLocker (decisión de
  despliegue, no toca código) y SQLCipher (más fuerte, pero su integración vía `rusqlite`
  es frágil en Windows: exige Perl+NASM o un `OPENSSL_DIR` externo, ver
  `docs/evaluacion-sqlite.md` sección 8). Decisión explícita del usuario: no se implementa
  ninguna de las dos, ni siquiera cifrar sólo los respaldos exportados. No queda como
  "abierto para más adelante" — si la amenaza cambia (ej. equipos que salen de las
  instalaciones sin BitLocker propio), se reevalúa desde cero con ese caso concreto
  delante, no se retoma esta nota.
- [x] **Hecho (2026-08-31): tablas `STRICT`.** `MIGRACION_15` (`SCHEMA_VERSION` 14 → 15)
  recreó las 7 tablas normales (`empresas`, `usuarios`, `contratistas`, `gafetes`,
  `gafetes_incidentes`, `registro_ingresos`, `auditoria_cambios`) con `STRICT` —
  `docs/evaluacion-sqlite.md` sección 7 sigue teniendo el detalle de qué hace `STRICT`,
  ya no dice "pendiente". Las tablas FTS5 (`*_fts` y sus tablas sombra) no admiten
  `STRICT` y quedaron igual. Hallazgo real durante la migración: `DROP TABLE` sobre una
  tabla con hijos (`empresas`/`usuarios`/`contratistas`/`gafetes`) dispara `ON DELETE
  RESTRICT` en cada uno — SQLite trata el drop como si borrara todas las filas antes de
  eliminarla. `foreign_keys` no se puede tocar dentro de una transacción activa, así
  que `MIGRACION_15` corre en su propia transacción (`aplicar_migracion_15`,
  `schema.rs`), separada de la de migraciones 1-14, con `foreign_keys=OFF` sólo
  mientras dura y un `PRAGMA foreign_key_check` antes de reactivarlo. Verificado con
  `cargo test --lib --tests` completo (incluye una prueba nueva,
  `migracion_15_deja_tablas_strict_sin_romper_claves_foraneas`, que confirma que
  `STRICT` rechaza un tipo incorrecto y que ninguna FK quedó rota) y build limpio de
  `desktop/src-tauri`.

## Respaldos — acción menor no acordada

- [ ] Eliminar un respaldo no utilizado, con confirmación (mismo patrón que Exportar). No
  se acordó para la pasada de Fases 1-4 de respaldos; sigue disponible si se quiere.

## Respaldos en la GUI (Tauri)

- [x] **Hecho (2026-09-01): pantalla de Respaldos en `desktop/`.** Paridad completa con la
  TUI (crear/listar/validar/exportar/restaurar) — `comandos/respaldos.rs` (5 comandos) +
  `pantallas/Respaldos.tsx`, sección visible sólo para Root (`rolesPermitidos: ["Root"]`,
  espejo de `Operacion::GestionarRespaldos`). Tabla simple sin AG Grid (mismo criterio que
  `HistorialGafeteModal.tsx`: la lista de respaldos es chica, no hace falta virtualización).

  Cada comando evita la trampa descrita más abajo (retener `GuiState.core` durante trabajo
  largo) sin necesitar un hilo manual — a diferencia de TUI/CLI, un comando Tauri no-async
  ya corre en el pool de hilos bloqueantes propio de Tauri, así que "no bloquear la UI" y
  "no retener el mutex compartido" son dos problemas distintos y sólo el segundo aplicaba
  acá: `crear_respaldo` autoriza con el núcleo, suelta el candado y usa
  `GuiState::conexion_secundaria()` (ya existente, de los fixes de Historial/Auditoría) para
  copiar+validar sin bloquear otros comandos; `validar_respaldo`/`exportar_respaldo`
  autorizan y sueltan el candado antes de tocar el archivo (ninguno de los dos pasa por
  `self.connection` en absoluto del lado del núcleo).

  `restaurar_respaldo` es la excepción deliberada: retiene el candado de principio a fin,
  porque acá SÍ es correcto bloquear todo — nada más debería tocar la base mientras se
  reemplaza el archivo. El intercambio (mismo flujo que la TUI en
  `tui/app/actions/admin.rs`/`main.rs`, adaptado a que acá `AppCore` vive en un `Mutex` para
  toda la vida del proceso, sin poder reiniciarlo): 1) crea un respaldo `PreRestauracion`
  con la conexión todavía viva; 2) la cierra (`AppCore::cerrar`, nuevo — consume `self` y
  devuelve la ruta) reemplazándola por un `AppCore` en memoria mientras dura el intercambio
  de archivos, exigido por el contrato de `database::backup::restaurar_respaldo`; 3)
  reemplaza el archivo; 4) abre un `AppCore` nuevo. Cierra la sesión al terminar (éxito o
  error) — la base cambió de identidad, igual que la TUI fuerza login nuevo.

  `TipoRespaldo`/`RespaldoResumen`/`ResultadoValidacion` (`database/backup.rs`) ganaron
  `#[cfg_attr(feature = "serde", derive(Serialize))]` para cruzar el IPC — mismo criterio ya
  usado en el resto del núcleo (`CargaCompleta`, `CambioAuditado`), sin afectar TUI/CLI (el
  feature no está activo ahí). `desktop/src-tauri` ya tenía `rusqlite` como dependencia
  directa desde los fixes de Historial/Auditoría.

  Verificado del lado del frontend con el toolchain completo disponible esta vez
  (`npm install` sí funcionó en este entorno): `tsc --noEmit`, `vitest run` (140 tests) y
  `vite build` los tres limpios. Del lado de Rust, `cargo fmt`/Clippy estricto/502 tests
  del crate raíz en verde con y sin el feature `serde`; `desktop/src-tauri` en sí sigue sin
  poder compilarse en este entorno (faltan las libs de GTK, mismo límite de siempre) —
  revisado a mano con el máximo cuidado posible, pendiente de un `cargo check` real.

- **Trampa real a no repetir** (documentada antes de resolver lo de arriba, se deja como
  referencia): `GuiState.core` es un `Mutex<AppCore>` compartido por *todos* los comandos
  (`desktop/src-tauri/src/estado.rs`). Si el comando de respaldo mantiene ese lock durante
  los ~200ms–2s que tarda copiar+validar (medido, ver más abajo "Respaldo automático"),
  **cualquier otro comando de la GUI** (buscar contratistas, registrar ingreso, lo que sea)
  se queda esperando el mismo lock — la ventana no se congela visualmente, pero la app
  entera deja de responder igual que antes se congelaba la TUI. La solución es la misma
  idea que ya se usó en `tui/app/backup_jobs.rs`: abrir una conexión de lectura aparte al
  mismo archivo (`Connection::open(core.ruta_base_datos())` + `busy_timeout`) y soltar el
  lock de `GuiState` apenas se lee la ruta y se autoriza, antes de copiar/validar.

## Módulo de actualizaciones — ya construido (esta nota estaba desactualizada)

Esta sección decía "no iniciado, condicionado" — quedó obsoleta: el actualizador ya está
construido, firmado y funcionando en CI (`desktop/docs/pendientes.md`, sección "Empaquetado
y actualizaciones", tiene el detalle completo — no se duplica acá). Repaso de los 5 puntos
que esta nota dejaba como condición, contra lo que de verdad se construyó:

- [x] **El fallo de red nunca impide iniciar la app.** `buscarActualizacion()` corre en un
  `useEffect` de `Shell` (`App.tsx`) que ya arrancó con la sesión autenticada — un `catch`
  sólo hace `console.error`, nunca bloquea el render. Confirmado leyendo el código.
- [x] **Descargar y verificar firma antes de instalar.** `instalarActualizacion()`
  (`api/actualizaciones.ts`) delega en `Update.downloadAndInstall()` de
  `tauri-plugin-updater`, que verifica contra `plugins.updater.pubkey`
  (`tauri.conf.json`) antes de instalar — si no coincide, la promesa rechaza sin tocar
  nada. Llave real cargada y confirmada firmando bien en CI (`desktop/docs/pendientes.md`).
- [ ] **Impedir la actualización mientras la app tenga el bloqueo de instancia — sin
  verificar de verdad.** A diferencia de los otros cuatro puntos, no encontré evidencia de
  que esto se haya probado explícitamente: `relaunch()` reinicia el mismo proceso (no hay
  un instalador externo separado corriendo mientras `InstanciaGuard` sigue en pie), así que
  el riesgo original probablemente no aplica tal cual se pensó, pero eso es una suposición
  mía, no algo confirmado. Si se quiere cerrar del todo, valdría instalar una actualización
  real en una máquina con otra ventana de la app abierta y confirmar qué pasa.
- [x] **Respaldar antes de migrar, con rollback.** Ya existe independientemente del
  actualizador — `respaldar_antes_de_migrar` (`src/database/connection.rs`) crea un
  respaldo `TipoRespaldo::PreMigracion` antes de que `initialize_database` aplique
  cualquier migración pendiente, obligatorio (si el respaldo falla, la migración no corre).
  Cubre una actualización nueva igual que cualquier apertura con esquema pendiente.
- [x] **Cliente de actualización separado del núcleo.** `api/actualizaciones.ts` sólo usa
  `@tauri-apps/plugin-updater`/`@tauri-apps/plugin-process` — ningún comando en
  `desktop/src-tauri/src/comandos/` ni tipo de `control_acceso` está involucrado.

4 de 5 confirmados; el de instancia queda anotado como el único punto realmente sin
verificar, no descartado ni dado por hecho.

## Permisos granulares por usuario — descartado por ahora

- [x] **Evaluado (2026-08-22): no se construye.** Hoy los permisos son RBAC simple
  (`rol.puede(Operacion::X)`, ej. `application/catalogos.rs`) — agregar que un rol pueda
  hacer algo nuevo es un match arm, no una migración. Un sistema de permisos por usuario
  individual (matriz configurable, pantalla de asignación) resolvería un problema
  hipotético, no uno real. Retomar sólo si aparece un caso concreto donde los 3 roles
  actuales ya no alcanzan (ej. "este operador puede X pero no Y") — ahí se evalúa una vez
  con el caso real delante, no antes.

## Percepción de velocidad — sugerencias UX (sin acordar, evaluar con calma)

Origen: el usuario reportó una sensación de lentitud en la app (no en el buscador ni en
animaciones puntuales — "no sé si es la app o soy yo"). Auditoría de
`src/tui/app.rs`/`terminal.rs`/`ui_kit/debounce.rs`: no hay nada objetivamente lento —
redibujo sólo por evento (no hay loop de 60fps quemando CPU), `event::poll` cada 50ms,
debounce de búsqueda en 120ms (`activos`/`empresas`/`nuevo_ingreso`/`usuarios`/`historial`/
`contratistas`/`salida_rapida`, todos `state.rs`), Argon2 en hilos aparte
(`app/auth_jobs.rs`) sin bloquear el loop. Conclusión: es percepción, no rendimiento real
— Ratatui redibuja por *snapshot* (la pantalla entera cambia de golpe, sin transición),
mientras que una CLI tipo Ink/React va mostrando actividad progresiva (cursor, streaming,
spinners), lo que se lee como "viva" aunque no sea más rápida. Ideas para achicar esa
brecha perceptual, en orden de costo/beneficio:

- [ ] **Spinner o indicador durante el debounce de búsqueda (120ms).** Hoy no hay ninguna
  señal entre que se deja de teclear y que aparece el resultado filtrado — se siente como
  un salto. Un indicador chico (p. ej. `⏳` o `…` en la etiqueta de búsqueda) mientras
  `Debounce::listo` todavía no disparó daría la sensación de "está procesando" en vez de
  "no reaccionó".
- [ ] **Parpadeo de cursor en campos de texto ya existe en Login** (`DURACION_PARPADEO`,
  `login/state.rs`) **pero no en los demás formularios** (Contratistas, Usuarios, Nuevo
  Ingreso, etc.). Extender el mismo patrón a `ui_kit/text_input.rs` daría consistencia y
  una señal continua de "esto está vivo" incluso sin tecleo.
- [ ] **Confirmación visual breve tras guardar/registrar** (p. ej. resaltar la fila
  recién creada/editada por un instante) en vez de sólo el mensaje de estado en texto —
  hoy el cambio es instantáneo y silencioso, lo que puede leerse como "¿pasó algo?".
- [x] **Medido y reparado (2026-08-31): crear respaldo sí bloqueaba, y de verdad.**
  Medición real (no sólo lectura de código) con datos de volumen creciente: copiar +
  validar la base completa (Online Backup API + `integrity_check` + `foreign_key_check`)
  tarda ~200ms con unos pocos miles de movimientos y **~2 segundos con ~100,000** — ya
  perceptible hoy, y empeora con la antigüedad de la instalación. Antes corría
  síncrono en el mismo hilo que dibuja la pantalla, tanto para el botón manual
  (Respaldos → Crear) como para la revisión automática diaria (`run_internal`, cada
  60s). El resto de la app se midió también y está lejos de ser un problema: cada
  escritura normal (Nuevo Ingreso, contratistas, gafetes) tarda ~1.3ms pese al perfil
  de durabilidad estricto (`journal_mode=DELETE` + `synchronous=EXTRA`), y las
  búsquedas/Historial son sub-milisegundo incluso con 10,000 filas — los índices están
  bien puestos.

  Reparado con el mismo patrón que `auth_jobs.rs` (hilo + `mpsc::Receiver` sondeado en
  el bucle) en un módulo nuevo, `tui/app/backup_jobs.rs`: el hilo abre su PROPIA
  conexión de sólo lectura al mismo archivo (`Connection::open` + `busy_timeout`) en
  vez de compartir la conexión viva de `AppCore` entre hilos — SQLite admite varias
  conexiones concurrentes al mismo archivo, y `Backup::run_to_completion` ya reintenta
  solo ante un bloqueo transitorio. `AppCore` ganó `autorizar_creacion_respaldo`
  (autorización sola, rápida, en el hilo principal), `ruta_base_datos()`,
  `directorio_respaldos()` (ahora público) y `hace_falta_respaldo_automatico_hoy`
  (la mitad "decidir" de `respaldo_automatico_diario_si_hace_falta`, que se conserva
  intacta para los llamadores previos al bucle de la TUI en `main.rs`, donde un
  respaldo síncrono no compite con nadie mirando la pantalla). Respaldos → Crear
  muestra "⠋ Creando respaldo…" (`RespaldosState::creando`, mismo patrón que
  `UsuariosState::guardando`) y bloquea disparar uno segundo mientras el primero sigue
  en vuelo. El respaldo previo a una restauración (`AccionRespaldos::Restaurar`) se
  dejó síncrono a propósito: la app sale inmediatamente después
  (`SalidaApp::Restaurar`), así que un freno breve ahí importa mucho menos que uno en
  medio de una sesión activa. `cargo fmt`, Clippy estricto y la suite completa (499
  tests) en verde, con pruebas nuevas de la creación real en un hilo aparte
  (`tui/app/tests.rs`) y del límite de decisión "hace falta hoy" (`tests/configuracion_respaldos.rs`).
- [x] **Medido y reparado (2026-08-31): exportar Historial a XLSX (F5) era el punto
  realmente bloqueante, peor que el respaldo.** Al verificar si el respaldo era el
  único proceso que necesitaba su propio hilo, se midió también la exportación:
  armar el XLSX de 100,000 movimientos tarda **~33 segundos** — muy por encima de los
  ~2 segundos del respaldo, y corría igual de síncrona en el hilo que dibuja la
  pantalla. Peor todavía: el aviso "Exportando historial…" que ya existía en el
  código nunca llegaba a pintarse — se fijaba en el mismo tick que arrancaba la
  exportación, pero el `terminal.draw()` que lo hubiera mostrado corre en la vuelta
  *siguiente* del bucle, que nunca llegaba hasta que la exportación (síncrona)
  terminaba. El operador se quedaba con la pantalla congelada sin ninguna señal.

  Reparado con el mismo patrón que `backup_jobs.rs`: módulo nuevo
  `tui/app/historial_jobs.rs` (hilo + `mpsc::Receiver`, conexión de sólo lectura
  propia). El núcleo de `AppCore::{buscar_historial, movimientos_en_orden,
  exportar_historial_seleccion}` se extrajo a funciones libres que reciben
  `&Connection` en vez de `&self` (`buscar_historial_con_conexion`,
  `movimientos_en_orden_con_conexion`, `exportar_historial_seleccion_con_conexion`,
  `application/historial.rs`, reexportada la última desde `application::mod`) —
  mismo motivo que separar `crear_respaldo` de `AppCore`: el hilo necesita operar
  sobre una conexión que no es la de `AppCore` sin duplicar la lógica de consulta.
  Los métodos de `AppCore` quedaron como delegadores finos sobre `&self.connection`,
  API pública sin cambios (la GUI/Tauri, que sí usa estos métodos directo, no se vio
  afectada). Historial muestra "⠋ Exportando historial…"
  (`HistorialState::exportando`, mismo patrón que `RespaldosState::creando`) y F5
  no encola una segunda exportación mientras la primera sigue en vuelo. `cargo fmt`,
  Clippy estricto y la suite completa (501 tests) en verde, con pruebas nuevas del
  guard de F5 (`tui/historial/tests.rs`) y de la exportación real en un hilo aparte
  con el archivo terminando en disco (`tui/app/tests.rs`).
- [x] **Frame de transición mínimo en cambios de vista: descartado (2026-08-22).** La
  navegación por pestañas se pidió sin animaciones ni transiciones; se conserva el cambio
  inmediato y no se agrega trabajo visual ajeno al alcance.
- [x] **Auditoría de cuellos de botella en las tres capas (2026-09-01): mismo bug de
  exportar Historial replicado en CLI, más dos hallazgos propios de Tauri.** Con el fix
  de la TUI ya en pie, se auditaron también CLI (`src/cli/`) y la GUI de escritorio
  (`desktop/src-tauri/`) buscando el mismo patrón (trabajo bloqueante retenido en un
  lugar compartido). Tres hallazgos, los tres reparados:

  1. **CLI: exportar Historial (F5) tenía el mismo freeze que la TUI antes del fix.**
     El bucle de eventos del CLI es igual de single-thread que el de la TUI, y
     `confirmar_exportacion` llamaba `exportar_historial_seleccion` de forma síncrona.
     Reparado reusando las mismas funciones libres `_con_conexion` extraídas para la
     TUI: `src/cli/historial_controller.rs` ahora lanza un hilo con conexión propia y
     un `mpsc::Receiver` sondeado en `run()` (`recibir_exportacion_si_lista`, agregado
     también a `proxima_espera` para no dormir hasta 1h sin notar que terminó). La
     Surface de Historial muestra "⠋ Exportando…" y bloquea el input mientras dura
     (`HistorialState::exportando`). Prueba nueva de la exportación real en un hilo
     aparte con el archivo terminando en disco.
  2. **Tauri: `exportar_historial` retenía el `Mutex<AppCore>` compartido durante toda
     la exportación.** A diferencia de la TUI/CLI, un comando Tauri no-async ya corre
     en el pool de hilos bloqueantes propio de Tauri (no congela la ventana) — pero
     retener el núcleo compartido sí bloqueaba cualquier OTRO comando (Activos,
     Contratistas, etc.) mientras duraban esos ~33s con un historial grande. No hizo
     falta un hilo propio: alcanzó con no abrir el mutex durante la consulta.
     `GuiState` ganó `conexion_secundaria()` (`desktop/src-tauri/src/estado.rs`), que
     sólo toma el candado para leer `ruta_base_datos()` y abre una `Connection`
     independiente (mismo patrón `busy_timeout` que TUI/CLI); `exportar_historial`
     ahora la usa con `exportar_historial_seleccion_con_conexion` en vez de
     `state.core().exportar_historial_seleccion(...)`.
  3. **Tauri: `listar_historial`/`listar_auditoria` retenían el mismo mutex ~750ms al
     tope de carga completa (20,000 filas, `LIMITE_CARGA_COMPLETA_MAXIMO`).** Menos
     grave que el punto 2, pero mismo problema de fondo. Se extrajeron
     `buscar_historial_completo_con_conexion` (`src/application/historial.rs`) y
     `buscar_auditoria_con_conexion`/`buscar_auditoria_completo_con_conexion`
     (`src/application/catalogos.rs`), reexportadas desde `application::mod`; los
     métodos de `AppCore` quedaron como delegadores finos, igual criterio que ya
     se usó para historial. Ambos comandos Tauri pasaron a usar
     `GuiState::conexion_secundaria()` en vez de `state.core()`.

  `desktop/src-tauri` sumó `rusqlite` como dependencia directa (misma versión que fija
  el crate raíz, Cargo unifica una sola copia) para poder abrir esa segunda conexión.
  No se pudo compilar `desktop/src-tauri` en el entorno de esta sesión (siguen
  faltando las librerías de desarrollo de GTK, mismo límite ya documentado al renombrar
  `comandos/` → `cli/`) — los cambios se verificaron con la máxima revisión manual
  posible y comparando contra el patrón ya probado de TUI/CLI, pero quedan pendientes
  de una compilación real (`cargo check`) en un entorno con el toolchain completo antes
  de darlos por definitivos. El núcleo (`cargo fmt`, Clippy estricto, 502 tests) sí está
  verde.

Nada de esto es un bug ni tiene prioridad definida — quedan acá como banco de ideas para
retomar cuando se decida invertir tiempo en pulido visual, no porque haya un problema de
rendimiento real que resolver.

## Empaquetado — ideas de lujo, no para v1

- [ ] **Instalador único que incluya la CLI (`control_acceso.exe`) además del bundle de
  Tauri.** Es técnicamente posible: el bundler de Tauri (NSIS/WiX en Windows) admite
  copiar binarios extra al directorio de instalación vía `bundle.resources` en
  `tauri.conf.json`, y un template NSIS custom podría agregarle su propio acceso directo
  de Start Menu. Explícitamente catalogado por el usuario como "lujo", no una necesidad —
  se descartó para v1: hoy los dos binarios ya se generan en el mismo `release.yml`
  (jobs `build` y `build-gui`, ver más abajo), sólo que en destinos separados (artifact de
  Actions vs. GitHub Release). Fusionarlos en un solo instalador suma un template NSIS a
  mantener y no cambia nada para quien sólo usa la GUI — se retoma si en algún momento hay
  una razón concreta (ej. onboarding de alguien que necesita ambas interfaces a la vez).

- [ ] **CLI: detectar Alacritty instalado y usarlo como terminal por defecto, con una
  preferencia en config para elegir cuál usar.** Anotado a pedido del usuario, sin
  acordar todavía — opinión técnica para cuando se retome:
  - La CLI hoy no elige su terminal: `control_acceso.exe` simplemente renderiza con
    Ratatui dentro del proceso que ya lo lanzó (conhost, PowerShell, Windows Terminal,
    lo que sea). "Usar Alacritty automáticamente" implicaría que el binario se
    re-ejecute a sí mismo envuelto en `alacritty -e control_acceso.exe` y cierre la
    ventana original — no es una config que se lee, es reemplazar el proceso en
    marcha.
  - Eso trae riesgos que no tiene la recomendación pasiva que ya está en el README
    ([línea 142](../README.md#L142)): parpadeo de ventana (se abre una, se cierra,
    abre otra), y sobre todo **una carrera con `InstanciaGuard`** (el candado de
    instancia única que ya usa tanto la CLI como la GUI) — si el proceso padre suelta
    el candado antes de que el hijo lo tome, hay una ventana donde en teoría podría
    colarse otra instancia.
  - Además, si alguien ya abrió la CLI a propósito desde su terminal de siempre
    (PowerShell, Windows Terminal), reemplazarle la ventana por Alacritty sin que lo
    pida es sorpresivo — va contra elegir la terminal, que es justamente lo que se
    quiere respetar con la preferencia de config.
  - **Alternativa más simple que da el mismo resultado práctico:** un acceso directo
    opcional ("Control de Acceso (Alacritty)") que invoque
    `alacritty -e control_acceso.exe`, generado sólo si el instalador detecta
    Alacritty presente. Cero código nuevo en el binario, cero relanzamiento, cero
    carrera con el candado — es packaging, no lógica de la app. La preferencia de
    config seguiría teniendo sentido aparte (qué terminal *recuerda* la app la
    próxima vez que haya que decirle algo al usuario, no cuál usar para lanzarla).
  - Sin decidir todavía si vale la pena ninguna de las dos — es la línea de base para
    cuando se retome.

## Roadmap de producto (V2/V3, fuera del alcance actual)

- V2: visitas/proveedores y "a quién viene a ver"; notificación proactiva de PRAIND
  descartada.
- V3: concurrencia multi-terminal — dispara revisar los agregados con campos públicos de
  arriba, y columna de versión + actualizaciones optimistas para ediciones concurrentes de
  contratistas (hoy imposible con instancia única + loop síncrono, así que no aplica).
