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
- [ ] **Fase 4 — Agrupar despachadores por área.** Separar en
  `src/tui/app/actions/{accesos,catalogos,admin}.rs` en vez de un archivo monolítico o uno
  por pantalla.
- [ ] **Fase 5 — Separar navegación y runtime.** `src/tui/app/navigation.rs` y
  `src/tui/app/runtime.rs`; evaluar reemplazar `Option<&AppCore>` por un modo de ejecución
  explícito (normal vs. arranque degradado).

Riesgos a conservar bajo prueba durante el refactor: una vista autenticada no debe quedar
activa sin sesión; F2 debe refrescar Activos/Historial/Nuevo Ingreso según la vista debajo
del overlay; los hilos de Argon2 no deben bloquear el loop ni perder una escritura
validada al cerrar; la restauración sólo devuelve `SalidaApp::Restaurar`; el modo sin
`AppCore` no debe dejar formularios esperando indefinidamente. Cada corte: `cargo fmt` +
suite completa + Clippy estricto.

## Siguiente candidato tras `app.rs`: repartir `AppCore`

- [ ] `src/application.rs` (965 líneas) concentra arranque, autenticación, contratistas,
  empresas, ingresos/historial, usuarios, respaldos y exportación XLSX. Repartir
  mecánicamente sus bloques `impl` sin cambiar la API pública:
  ```text
  src/application/mod.rs          (estructura, construcción, errores compartidos)
  src/application/autenticacion.rs
  src/application/accesos.rs
  src/application/catalogos.rs
  src/application/usuarios.rs
  src/application/respaldos.rs
  src/application/historial.rs
  ```

## Otros archivos grandes (orden sugerido, después de lo anterior)

- [ ] `src/database/queries/ingresos.rs` (807 líneas) — separar en
  `ingresos/{mod,activos,historial}.rs`; mantener los conversores compartidos en `mod.rs`.
- [ ] `src/tui/contratistas/state.rs` (934 líneas) — extraer `contratistas/query.rs` y
  `contratistas/form.rs`; dejar estado público y `handle_key` en `state.rs`.
- [ ] `src/tui/usuarios/state.rs` (870 líneas) — extraer `usuarios/form.rs` y
  `usuarios/password.rs`.
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

- [ ] **Historial no avisa de clave no reconocida.** Contratistas y Activos ya usan
  `resolver_terminos_detallado` y muestran `no_reconocidos`
  (`contratistas/state.rs:429-475`, `activos/state.rs:279-321`); Historial sigue en
  `resolver_terminos` simple (`historial/filtros.rs:143`), así que un typo en la clave cae
  a texto libre sin aviso, casi siempre 0 resultados indistinguibles de una búsqueda
  legítima sin coincidencias.
- [ ] **Historial oculta la negación de `tipo`/`estado` en la etiqueta de búsqueda.**
  `-tipo:swat` se guarda como el complemento positivo internamente, así que el resumen
  (`historial/render.rs:140-148`) muestra `"tipo: PRAIND o IN HOUSE..."` sin rastro de que
  hubo negación — a diferencia de `ingreso`/`salida`, un poco más abajo en el mismo
  archivo, que sí muestran el signo `≠`.
- [ ] **F1 no documenta sintaxis de valores, sólo nombres de clave.** Ninguna de las 3
  pantallas (`contratistas/render.rs:103`, `activos/render.rs:88`, `historial/render.rs:54`)
  menciona en `ayuda_extra` los valores aceptados (`praind:vence|vencido|sin`,
  `ruta:si|no`, `medio:caminando|vehiculo`, `estado:activos|cerrados|todos`) ni la
  negación con guion.
- [ ] **`ingreso:`/`salida:` en Historial no pliegan tildes**, a diferencia de `empresa:`.
  `historial/filtros.rs:210-217` pasa el valor crudo a `LIKE ... COLLATE NOCASE`, que sólo
  pliega ASCII. Ej.: "María José" — `salida:jose` matchea, `salida:josé` no.

## Sistema visual / UI

- [ ] **Componentes visuales aún duplicados entre pantallas.** `FormField`/`ChoiceField`/
  separadores/layout maestro-detalle ya se centralizaron en `ui_kit`; faltan estados
  vacíos (`EmptyState`), confirmaciones (`ConfirmationView`) y mensajes de estado
  (`StatusMessage`) como componentes compartidos — hoy cada pantalla los reimplementa.
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
