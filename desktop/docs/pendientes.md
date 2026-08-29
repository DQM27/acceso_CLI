# Pendientes — GUI Tauri (`desktop/`)

Igual que `docs/pendientes.md` en la raíz, pero para lo específico de la GUI de escritorio
(`desktop/`). Cosas de bajo impacto que no bloquean nada hoy, pero que conviene revisar más
adelante en vez de perderlas en la conversación.

## Regla de este documento

**Al terminar una tarea de aquí, se marca `[x]` en el mismo commit (o el siguiente) que la
resuelve.** Si se descarta en vez de hacerse, se marca `[x]` igual con una nota de por qué.

---

## Empaquetado y actualizaciones

- [x] **Pipeline de release para la GUI (2026-08-29).** Antes `.github/workflows/release.yml`
  sólo compilaba y publicaba `control_acceso.exe` (consola) al pushear un tag `v*` — no
  existía ningún paso que generara un instalador de `desktop/`. Se agregó el job
  `build-gui` (mismo workflow, mismo trigger): instala Node, `npm ci` en `desktop/`, corre
  `npm run test` (Vitest) y `cargo test` en `desktop/src-tauri`, y finalmente
  `npm run tauri build` — publica `.msi`/`.exe` (`bundle/msi/`, `bundle/nsis/`) como
  artefacto (`actions/upload-artifact`, igual que el binario de consola). Se agregó
  `@tauri-apps/cli` como dependencia y el script `"tauri": "tauri"` a `desktop/package.json`
  (no existía ninguno de los dos). Validado localmente con `npm run tauri build --no-bundle`
  (el build de instaladores Windows — MSI/NSIS — no se puede probar en este entorno Linux;
  se valida recién en el runner `windows-latest` de CI).
- [x] **Decisión revisada: actualizaciones vía GitHub, automáticas (2026-08-29 → ampliado
  el mismo día).** Primero se documentó como "vía GitHub, manual" (sin plugin, el usuario
  baja el instalador a mano). El mismo día se decidió ir por la versión automática en vez
  de esa — sigue siendo "vía GitHub" (GitHub Releases como origen, sin servidor propio),
  pero ahora con `tauri-plugin-updater` avisando sola cuando hay versión nueva. El job
  `build-gui` de `release.yml` se reescribió para usar `tauri-apps/tauri-action` en vez de
  `npm run tauri build` + `actions/upload-artifact` — esa acción compila, firma cada bundle
  y publica un GitHub Release de verdad (con `latest.json` adjunto), no un artifact de
  Actions — así queda resuelto también el pendiente que había quedado abierto sobre
  artifact-vs-Release-público. `releaseDraft: true` a propósito: alguien revisa y publica a
  mano en vez de que cada tag quede público apenas termina el build.

- [ ] **Falta el paso manual: generar el par de llaves de firma y cargarlo como secretos del
  repo.** El código ya está listo (`tauri-plugin-updater`/`tauri-plugin-process` en
  `desktop/src-tauri/Cargo.toml`, registrados en `lib.rs`; `plugins.updater` en
  `tauri.conf.json` con `pubkey` todavía en `"REEMPLAZAR_CON_LA_LLAVE_PUBLICA_DE_TAURI_SIGNER_GENERATE"`;
  chequeo + toast de `sonner` con botón "Actualizar" en `App.tsx`/`Shell`, ver
  `api/actualizaciones.ts`), pero sin la llave real el updater no puede verificar nada. A
  propósito no se generó la llave privada dentro de esta sesión — es un secreto de firma de
  código, no algo que deba pasar por un log o un archivo de un contenedor efímero. Falta,
  hecho por una persona con acceso al repo:
  1. `npx tauri signer generate -w ~/.tauri/control-acceso.key` (local, fuera de CI).
  2. Pegar la llave pública que imprime en `tauri.conf.json` → `plugins.updater.pubkey`,
     reemplazando el placeholder.
  3. Cargar la llave privada y su contraseña como secretos del repo en GitHub:
     `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (Settings → Secrets
     and variables → Actions) — nunca committeados, nunca en texto plano fuera de ahí.
  Hasta que esto se haga, un tag `v*` va a fallar el paso de firma del job `build-gui` (o
  publicar un bundle sin firmar si `createUpdaterArtifacts` llegara a tolerarlo, lo cual no
  hay que asumir sin probarlo).

## Robustez

- [x] **Error Boundary de React (2026-08-29).** No existía ninguno — un error de render sin
  capturar en cualquier pantalla tumbaba todo el árbol de React (pantalla en blanco, sin
  mensaje, sin forma de recuperarse salvo reiniciar la app a mano). Se agregó
  `componentes/ErrorBoundary.tsx` envolviendo `<App />` en `main.tsx`: pantalla de error con
  el mensaje y un botón "Reiniciar" (recarga la ventana) en vez del blanco total. Con tests
  (`ErrorBoundary.test.tsx`).

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
