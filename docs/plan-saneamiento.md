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
- [ ] **Respaldo y recuperación (alta prioridad antes de producción; ejecución
  aplazada).** Seguir el [plan específico de respaldos](plan-respaldos.md): crear copias
  consistentes mediante la API de SQLite, definir retención y probar una restauración
  completa con rollback. Debe estar terminado antes de una actualización productiva
  que incluya migraciones.
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
- [ ] **Eventos denegados.** Definir y persistir intentos de acceso denegados si el
  historial funcionará como auditoría de seguridad.
- [ ] **Sesión vigente.** Comprobar que el usuario siga activo antes de cada movimiento,
  cerrar su sesión si es desactivado y añadir bloqueo por inactividad.
- [x] **Política horaria explícita.** Centralizar un reloj, usar la zona
  `America/Costa_Rica`, persistir UTC/offset y detectar retrocesos del reloj respecto al
  último movimiento.
- [ ] **Errores observables.** Mantener mensajes sencillos para el operador y escribir
  logs técnicos persistentes con contexto e identificador de incidente, sin secretos.
  No descartar resultados mediante `.ok()`.
- [x] **Lecturas paginadas coherentes.** Obtener el total y la página del historial en
  la misma lectura de SQLite y navegar con un cursor estable, para no repetir u omitir
  movimientos si los datos cambian entre páginas.

## Prioridad 2: ejecución y mantenibilidad

- [x] **TUI responsiva (login y búsquedas).** Se eliminó la espera artificial de 800ms del
  login; la verificación de Argon2 corre en un hilo aparte (primer uso de threading en la app,
  acotado a esa verificación — no bloquea `terminal.draw`). Las 5 pantallas de búsqueda
  (Historial, Contratistas, Activos, Empresas, Usuarios) tienen debounce de 250ms vía
  `ui_kit::Debounce`, en vez de una consulta SQL por tecla. Pendiente, señalado
  deliberadamente aparte: Argon2 en hilo aparte para crear usuario/cambiar contraseña/ROOT
  inicial — requiere partir validación de guardado en el service de usuarios, mayor riesgo,
  se aplaza a un pase propio.
- [ ] **Modo demo aislado.** Sustituir `Option<&AppCore>` por una implementación falsa
  explícita disponible sólo en pruebas o mediante una feature de desarrollo.
- [ ] **Estados sin `unwrap`.** Hacer que cada etapa de ingreso contenga los datos que
  necesita, de modo que los estados inválidos no puedan construirse.
- [ ] **Normalización de identidades.** Definir reglas canónicas para cédulas y nombres
  únicos y reforzarlas en SQLite.
- [x] **Configuración SQLite explícita (perfil base).** `busy_timeout`, `journal_mode`,
  `synchronous`, `application_id`, `trusted_schema` y `quick_check`/`optimize` ya están
  implementados y probados — ver la
  [evaluación y recomendaciones de SQLite](evaluacion-sqlite.md), secciones 2-6. Sigue
  pendiente el tratamiento explícito de `SQLITE_BUSY` como error observable para el
  operador (ver el punto de abajo) y la prueba de cierre/pérdida de energía simulada en
  un entorno real.
- [ ] **Capas sin dependencia de SQLite.** Mover los contratos y errores neutrales a
  aplicación/dominio y dejar que el adaptador de base de datos traduzca los errores de
  `rusqlite`.
- [ ] **Pruebas operativas.** Cubrir base bloqueada, disco lleno o no escribible,
  recuperación de respaldo, reloj incorrecto y cierre inesperado.
- [ ] **Verificación automatizada.** Ejecutar en cada cambio `cargo test`,
  `cargo fmt --check` y Clippy con advertencias tratadas como errores.

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
