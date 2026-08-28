# Pendientes — GUI Tauri (`desktop/`)

Igual que `docs/pendientes.md` en la raíz, pero para lo específico de la GUI de escritorio
(`desktop/`). Cosas de bajo impacto que no bloquean nada hoy, pero que conviene revisar más
adelante en vez de perderlas en la conversación.

## Regla de este documento

**Al terminar una tarea de aquí, se marca `[x]` en el mismo commit (o el siguiente) que la
resuelve.** Si se descarta en vez de hacerse, se marca `[x]` igual con una nota de por qué.

---

## Errores

- [ ] **`AutenticacionError::Database` filtra el mensaje crudo de SQLite al login.**
  `AutenticacionError` (`src/services/error.rs`) tiene `#[error(transparent)]
  Database(#[from] DatabaseError)`, y `DatabaseError::Sqlite` interpola el error crudo de
  rusqlite (`"Error de SQLite: {0}"`). El comando `login`
  (`desktop/src-tauri/src/comandos/autenticacion.rs:29`) hace `error.to_string()` sin
  distinguir variantes, así que un fallo de infraestructura (base bloqueada, corrupta,
  columna inesperada) mostraría ese mensaje interno en pantalla en vez de un genérico. El
  camino normal (cédula inexistente / contraseña incorrecta) sí está bien: ambos colapsan
  a `CredencialesInvalidas` a propósito, sin filtrar cuál de las dos falló. Caso raro en la
  práctica — sólo pasa con fallos reales de la base, no con credenciales mal digitadas.
  Corrección si se retoma: mapear `AutenticacionError::Database` a un mensaje genérico
  ("Error interno, intenta de nuevo") en el comando, en vez de dejar pasar el `to_string()`
  transparente.

## Diferido a propósito (Historial v1)

- [ ] **Historial: sin filtro de fecha, sin vista Timeline.** Decisión explícita del
  usuario: v1 de la pantalla Historial (`desktop/src/pantallas/Historial.tsx`) trae todo
  el historial de una vez (`buscar_historial_completo`, sin paginado — AG Grid
  virtualiza/filtra del lado del cliente, igual que Activos) con un rango de fechas fijo y
  abierto (año 2000 a mañana, `filtro_sin_acotar()` en
  `desktop/src-tauri/src/comandos/historial.rs`) porque `FiltroHistorial` exige
  `desde`/`hasta` siempre. La vista Timeline (vs. la Clásica/tabla ya implementada) queda
  para una fase posterior — no implementar sin retomarlo con el usuario primero.
  (La exportación SÍ respeta ahora el filtro por fila y las columnas ocultas de la grilla
  — ver `AppCore::exportar_historial_seleccion` y `ColumnaHistorial::from_clave` — eso ya
  no está diferido.)
