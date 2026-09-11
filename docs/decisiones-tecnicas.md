# Decisiones técnicas

Bitácora de decisiones de infraestructura/tooling que no son obvias leyendo
el código, para que otro dev (o vos mismo en unos meses) no tenga que
reconstruir el razonamiento. Orden cronológico, entradas más nuevas abajo.
No reemplaza los docs de `docs/auditorias/` (esos son diagnóstico técnico
puntual); esto es "por qué está configurado así".

---

## 2026-09-11 — Normalización de EOL (`.gitattributes`)

**Problema:** `desktop/src-tauri/Cargo.toml` aparecía como modificado en
`git status` sin ningún cambio real, incluso recién después de un
`git checkout` limpio, y reaparecía apenas algo (un build) le tocaba el
`mtime`.

**Causa:** el archivo quedó commiteado con CRLF en algún punto, mientras el
resto del repo usa LF. Con `core.autocrlf=true` (típico en Windows), Git
compara el archivo "limpiado" (CRLF→LF) contra el blob guardado -- si el
blob ya era CRLF, nunca coinciden.

**Decisión:** agregar `.gitattributes` con `* text=auto eol=lf` y
renormalizar el árbol (`git add --renormalize .`). Todo archivo de texto
se guarda en LF en el repo sin importar el `autocrlf` de cada máquina.

---

## 2026-09-11 — Switch de motor SQLite de tres vías

**Contexto:** esta rama (`bench/sqlite-3way-*`) existe para comparar tres
motores SQLite -- ver
[`docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`](auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md),
sección 12:

- `cifrado-sqlcipher` -- motor real de producción. Compila OpenSSL
  vendorizado desde fuente (`rusqlite/bundled-sqlcipher-vendored-openssl`),
  lento en frío (¬20-40 min según la máquina) y depende de un Perl completo
  (ver el gotcha de Strawberry Perl en la sección 18 del mismo audit doc).
- `sqlite-plano` -- SQLite sin cifrar, rápido para iterar (`rusqlite/bundled`).
  **Nunca** para builds que tocan datos reales.
- `cifrado-sqlite3mc` -- candidato en evaluación (SQLite3 Multiple Ciphers,
  ChaCha20-Poly1305). **Todavía no enlaza un motor real** -- no hay crate de
  bindings en crates.io, falta decidir e implementar la integración
  (vendorizar el amalgamation vs. link a DLL, sección 11 del audit doc).
  Activarla sola hoy hace que el build falle en el link -- es el
  comportamiento esperado hasta que se resuelva esa integración, no un bug.

Las tres son mutuamente excluyentes (`compile_error!` en `src/lib.rs`); no
elegir ninguna también es error.

### `desktop/src-tauri` -- default invertido a `sqlite-plano`

Antes, `desktop/src-tauri/Cargo.toml` tenía `cifrado-sqlcipher` hardcodeado
en la lista de features de su dependencia a `control_acceso`, así que
`cargo tauri dev`/`build` **siempre** compilaba el motor real sin forma de
pedir otro.

