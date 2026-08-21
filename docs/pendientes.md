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

## Respaldo automático — fallos silenciosos (a discutir a continuación)

- [ ] `respaldo_automatico_diario_si_hace_falta` (`src/application.rs`) descarta todos sus
  errores en silencio, incluida la limpieza por retención — si falla una vez, queda roto
  para siempre sin ningún aviso y los respaldos se acumulan sin límite. Es **por diseño**
  (decisión previa: "ignorar en silencio si falla, no es obligatorio"), pero queda abierto
  para que el usuario decida si quiere reconsiderarlo.

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

## Sistema visual / UI

- [x] **Bug (2026-08-21): el buscador no arrancaba vacío al activarlo.** Al presionar `/`
  en Activos, Contratistas, Empresas, Nuevo Ingreso y Usuarios, el campo se prellenaba con
  el filtro anterior (`TextInput::new(self.filtro.clone())`) en vez de arrancar limpio —
  reportado por el usuario como "entorpece" escribir una búsqueda nueva. Reparado en las 5
  pantallas (`TextInput::default()`); `self.filtro` no se toca hasta que el operador
  escribe algo, así que salir sin escribir (Enter sin cambios) conserva el filtro anterior
  intacto. Historial y Salida Rápida no tenían el bug (su caja es un campo permanente, no
  un modo que se abre/cierra).
- [x] **Parcial (2026-08-21): componentes visuales duplicados — buscador.** `render_campo`
  (alias sin valor agregado de `render_form_field`) estaba copiado en 8 `render.rs`; se
  eliminó y esos 8 llaman a `render_form_field` de `ui_kit` directo. El cálculo de
  posición del cursor (`antes_del_cursor`/`ancho_visible`/`set_cursor_position`) estaba
  duplicado igual en 6 sitios — se movió a `ui_kit::{posicionar_cursor,
  posicionar_cursor_campo}`. `FormField`/`ChoiceField`/separadores/layout maestro-detalle
  ya estaban en `ui_kit` de antes; faltan estados vacíos (`EmptyState`), confirmaciones
  (`ConfirmationView`) y mensajes de estado (`StatusMessage`) como componentes
  compartidos — hoy cada pantalla los reimplementa. La máquina de estados del modo
  búsqueda (abrir/cerrar/debounce) sigue duplicada 5 veces — no se tocó porque cada
  pantalla interpreta su propio `clave:valor`; unificarla exigiría un callback
  conectable, no es un corte mecánico.
- [ ] **Foco, selección y severidad dependen demasiado del color.** Reforzar con `>`,
  `[x]`, `!` o etiqueta textual de forma consistente (hoy no todas las vistas lo hacen
  igual); verificar ambos temas con contraste reducido y paleta de 16 colores.
- [ ] **Cobertura visual automatizada sin snapshots aprobados.** Ya existe una matriz que
  renderiza 12 pantallas en 5 tamaños × 2 temas (120 combinaciones, `visual_tests.rs`),
  pero sin snapshots de referencia guardados — falta comparar contra un estado aprobado,
  no sólo que no truene.
- [ ] **Paneles de confirmación sin componente unificado.** Cada pantalla arma su propia
  confirmación; falta un componente común que incluya siempre acción, identidad del
  objeto, datos para evitar confusión, consecuencia y atajos de confirmar/cancelar, con
  estilo especial para lo destructivo/irreversible.
- [ ] **Atajos por letra inconsistentes entre pantallas.** `A`, `C`, `E`, `N`, `P`, `R`
  cambian de significado según la pantalla. Aceptable localmente, pero sin una referencia
  global de teclado que lo explique de un vistazo.

## SQLite — decisiones abiertas

- [ ] **Política de cifrado en reposo.** `secure_delete=FAST` ya reduce restos
  recuperables pero no cifra la base. Si se decide que hace falta proteger contra robo
  del equipo o copia del archivo: BitLocker es una decisión de despliegue (no toca
  código); SQLCipher es más fuerte pero su integración vía `rusqlite` es frágil en
  Windows (exige Perl+NASM o un `OPENSSL_DIR` externo) — ver
  `docs/evaluacion-sqlite.md` sección 8 para el detalle. Cifrar sólo los respaldos
  exportados es la alternativa más liviana si la amenaza real es "un respaldo en un
  medio sin cifrar". Requiere decisión de política antes de tocar código, no es un bug.
- [ ] **Evaluar tablas `STRICT`.** Refuerzan tipos almacenados; no urgente porque el
  modelo Rust y las restricciones actuales ya cubren la mayoría del riesgo. Convertir
  las tablas existentes exige una migración completa — evaluar junto con la próxima
  migración real, no aparte.

## Respaldos — acción menor no acordada

- [ ] Eliminar un respaldo no utilizado, con confirmación (mismo patrón que Exportar). No
  se acordó para la pasada de Fases 1-4 de respaldos; sigue disponible si se quiere.

## Módulo futuro de actualizaciones (no iniciado, condicionado)

Sólo aplica si se decide construir un actualizador. Nada de esto es una carencia del
sistema actual:

- [ ] El fallo de red o del servidor de actualizaciones nunca impide iniciar la app.
- [ ] Descargar a un archivo temporal y verificar firma criptográfica antes de instalar.
- [ ] Impedir la actualización mientras la aplicación tenga el bloqueo de instancia.
- [ ] Respaldar SQLite antes de migrar y conservar un mecanismo probado de rollback.
- [ ] Separar completamente el cliente de actualización del núcleo de control de acceso.

## Roadmap de producto (V2/V3, fuera del alcance actual)

- V2: visitas/proveedores y "a quién viene a ver"; notificación proactiva de PRAIND
  descartada.
- V3: concurrencia multi-terminal — dispara revisar los agregados con campos públicos de
  arriba, y columna de versión + actualizaciones optimistas para ediciones concurrentes de
  contratistas (hoy imposible con instancia única + loop síncrono, así que no aplica).
