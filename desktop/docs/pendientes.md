# Pendientes — GUI Tauri (`desktop/`)

Igual que `docs/pendientes.md` en la raíz, pero para lo específico de la GUI de escritorio
(`desktop/`). Cosas de bajo impacto que no bloquean nada hoy, pero que conviene revisar más
adelante en vez de perderlas en la conversación.

## Regla de este documento

**Al terminar una tarea de aquí, se marca `[x]` en el mismo commit (o el siguiente) que la
resuelve.** Si se descarta en vez de hacerse, se marca `[x]` igual con una nota de por qué.

---

## Exportación PDF de Historial (2026-08-30)

- [x] **Historial a PDF, implementado — terminó siendo HTML/CSS + WebView2, no Typst.**
  Se retomó esta idea con el usuario y, tras comparar Typst / `printpdf`-`genpdf` / HTML
  renderizado por el WebView2 que la app ya trae, se eligió esta última: cero dependencias
  de tipografía/parsing nuevas, y el diseño se pudo iterar como una página web común
  (`.html` abierto directo en el navegador) en vez de a ciegas — el bloqueo original de
  "sin forma de previsualizar un PDF" quedó resuelto así, no evitado.
  Mismo criterio que Excel: exporta exactamente los `ids`/columnas que la grilla tiene
  filtrados/visibles (`pdf::generador::generar_pdf`, `desktop/src-tauri/src/pdf/`), no un
  "resumen del día" aparte — ese tipo de reporte específico (y cualquier otro tipo de
  generación — por contratista, etc.) queda pendiente como su propia fase si se pide.
  **Alcance de bajo nivel — no armar sin retomarlo primero:** numeración de página
  estilizada ("Página X de Y") necesita el DevTools Protocol de WebView2
  (`Page.printToPDF` vía `CallDevToolsProtocolMethod`, no `ICoreWebView2PrintSettings`,
  que sólo da un footer fijo de Chromium sin estilo propio) — no investigado a fondo,
  descartado para esta vuelta.
  **Bug real encontrado y resuelto durante la implementación, dejar registrado por si
  reaparece en otro lado:** el completion handler de `PrintToPdf` no es confiable en este
  embedding (Tauri 2.11 + wry 0.55 + `webview2-com` 0.38.2) — probadas tres formas de
  esperarlo (channel manual, `PrintToPdfCompletedHandler::wait_for_async_operation` —la
  función de la librería pensada justo para esto—, y esa misma función movida afuera del
  callback `on_page_load`), y en las tres el PDF se escribía bien y rápido en disco pero el
  aviso de "terminé" nunca llegaba a Rust (confirmado con logging contra datos reales, no
  es una suposición). La solución que quedó: disparar `PrintToPdf` sin esperar su
  callback y confirmar que terminó sondeando el archivo en disco hasta que su tamaño se
  estabiliza (`pdf/generador.rs`, `esperar_archivo_listo`). Si esto se puede reproducir de
  forma aislada valdría la pena reportarlo río arriba (wry/webview2-com), pero no se
  investigó ese camino todavía.

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

- [x] **`AutenticacionError::Database` filtraba el mensaje crudo de SQLite al login
  (2026-08-29).** Se agregó `mensaje_autenticacion` a `src/mensajes.rs` (mismo criterio que
  `mensaje_empresa`/`mensaje_contratista`/etc.): `CredencialesInvalidas`/`UsuarioInactivo`
  conservan su texto, `HashInvalido` y `Database(_)` colapsan juntos a "No se pudo iniciar
  sesión, intentá de nuevo" — los dos son fallos de infraestructura, no algo que el usuario
  hizo mal, así que no hace falta distinguirlos en pantalla. **El bug no era sólo de la
  GUI:** `src/tui/app/auth_jobs.rs` (líneas 65 y 101) hacía el mismo `error.to_string()`
  directo sobre `AutenticacionError` — se corrigió ahí también, no sólo en
  `desktop/src-tauri/src/comandos/autenticacion.rs`. Con tests
  (`mensajes::tests::un_fallo_de_sqlite_en_el_login_no_filtra_el_mensaje_crudo`).

## Diferido a propósito (Historial v1)

- [x] **Historial: filtro de fecha real, agregado (2026-08-29).** Ya no trae el rango fijo
  abierto (año 2000 a mañana) — `desktop/src-tauri/src/comandos/historial.rs` acepta
  `desde`/`hasta` opcionales desde la pantalla (`SelectorRangoFecha.tsx`: botón "Período"
  con accesos rápidos — Hoy/Ayer/Esta semana/Semana pasada/Este mes/Mes pasado/Últimos
  7-30 días — más los dos campos para un rango custom). Por defecto trae los últimos 6
  meses. Sigue sin paginar (AG Grid virtualiza/filtra del lado del cliente sobre lo que
  trae el rango elegido, igual criterio que Activos).
  (La exportación SÍ respeta el filtro por fila, las columnas ocultas y el orden de la
  grilla — ver `AppCore::exportar_historial_seleccion`, `ColumnaHistorial::from_clave` y
  `AppCore::movimientos_en_orden`.)

- [x] **Vista Timeline: descartada (2026-08-29).** Se evaluaron 4 enfoques (agenda de un
  día, feed cronológico por día, calendario con densidad, timeline continua sobre el
  rango) — decisión explícita del usuario tras verlos: no le encuentra utilidad real para
  este proyecto. No es un "todavía no" — es la decisión final salvo que se retome
  explícitamente. La vista Clásica (tabla, ya implementada) queda como única vista de
  Historial.

- [x] **Tipografía y cebra en la exportación a Excel (2026-08-29).** Toda la hoja (antes
  sólo el encabezado) pasa a Arial 10 negrita, con filas de datos alternadas
  celeste/blanco — pedido explícito del usuario, confirmado contra una muestra generada a
  mano antes de commitear. `FormatosHistorial` (`src/historial/exportacion.rs`) pasó de un
  `Format` por tipo de columna a `[Format; 2]` (fila impar/par), elegido según `fila % 2`
  en `escribir_movimiento`. Gafete como celda numérica y bordes en filas de datos quedan
  fuera de este cambio — no se pidieron, se agregan si hace falta más adelante (misma
  API de `rust_xlsxwriter`, `set_num_format`/`set_border` ya en uso para otras columnas).

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