El fix obvio ("agregar un `[features]` que reenvíe al crate raíz, default
= motor real, pedir el plano con `-f`") **no funciona** con `tauri-cli`
2.11.4 (verificado): su flag `-f/--features` sólo agrega features, no
existe `--no-default-features` en `tauri dev`. Si el default fuera un
motor real, pedir el otro con `-f` activaría los dos a la vez y dispararía
la exclusión mutua.

Por eso el default de `desktop/src-tauri` es `sqlite-plano`, no
`cifrado-sqlcipher` -- al revés que el crate raíz. `cargo tauri dev` sin
flags es rápido en cualquier máquina mientras se evalúan los tres motores.
Para compilar/verificar un motor real específicamente, usar `cargo build`
directo (sí soporta `--no-default-features`), no `tauri dev -f`:

```sh
cargo build-desktop-cipher   # cifrado-sqlcipher, sin hot-reload
cargo build-desktop-3mc      # cifrado-sqlite3mc (no compila todavía, ver arriba)
```

(alias en `.cargo/config.toml`, funcionan desde cualquier directorio del
repo vía `--manifest-path`).

**Cuando se decida el motor final para producción**, este default debe
revisarse -- `sqlite-plano` como default de un crate que sí se empaqueta y
distribuye es intencional *sólo* mientras dura la evaluación de esta rama.

### `mobile/rust-core` -- mismo patrón, sin la limitación de CLI

`cargo ndk`/`cargo build`/`cargo test` sí soportan `--no-default-features`
nativamente (no son `tauri-cli`), así que ahí el default se quedó en el
motor real (`cifrado-sqlcipher`) -- comportamiento sin cambios para el día
a día. Alias para iterar rápido:

```sh
cargo build-mobile-plano   # host, --no-default-features --features sqlite-plano
cargo test-mobile-plano
cargo build-mobile-3mc     # ídem, no compila todavía (ver arriba)
```

Para el `.so` de dispositivo (`cargo ndk -t <target> build --release`), agregar
las mismas flags a mano -- `cargo-ndk` reenvía argumentos desconocidos a
`cargo build` (ver `mobile/README.md`).

### rust-analyzer

`.vscode/settings.json` tenía `"rust-analyzer.cargo.features": "all"`, que
activa TODAS las features de Cargo para el análisis del editor -- con dos
(y ahora tres) motores mutuamente excluyentes, eso dispara el
`compile_error!` como falso positivo permanente en `lib.rs`. Cambiado a
`[]` (usa las `default` del crate raíz, `cifrado-sqlcipher`). Es sólo
configuración del editor, no afecta ningún build real.

---

## 2026-09-11 — `mobile/rust-core` desactualizado tras el merge de conflictos de ingreso

Lo que en la entrada anterior quedó anotado como "pendiente" se resolvió
en la misma sesión. Causa real: el commit `72355da` (*"avisa
simétricamente cuando un conflicto de sitios se cuela offline"*) agregó
`conflictos_ingreso` a `ResumenSincronizacion` y migró la apertura de la
conexión secundaria a la fábrica central
(`database::connection::abrir_conexion_secundaria_escritura`), y lo hizo
en desktop (`desktop/src-tauri/src/comandos/nube.rs`) pero no en
`mobile/rust-core/src/lib.rs` -- quedó desincronizado, no relacionado al
switch de motores de esta rama.

Dos fixes, espejando exactamente el patrón ya usado en desktop:

1. **`conflictos_ingreso`**: en `sincronizar_con_secreto` (sync real), se
   calcula con `nube::contratistas_con_conflicto_activo(&conexion,
   &contexto).unwrap_or_default()` -- mejor esfuerzo, igual que desktop, no
   tumba el resto del sync si falla. En
   `configurar_dispositivo_inicial_con_secreto` (activación inicial, base
   recién configurada) queda `Vec::new()`, mismo criterio que desktop y que
   el `From<ResumenSincronizacionNucleo>` ya existente.
2. **`ruta_base_datos()`**: el núcleo (`AppCore`) ya no expone ese método
   (el free-function equivalente en `database::connection` es para el path
   *por defecto*, no el de una instancia abierta). `Nucleo::abrir` en
   mobile YA recibe el path como parámetro del constructor -- el fix fue
   simplemente guardarlo en un campo (`ruta_base_datos: PathBuf`, mismo
   patrón que `GuiState::ruta_base_datos` en escritorio) en vez de intentar
   leerlo de vuelta desde `AppCore`.

Verificado: `cargo build-mobile-plano --lib` compila limpio y
`cargo test-mobile-plano --lib` -- los 12 tests existentes del crate mobile
pasan sin cambios.

---

## 2026-09-11 — Aplanado de roles: la autorización real vive en el panel

**Contexto:** la app nació como un kiosco 100% local (sin nube ni
sincronización posible), así que tenía sentido que ROOT/Administrador
fueran los únicos que podían crear usuarios, ver auditoría o configurar la
nube -- no había otra autoridad. Con la nube ya como fuente de verdad
(`administradores_panel`/Supabase Auth deciden quién existe y qué rol
tiene), esa duplicación de autoridad ya no aporta nada: sólo agrega
fricción y una superficie extra que mantener sincronizada con el panel.

**Decisión:** las apps (desktop/TUI/CLI) dejan de negarle ninguna acción a
nadie por su rol -- quien tiene una sesión válida (que ya pasó el filtro
del panel) puede todo dentro de la app. La única protección que se
mantiene es que nadie asigna/gestiona el rol ROOT salvo otro ROOT
(`puede_gestionar_usuario` en `src/domain/autorizacion.rs`) -- no es un
gate de "quién ve qué pantalla", es específicamente para que nadie se
autopromueva a la identidad más alta desde un formulario.

`RolUsuario::puede(Operacion)` pasa a devolver `true` siempre. Los ~30
call-sites que hacían `if !rol.puede(...) { return Err(...) }` quedan
como código muerto (nunca se disparan) en vez de removerse uno por uno --
riesgo/beneficio no lo justificaba en esta pasada; queda como limpieza
futura si se quiere.

**Se encontraron y cerraron dos gates duplicados** que NO pasaban por
`Operacion`/`puede()` (por eso `cargo test-plano` no los detectó
automáticamente al cambiar sólo `autorizacion.rs` -- hubo que buscarlos a
mano y corregir los tests que asumían la restricción vieja):
- `src/tui/menu_principal/state.rs::visible_para` -- ocultaba
  Usuarios/Auditoria del menú a un Operador con un `match` hardcodeado,
  sin tocar `Operacion::VerAuditoria`/`GestionarUsuarios`.
- `src/tui/contratistas/state.rs` -- `cedula_editable`/`acceso_editable` sí
  venían de `Operacion::EditarCedulaContratista`/`ActivarDesactivarContratista`
  (ya aplanados), pero varios tests de navegación de formulario (`campo`,
  orden de Tab) tenían hardcodeado el índice que resultaba de excluir
  Cédula -- se actualizaron para reflejar que Cédula es el campo 0 para
  cualquier rol ahora.

**Auditoría se queda local** (no migra a Supabase, revirtiendo la idea de
la entrada anterior) -- con roles aplanados no queda ninguna pantalla
"sólo para admin" que justificar por presencia física en desktop, así que
mover la tabla es ingeniería sin beneficio real. La transparencia del log
(quién cambió qué) es el control en sí mismo, no algo que haya que
restringir a quien lo vea -- no son datos sensibles.

**`GestionarNube` no se aplanó, se eliminó** -- la "pantalla Nube" de
post-login (repegar/rotar el secreto de dispositivo desde una sesión ya
abierta) resultó ser código muerto en las tres plataformas: ningún
componente de desktop ni de Android la llamaba (`guardarSecretoDispositivo`/
`secretoDispositivoGuardado` existían en `desktop/src/api/nube.ts` y como
comandos Tauri, pero sin ninguna pantalla que los invocara). Se eliminaron
esos dos comandos Tauri, su registro en `lib.rs`, y sus wrappers en
`api/nube.ts`. El secreto de dispositivo se configura una sola vez, en el
arranque inicial (`configurar_dispositivo_inicial`), y no se vuelve a
tocar desde adentro de la app corriendo. `AppCore::guardar_secreto_dispositivo`/
`secreto_dispositivo_guardado` (núcleo) se dejaron intactos porque
`mobile/rust-core` los sigue exponiendo vía `uniffi` (tampoco los llama
ninguna pantalla de Android hoy, pero tocar los bindings generados de Kotlin
sin poder recompilar/verificar el lado Android en este entorno era más
riesgo que beneficio para esta pasada).

### Cierre del bootstrap local de ROOT

**Problema:** cuando la base local está vacía (`requiere_configuracion_inicial`),
tanto `--cli` como `--tui-clasica` ofrecían una cadena `RootCedula → RootNombre
→ RootPassword → RootConfirmarPassword` que fabricaba un usuario ROOT sin
verificar nada contra la nube -- sólo hacía falta tener el ejecutable y el
archivo de la base (o ni siquiera eso: bastaba con que el archivo no
existiera o se borrara). Vestigio directo de la era kiosco-sin-nube;
con `configurar_dispositivo_inicial` (desktop) ya trayendo el catálogo real
desde Supabase con el secreto de dispositivo, esa ventana ya no tenía
ninguna función legítima -- sólo quedaba como una vía de "colarse"
localmente en cualquier PC del sitio.

**Decisión:** ni `--cli` ni `--tui-clasica` vuelven a ofrecer esa cadena.
`run_cli`/`run_tui_clasica` (`src/main.rs`) chequean `requiere_configuracion_inicial()`
apenas abren el `AppCore` y, si es `true`, imprimen un mensaje remitiendo a
la app de escritorio y salen sin entrar a la terminal -- el arranque real
de un sitio nuevo es exclusivamente desktop + secreto de dispositivo. El
código de la cadena `Fase::RootCedula`/`src/cli/root.rs` queda sin tocar
(no se justificaba la cirugía de sacarlo del enum y sus `match` en esta
pasada) pero inalcanzable: nada llama a `AppState::nueva_configuracion_inicial()`.

También se eliminó `--reset-root` (resetear la contraseña de un ROOT
existente sin loguearse, para recuperación). Con ROOT unificado a la misma
identidad que gestiona el panel (`administradores_panel`/Supabase Auth), el
reset de contraseña pasa a vivir ahí (`admin-reset-password-usuario`, ya
desplegado) -- no hace falta un camino de recuperación aparte por CLI.

### Pendiente, sin implementar todavía

- **Backlog (seguridad/UX):** cuando una entrada se deja pasar porque el
  chequeo de conflicto contra la nube (`nube::contratistas_con_conflicto_activo`)
  no pudo correr por falta de internet, hoy queda indistinguible de un
  chequeo limpio en el historial. Falta: registrarlo explícito en auditoría
  y avisarle al operador al reconectar, para que quede claro que fue una
  limitación de conectividad, no negligencia.
- **Backlog (seguridad, mobile):** `mobile/rust-core`/`LoginViewModel.kt`/
  `PantallaFijarPasswordInicial.kt` siguen con el mismo patrón de "reclamar
  cuenta con sólo la cédula" que se cerró en desktop
  (`docs/plan-autenticacion-supabase-auth.md`) -- portar Supabase Auth a
  mobile es un trabajo aparte y más grande, no incluido en esta pasada.
- **Delicado, no implementado:** una vez que se decida la unificación de
  identidad (ROOT = admin del panel) y se termine de migrar todo lo de
  arriba, hace falta vaciar los datos de prueba/transición actuales en
  Supabase y repoblar con datos frescos y consistentes con el modelo nuevo
  -- no alcanza con dejar el esquema andando si los datos que ya existen
  quedaron a mitad de camino entre el modelo viejo (ROOT local por
  dispositivo) y el nuevo (ROOT = identidad única del panel). Requiere
  cuidado -- es la base de producción, no un ambiente de prueba.

Verificado (2026-09-11): `cargo test-plano --lib` (591 tests, núcleo +
TUI/CLI), `cargo check`/`cargo build-desktop-cipher --lib` (desktop, los
dos motores), `tsc --noEmit` y `vitest run` (desktop frontend, 203 tests) --
todo limpio tras el aplanado de roles y el cierre del bootstrap de ROOT.

---

## 2026-09-12 — Reconciliación de drift de migraciones + GitHub Integration + limpieza de Advisors

**Contexto:** al preparar la conexión de GitHub Integration de Supabase
(deploy automático a producción al mergear a `main`), se comparó
`supabase/migrations/*.sql` contra `supabase_migrations.schema_migrations`
de producción antes de activarla -- resultado: **drift real**, no solo
teórico.

**Hallazgo 1 -- 19 migraciones con timestamp de archivo incorrecto:** mismo
contenido que lo aplicado en producción, pero el nombre del archivo tenía un
timestamp distinto al que realmente quedó registrado (varios con timestamps
"redondos" tipo `20260906130000`, sugiriendo renumeración manual en algún
punto). Si se activaba GitHub Integration con "Deploy to production" así,
Supabase las iba a tratar como migraciones nuevas y reintentarlas -- al
menos 3 (`CREATE POLICY`/`ADD COLUMN` sin `IF NOT EXISTS`) habrían fallado a
mitad de un deploy real. Se renombraron los 19 archivos a su versión real
(`git mv`, sin cambiar contenido).

**Hallazgo 2 -- 10 migraciones aplicadas en producción, ausentes de git en
cualquier rama, siempre:** incluye toda la tabla `movimientos_visita`, la
RPC `crear_cita_anfitrion`, y el fix de recursión infinita de RLS entre
`citas`/`cita_sitios`. Rescatadas con
`select statements from supabase_migrations.schema_migrations` (esa tabla
guarda el SQL real de cada versión aplicada) y versionadas con su número
real.

**Hallazgo 3 -- el esquema `private` nunca tuvo su propio `CREATE SCHEMA`
versionado** -- se creó a mano en algún punto antes de la primera migración
que lo usa. Se agregó `create schema if not exists private` como migración
nueva (no-op contra producción, donde ya existía) para que una
reconstrucción desde cero no falle.

Verificado: comparación 1:1 de los 60 (luego 63, tras las migraciones de
esta misma entrada) timestamps de archivo local contra
`select version from supabase_migrations.schema_migrations` -- coinciden
exactamente. GitHub Integration se conectó recién después de esta
reconciliación, no antes.

### Limpieza de Performance/Security Advisors (misma sesión)

Con GitHub ya conectado y pidiendo "dejar esto fino", se resolvieron los
hallazgos de bajo riesgo que quedaban:

- **12 políticas RLS de `citas`/`cita_sitios`/`cita_visitantes`/`sitios`/
  `anfitriones`** reevaluaban `auth.email()` fila por fila (no estaba
  envuelto en `select`, a diferencia de `auth.jwt()` en esas mismas
  políticas) -- corregido con `ALTER POLICY`.
- **3 FKs sin índice** (`cita_sitios.sitio_id`,
  `movimientos_visita.dispositivo_entrada_id/salida_id`) + **1 índice
  duplicado** en `gafetes` -- corregidos.
- **4 tablas con políticas RLS permisivas duplicadas**
  (`anfitriones`/`dispositivos`/`sitios`/`usuarios`, cada una con dos
  políticas separadas para el mismo rol+acción) -- fusionadas en una sola
  con el mismo `OR` explícito. `dispositivos` y `usuarios` están publicadas
  a Realtime -- fusionar no cambia qué filas ve/escribe cada rol, sólo
  cuántas veces se evalúa, así que no afecta qué le llega a cada cliente
  por Postgres Changes.
- **`public.es_admin_global()` invocable por HTTP** (cualquier
  `authenticated` podía pedir `/rest/v1/rpc/es_admin_global`, aunque solo
  revela el estado del propio llamador) -- movida a `private` con
  `ALTER FUNCTION ... SET SCHEMA`, que preserva el OID de la función: las
  16 políticas que ya la usaban (incluida una de `realtime.messages`,
  presencia) siguieron funcionando sin tocarlas una por una, porque
  Postgres resuelve la llamada por OID, no por nombre calificado.

**Verificación exhaustiva antes/después de cada cambio de RLS:** se corrieron
las 8 baterías de diagnóstico de `supabase/tests/*_autorizacion.sql` a mano
contra producción (transacción con `rollback`, cero riesgo de dejar datos
de prueba) más un caso nuevo de presencia+admin_global (el único camino que
ejercitaba `es_admin_global()` dentro de Realtime), antes y después de cada
migración -- resultado idéntico en los 9 casos, las dos veces. Se
actualizaron los comentarios de `dispositivos_autorizacion.sql`,
`usuarios_autorizacion.sql` y `sitios_autorizacion.sql` para reflejar los
nombres de política fusionados, y se agregó cobertura del caso "anfitrión"
a `sitios_autorizacion.sql` (no estaba cubierto, aunque la política ya
existía en producción desde el 2026-09-09).

**Pendientes, deliberadamente no tocados:** `pg_net` en esquema `public`
(mover una extensión con dependencias activas amerita su propia pasada);
"Leaked password protection" desactivado (toggle de dashboard, no de SQL --
la razón original para dejarlo desactivado ya no aplica, ver
`docs/arquitectura-supabase.md` sección 6.4); A-01 (RLS cross-site,
riesgo aceptado y documentado, no un olvido).

Se escribió `docs/arquitectura-supabase.md` como documento de referencia
completo (modelo de datos, los dos sistemas de autorización, Realtime en
detalle, Edge Functions, Vault, flujo de migraciones) -- ese archivo es el
mapa completo del "cómo funciona hoy"; este archivo sigue siendo el
registro cronológico del "por qué".
