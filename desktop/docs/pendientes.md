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

## Pantallas del plan original (`docs/plan-tauri.md`, grupo 4-5) aún sin construir

- [x] **Auditoría — genérica (contratistas, empresas, usuarios), no sólo contratistas.**
  Pantalla `desktop/src/pantallas/Auditoria.tsx` agregada (2026-08-28), ampliada el mismo
  día a las tres entidades — decisión explícita del usuario, incluyendo `nombre`/`empresa_id`
  de Contratista (antes no se auditaban) y un marcador de "se cambió la contraseña" para
  Usuario (sólo fecha, sin valores, a propósito). Mismo patrón que Historial: fetch-all sin
  paginado (`AppCore::buscar_auditoria_completo`, núcleo) + AG Grid virtualiza del lado del
  cliente, un solo buscador (`quickFilterText`), sin exportación (no se pidió). Columnas
  separadas (Fecha, Hora, Entidad, Tipo, Campo, Valor anterior, Valor nuevo, Modificado por)
  en vez del texto combinado "Campo: antes → después" que usa `--comandos`/TUI clásica —
  mejor para ordenar/buscar en una grilla.
  - Tabla vieja `auditoria_contratistas` reemplazada por `auditoria_cambios` (genérica,
    `entidad`/`entidad_id`/`entidad_nombre` + snapshot de `usuario_nombre`/`entidad_nombre`
    en la propia fila en vez de `JOIN` en vivo — antes un contratista renombrado/borrado le
    hacía perder sentido a filas viejas) — `MIGRACION_13` en `src/database/schema.rs`. Los
    datos existentes eran de prueba, se descartaron sin migrar (decisión explícita).
  - Sin filtro por entidad/fecha/tipo de cambio (ninguna de las otras dos interfaces lo tiene
    tampoco hoy — `FiltroAuditoria` sólo soporta `limite`/`offset`); se agrega si hace falta
    más adelante.

- [x] **Respaldos — descartado de la GUI a propósito (2026-08-28).** Decisión explícita del
  usuario, por seguridad: Respaldos se queda exclusivo de la consola (`--comandos`/TUI
  clásica), no se construye una pantalla en la GUI. No es un "todavía no" — es la decisión
  final salvo que el usuario lo retome explícitamente.

- [x] **RBAC visual en el sidebar.** Resuelto (2026-08-28): `SECCIONES` en `App.tsx` ahora
  acepta un campo opcional `rolesPermitidos: RolUsuario[]` — sección sin ese campo es
  visible para cualquier rol logueado, con el campo se filtra contra `sesion.rol`
  (`seccionesVisibles`, antes de renderizar `.shell-nav`). Sólo "Auditoría" lo usa hoy
  (`["Root", "Administrador"]`, espejo de `RolUsuario::puede(Operacion::VerAuditoria)` en
  `src/domain/autorizacion.rs`) — decisión explícita del usuario de que el resto de las
  pantallas no necesita ocultarse por rol (las acciones puntuales restringidas adentro,
  como activar/desactivar, ya las rechaza el comando correspondiente sin necesidad de
  esconder la sección entera).
