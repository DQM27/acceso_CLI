# Plan de saneamiento técnico

Este plan parte de las siguientes decisiones:

- La aplicación funciona sin conexión a internet.
- SQLite es local al equipo; no se comparte el archivo por red.
- Sólo puede existir una instancia de la aplicación por base de datos.
- El futuro actualizador será opcional y no impedirá trabajar sin internet.

Cada tarea sólo se considera terminada cuando tiene pruebas y manejo de errores visible.

## Prioridad 0: integridad y continuidad operativa

- [x] **Instancia única por base de datos.** Adquirir un bloqueo del sistema antes de
  abrir SQLite. Una segunda ejecución debe terminar con un mensaje claro y nunca debe
  alcanzar las migraciones. El bloqueo debe liberarse automáticamente incluso si el
  proceso termina de forma inesperada.
- [x] **Ruta estable para SQLite.** Sustituir la ruta relativa al directorio de trabajo
  por una ubicación absoluta de datos de la aplicación. Mantener
  `CONTROL_ACCESO_DB` como sobreescritura explícita y no crear una base productiva
  silenciosamente en una ubicación inesperada.
- [x] **Respaldo y recuperación.** Ver el [plan específico de respaldos](plan-respaldos.md)
  (Fases 1-4 completas): motor de creación/validación con la API de SQLite, restauración
  con rollback automático, pantalla Configuración → Respaldos en la TUI (Crear, Listar,
  Revalidar, Exportar, Restaurar), respaldo obligatorio antes de migrar el esquema, y
  respaldo automático diario con retención. Pendiente sólo la acción Eliminar (menor, no
  bloqueante).
- [x] **Migraciones globalmente atómicas.** Tomar `BEGIN IMMEDIATE` antes de leer
  `user_version` y ejecutar dentro de la misma transacción todas las migraciones
  pendientes. Probar dos aperturas simultáneas.
- [x] **Entrada atómica.** Leer al contratista, validar acceso/PRAIND, comprobar ingreso
  y gafete e insertar el movimiento dentro de una sola transacción. Traducir colisiones
  de índices a errores de negocio comprensibles.
- [x] **Ingresos activos completos.** Eliminar el límite silencioso de 100, devolver el
  listado completo y hacer que la búsqueda por gafete devuelva directamente el registro
  encontrado. La vista operativa debe mostrar el total real de personas dentro.

## Prioridad 1: auditoría y seguridad de operación

- [x] **Cédula inmutable del contratista.** Tratarla como identidad: no admitirla en
  actualizaciones normales, mostrarla como sólo lectura y bloquear cambios directos
  mediante SQLite. Los demás datos editables sólo afectan ingresos futuros.
- [x] **Historial inmutable.** Guardar snapshots de cédula/nombre del contratista,
  empresa y operadores, además del resultado de acceso, motivo, PRAIND evaluado y
  versión de reglas. Los cambios posteriores no alteran movimientos históricos. Los
  datos anteriores a esta mejora quedan identificados como reconstruidos durante la
  migración, porque no es posible recuperar valores que ya fueron sobrescritos.
- [x] ~~**Eventos denegados.**~~ Descartado: el usuario determinó que este nivel de
  auditoría no aplica para este tipo de app.
- [x] ~~**Sesión vigente.**~~ Descartado, mismo motivo.
- [x] **Política horaria explícita.** Centralizar un reloj, usar la zona
  `America/Costa_Rica`, persistir UTC/offset y detectar retrocesos del reloj respecto al
  último movimiento.
- [x] ~~**Errores observables** (log técnico persistente).~~ Descartado, mismo motivo. El
  tratamiento de `SQLITE_BUSY` como error observable para el operador (mencionado también
  en Prioridad 2) queda descartado junto con este punto.
- [x] **Lecturas paginadas coherentes.** Obtener el total y la página del historial en
  la misma lectura de SQLite y navegar con un cursor estable, para no repetir u omitir
  movimientos si los datos cambian entre páginas.

## Prioridad 2: ejecución y mantenibilidad

- [x] **TUI responsiva (login, búsquedas, alta/edición de usuarios).** Se eliminó la espera
  artificial de 800ms del login. Argon2 corre en un hilo aparte en los cuatro lugares donde
  se calcula un hash: login, crear usuario, cambiar contraseña y ROOT inicial — nunca
  bloquea `terminal.draw`. En los tres últimos, `UsuarioService` quedó partido en
  `validar_...`/`..._con_hash` (lo rápido y ligado a SQLite se queda en el hilo principal;
  sólo el hash puro se calcula aparte), con `UsuariosState`/`ConfiguracionInicialState`
  bloqueando la edición mientras se espera el resultado real. Las 5 pantallas de búsqueda
  (Historial, Contratistas, Activos, Empresas, Usuarios) tienen debounce de 250ms vía
  `ui_kit::Debounce`, en vez de una consulta SQL por tecla.
- [x] ~~**Modo demo aislado.**~~ Descartado: el usuario determinó que esta deuda de
  mantenibilidad no aplica para producción, sólo agrega complejidad.
- [x] ~~**Estados sin `unwrap`.**~~ Descartado, mismo motivo.
- [x] ~~**Normalización de identidades.**~~ Descartado, mismo motivo.
- [x] **Configuración SQLite explícita (perfil base).** `busy_timeout`, `journal_mode`,
  `synchronous`, `application_id`, `trusted_schema` y `quick_check`/`optimize` ya están
  implementados y probados — ver la
  [evaluación y recomendaciones de SQLite](evaluacion-sqlite.md), secciones 2-6. El
  tratamiento explícito de `SQLITE_BUSY` como error observable queda descartado junto con
  "Errores observables" (Prioridad 1).
- [x] ~~**Capas sin dependencia de SQLite.**~~ Descartado, mismo motivo que arriba.
- [x] ~~**Pruebas operativas** (disco lleno, base bloqueada, reloj incorrecto, cierre
  inesperado, prueba de pérdida de energía en un entorno real).~~ Descartado, mismo
  motivo.
- [x] ~~**Verificación automatizada** (CI con `cargo test`/`fmt`/Clippy).~~ Descartado,
  mismo motivo.

## Capacidades futuras condicionadas

Estos puntos no son fallos del modelo actual. Sólo deben implementarse si cambia la
arquitectura indicada:

- **Ediciones concurrentes de contratistas.** Si en el futuro se permiten varias
  terminales, conexiones escritoras o procesos en segundo plano, añadir una columna de
  versión y actualizaciones optimistas para impedir que un formulario antiguo
  sobrescriba un cambio reciente. Con la instancia única, una sola vista activa y el
  bucle síncrono actual, este escenario no puede producirse mediante la aplicación.

## Módulo futuro de actualizaciones

- [ ] El fallo de red o del servidor de actualizaciones nunca impide iniciar la app.
- [ ] Descargar a un archivo temporal y verificar firma criptográfica antes de instalar.
- [ ] Impedir la actualización mientras la aplicación tenga el bloqueo de instancia.
- [ ] Respaldar SQLite antes de migrar y conservar un mecanismo probado de rollback.
- [ ] Separar completamente el cliente de actualización del núcleo de control de acceso.
