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
  (La exportación SÍ respeta ahora el filtro por fila, las columnas ocultas y el orden de
  la grilla — ver `AppCore::exportar_historial_seleccion`, `ColumnaHistorial::from_clave` y
  `AppCore::movimientos_en_orden` — eso ya no está diferido.)

- [ ] **Módulo de exportación avanzado: fuente/tamaño de letra y formato de celda
  configurables.** `rust_xlsxwriter` (ya en uso) sí soporta esto sin herramienta adicional
  — `Format::set_font_name`/`set_font_size`/`set_bold`/`set_font_color` para tipografía,
  `set_num_format` para tipo de celda (numérico vs. texto; ya se usa para fecha/hora en
  `FormatosHistorial`, `src/historial/exportacion.rs`), más bordes/alineación/color de
  fondo (hoy sólo aplicados al encabezado, no a las filas de datos). Decisión explícita del
  usuario (2026-08-28): no se define el alcance todavía — se retoma como un módulo de
  exportación avanzado más adelante, con el usuario definiendo primero qué configurar
  (tipografía general, Gafete como celda numérica en vez de texto, bordes/zebra en filas de
  datos, u otra cosa puntual).
