# Plan Tauri — GUI de escritorio (definitivo)

> Sustituye al borrador original de este mismo archivo. El borrador proponía Tauri sin comparar
> alternativas ni resolver el detalle de aislamiento de dependencias; este documento incorpora esa
> propuesta, la contrasta contra el código real, y agrega las decisiones de producto que se
> tomaron con el usuario (2026-08-27). El borrador original sigue disponible en `git log` si hace
> falta consultarlo.

## Contexto

`control_acceso` tiene hoy dos interfaces de terminal que conviven sobre el mismo núcleo: la TUI
clásica (`src/tui/`, ratatui+crossterm) y `--comandos` (`src/comandos/`, la interfaz por defecto).
Este documento agrega una tercera: una GUI de escritorio con Tauri v2.

**Decisión de producto explícita: la GUI complementa, no reemplaza.** La meta es tener **tres
formas de usar la misma aplicación**, y que cada usuario elija la que prefiera. La lógica de
negocio se escribe **una sola vez**; cada interfaz sólo conecta su propia forma de mostrar/capturar
datos. Ninguna interfaz tiene fecha de congelamiento ni de reemplazo.

**La GUI no es un calco.** Cada pantalla, al construirse, se diseña como su propio sistema gráfico
(tablas, mouse, modales) — no una traducción literal del layout de texto de la TUI/comandos. Sólo
se reutiliza la lógica detrás (qué datos, qué reglas, qué errores), nunca el layout.

**Estado actual: sólo planeación.** No hay código de la GUI todavía. Cuando se decida empezar, el
orden ya acordado es incremental: login primero, después las tablas.

## Veredicto de viabilidad

**Sí, es viable, y no implica duplicar el proyecto** — con una condición estructural concreta
(Fase 0) que hay que resolver antes de escribir la primera línea de la GUI. Verificado en el código,
no sólo asumido:

- `src/lib.rs` ya es una librería separada de `src/main.rs` — el mecanismo de Cargo para que un
  binario nuevo reutilice el núcleo ya existe hoy, sin cambios.
- Cero archivos de `application/`, `services/`, `domain/`, `database/`, `models/` importan
  `ratatui`/`crossterm`/`tui-input` (grep exhaustivo: 79 archivos con esos crates, 100% dentro de
  `src/tui/**` o `src/comandos/**`).
- `AppCore` (`src/application/mod.rs`) ya es consumido hoy por **dos** interfaces de terminal
  distintas sin una sola regla de negocio duplicada — precedente real de que "una interfaz nueva
  sobre `AppCore`" funciona en este proyecto.
- ~375 tests de negocio/persistencia no dependen de ninguna interfaz (instancian `AppCore`
  directamente).

Lo único que falta antes de empezar: un feature flag en Cargo que aísle `comandos`/`tui` (y por lo
tanto `ratatui`/`crossterm`/`tui-input`) para que el binario GUI no los arrastre (Fase 0, abajo).

Dimensión honesta del esfuerzo: de 37 247 líneas en `src/`, el 79% (29 574 líneas) es interfaz
(`tui/`+`comandos/`) y sólo 18% es núcleo. Sumar una tercera interfaz es escribir una capa de
presentación nueva de tamaño comparable a las que ya existen — eso es inherente a "3 formas de ver
lo mismo", no un atajo posible. Lo que sí se evita es duplicar lógica de negocio.

## Arquitectura de fondo (ya existe, no se inventa)

```
TUI clásica ─┐
comandos ────┼──► AppCore (src/application/) ──► services/ ──► database/ (9 traits + SQLite) ──► SQLite
Tauri (nuevo)┘         (fachada única,              (genéricos sobre
                     sin reglas propias)              traits de repo)
```

SQLite es un solo archivo local (`%LOCALAPPDATA%\ControlAcceso\control_acceso.db`, o
`CONTROL_ACCESO_DB` si está seteada — `src/database/connection.rs`), de una sola máquina por diseño
explícito (`README.md`). Un candado de instancia (`src/instancia.rs`, `File::try_lock`) impide que
dos procesos abran la misma base a la vez — relevante para desarrollo (ver Riesgos).

## Fase 0 — Preparación (bajo riesgo, reversible, cero cambio de comportamiento)

**1. Feature flag `terminal-ui`** en `Cargo.toml` (raíz). **Ojo, esto ya se implementó y reveló un
error real de la primera versión de este plan:** gatear sólo los *módulos* (`#[cfg(feature =
"terminal-ui")] pub mod tui;`) no alcanza — Cargo sigue compilando `ratatui`/`crossterm`/
`tui-input` como dependencias igual, porque nada le dijo que son opcionales. Hay que marcar los
crates mismos `optional = true` y ligarlos a la feature con `dep:`:
```toml
[features]
default = ["terminal-ui"]
# Verificado por grep: cero uso fuera de src/tui/** y src/comandos/**.
# rust_xlsxwriter queda AFUERA a propósito — application/historial.rs lo usa
# para exportar, y eso lo necesita cualquier interfaz, no sólo la terminal.
terminal-ui = ["dep:ratatui", "dep:crossterm", "dep:tui-input", "dep:query-parser", "dep:rpassword"]
dev-auth = []

[dependencies]
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"], optional = true }
crossterm = { version = "0.29.0", optional = true }
tui-input = { version = "0.15.4", default-features = false, features = ["crossterm"], optional = true }
query-parser = { version = "0.2.0", optional = true }
rpassword = { version = "7", optional = true }
```
En `src/lib.rs`:
```rust
#[cfg(feature = "terminal-ui")]
pub mod comandos;
#[cfg(feature = "terminal-ui")]
pub mod tui;
```
El binario de consola (`src/main.rs`) sigue compilando con las features por defecto — cero cambio
de comportamiento (confirmado: `cargo build`, `cargo test`, `cargo test --features dev-auth` y
`cargo clippy --all-targets -- -D warnings` en verde después del cambio). La verificación real de
que el aislamiento funciona no es `cargo check --no-default-features` a secas (eso falla porque
también intenta compilar el binario de consola, que sí necesita esos crates) — es
`cargo check --no-default-features --lib`, y sobre todo `cargo tree -i ratatui` **desde
`desktop/src-tauri`**, que debe devolver "did not match any packages".

**2. Extraer `src/tui/app/error_messages.rs`** (mapea errores de servicio a texto humano) a
`src/mensajes.rs`, un módulo neutral público (`pub mod mensajes;` en `lib.rs`, sin feature-gate —
lo van a usar tanto `tui`/`comandos` como, más adelante, los comandos Tauri). Ya hecho: funciones
pasaron de `pub(super)` a `pub`, y se actualizaron los 4 imports que lo usaban dentro de `tui/`
(`auth_jobs.rs`, `actions/{catalogos,admin,accesos}.rs`). Deliberadamente **no** se tocó la copia
manual paralela en `src/comandos/operando.rs` (~línea 519) — su texto difiere un poco del de
`mensajes.rs` (p. ej. antepone "Acceso denegado: " al motivo), y unificarla ahora habría cambiado
el texto que ve el usuario de `comandos` — eso es un cambio de comportamiento, fuera del alcance de
"cero cambio" de esta fase. Se consolida cuando se escriba el primer comando Tauri que mapee esos
mismos errores.

**Verificación:**
```powershell
cargo fmt --check
cargo build --verbose
cargo test --verbose
cargo test --features dev-auth
cargo clippy --all-targets -- -D warnings
cargo check --no-default-features   # confirma que el núcleo solo compila sin ratatui/crossterm
```
Manual: `cargo run --release` y confirmar que login/menú/comandos/`--tui-clasica` se comportan
exactamente igual que antes.

## Estructura para `desktop/` (ya generada)

Ajuste respecto a la versión original de este plan: en vez de forzar `src-tauri/` y `frontend/`
como carpetas hermanas, se usó el layout estándar de Tauri (más simple, es lo que generan sus
propias herramientas de scaffolding sin pelear contra ellas) — `desktop/` es la raíz del proyecto
frontend (Vite) y `src-tauri/` vive adentro:

```
control_acceso/
  Cargo.toml                # sin [workspace] — sigue siendo un solo paquete
  src/  ...                 # sin cambios de fondo más allá de Fase 0
  desktop/                   # raíz del frontend (Vite + React + TS), generada con
                              # `npm create vite@latest desktop -- --template react-ts`
    package.json              # incluye ag-grid-community + ag-grid-react
    vite.config.ts
    tsconfig.json
    src/
      api.ts                 # (por escribir) invoke() tipado, una función por comando Tauri
      pantallas/             # (por escribir)
      componentes/           # (por escribir) tabla AG Grid compartida, formulario compartido
    src-tauri/                 # raíz de Cargo INDEPENDIENTE (ver por qué abajo), generada con
                                # `cargo tauri init --ci ...`
      Cargo.toml
      tauri.conf.json
      capabilities/default.json
      src/
        main.rs
        lib.rs                # (por escribir) estado GuiState, comandos #[tauri::command]
```

Generado y verificado en la sesión de setup (2026-08-27): `cargo install tauri-cli` (para
`cargo tauri dev`/`build`), scaffold de Vite, `npm install ag-grid-community ag-grid-react`,
`cargo tauri init`. Ajustes manuales al `Cargo.toml` de `src-tauri` generado: nombre de paquete
`control-acceso-desktop` (el scaffold por defecto lo llama `app`), `edition = "2024"` para
alinearlo con la raíz, `identifier` en `tauri.conf.json` cambiado de `com.tauri.dev` (placeholder)
a `com.dqm27.controlaccesobrisas.desktop` (coherente con `Package/Identity` del MSIX en
`packaging/msix/AppxManifest.xml`). Se dejó `tauri-plugin-log` (viene por defecto en el scaffold,
sólo activo en debug) pero **no** se agregó todavía `tauri-plugin-dialog` — eso se suma recién
cuando se construya exportar historial/respaldos (Grupo 4-5), no antes.

**Por qué `desktop/src-tauri` NO debe unirse a un `[workspace]` con la raíz:** Cargo unifica
features sobre todo el grafo de resolución de un mismo workspace. Si ambos paquetes compartieran
workspace, la raíz pidiendo `default = ["terminal-ui"]` para su propio binario podría forzar esa
misma feature en la dependencia que usa Tauri, anulando en silencio el aislamiento buscado. Dos
raíces de Cargo separadas (dos `Cargo.lock`, dos `target/`) lo evitan por construcción. Costo
aceptado: tiempo de compilación y disco duplicados (no de lógica) — y hay que repetir a mano el
bloque `[profile.release]` (`opt-level=3`, `lto="thin"`, `codegen-units=1`) en
`desktop/src-tauri/Cargo.toml`, porque no se hereda entre raíces independientes.

```toml
# desktop/src-tauri/Cargo.toml
[package]
name = "control-acceso-desktop"
edition = "2024"

[dependencies]
control_acceso = { path = "../..", default-features = false, features = ["serde"] }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```
El feature `serde` es nuevo, opcional, en el `Cargo.toml` de la raíz (`serde = { version = "1",
features = ["derive"], optional = true }`) — no afecta al binario de consola porque no es un
feature por defecto.

## Frontend: React + TypeScript + AG Grid (decisión del usuario)

- Scaffolding oficial: `create-tauri-app` trae plantilla `react-ts` (Vite + React + TypeScript).
- **Introduce Node.js/npm (o pnpm)** como herramienta nueva del proyecto — a diferencia del resto
  del repo, que es 100% Cargo. Trae su propio `package.json`/`package-lock.json` dentro de
  `desktop/frontend/`, con el mismo criterio de versiones fijas que ya aplica `Cargo.lock`.
- **AG Grid Community** (MIT, gratis, uso en producción sin licencia) cubre lo que estas tablas
  necesitan: orden, filtro, paginación, edición inline, celdas custom. Las funciones de
  **AG Grid Enterprise** (agrupar filas, pivotar, master/detail, modelo server-side, exportar)
  requieren licencia paga — no parecen necesarias hoy; evaluar el costo sólo si en el camino se
  quiere una función Enterprise específica.
- `tauri.conf.json` apunta `build.beforeDevCommand`/`beforeBuildCommand` a los scripts de Vite y
  `frontendDist` a `frontend/dist` — patrón estándar de cualquier template de Tauri con framework.
- El frontend sigue sin tocar SQLite ni lógica de negocio directamente — todo pasa por `invoke()`
  hacia comandos Rust finos que llaman a `AppCore`; React/AG Grid sólo deciden cómo se ve y captura
  eso.

## Superficie de comandos Tauri

Cada `#[tauri::command]` es una función fina que llama a un método ya existente de `AppCore`
(login, CRUD de contratistas/empresas/usuarios, ingreso/salida, activos, historial+export,
respaldos) — la API completa ya existe en `src/application/*.rs`.

**Argon2 sin canal `mpsc`** (a diferencia de la TUI, verificado en `src/tui/app/auth_jobs.rs`): la
TUI necesita el canal porque su loop de `crossterm` es un único hilo bloqueante. Tauri despacha
cada comando síncrono a su propio pool de hilos: el frontend ya espera una `Promise`, así que un
comando bloqueante no congela la ventana. El patrón de dos pasos que ya existe en
`autenticacion_service` (`buscar_candidato_autenticacion`, rápido, con `&AppCore` →
`verificar_candidato`, función libre sin `AppCore`) hay que preservarlo igual: soltar el
`Mutex<AppCore>` antes de calcular el hash, para no bloquear otros comandos mientras tanto.

**Hallazgo de seguridad:** `src/models/usuario.rs` tiene `password_hash: String` en claro. Nunca
serializar `Usuario` directo al webview — construir un DTO explícito (`UsuarioVista { id, cedula,
nombre, rol, activo }`, sin el hash) para cualquier comando que hoy devuelva `Usuario`.

`tauri-plugin-dialog` es el único plugin genuinamente necesario (exportar historial a XLSX,
exportar/restaurar respaldos requieren elegir una ruta real).

## Estado y sesión

```rust
struct GuiState {
    core: std::sync::Mutex<AppCore>,
    sesion: std::sync::Mutex<Option<UsuarioSesion>>,
}
```
Dos mutexes separados porque ningún flujo necesita actualizar sesión y base de datos
atómicamente. No hace falta token/cookie de sesión: es una ventana, un usuario a la vez, y el
candado de instancia ya garantiza un solo proceso por base de datos. La autorización real sigue
viviendo en `AppCore`/`domain::autorizacion`, evaluada contra SQLite en cada llamada — el frontend
puede ocultar botones por rol para UX, pero eso es cosmético, no la garantía real.

## Cómo avanzar cuando se decida empezar (incremental, no calco)

Orden fijado: **login primero, después las tablas.** Agrupación funcional de referencia (no es un
plan de "portar 13 archivos", es por dónde conviene empezar y qué depende de qué):

1. **Arranque**: login (patrón de dos pasos de Argon2) + contratistas (sólo búsqueda/tabla).
   Objetivo real: probar que el feature flag, los DTOs, la latencia de IPC y el empaquetado
   funcionan en la práctica.
2. **CRUD de catálogo**: contratistas (alta/edición completa), empresas, usuarios, cambiar
   contraseña propia.
3. **Operación diaria**: nuevo ingreso (matriz PRAIND/gafete de
   `docs/radiografia-dominio-comandos.md`), salida rápida, activos.
4. **Consulta pesada**: historial (paginado + export XLSX vía `tauri-plugin-dialog`), auditoría de
   contratistas.
5. **Administración sensible**: respaldos (crear/listar/validar/exportar es simple; **restaurar**
   es el punto delicado — hoy la consola cierra/reabre la conexión, en una app de ventana
   probablemente implique relanzar el proceso completo; diseñarlo al llegar ahí).

Aclaraciones de mapeo: el menú principal de la TUI no se migra como "pantalla" (su navegación la
absorbe el shell/barra lateral de la GUI); la pantalla "Configuración" de la TUI clásica es, por
dentro, la de Respaldos (grupo 5).

## `comandos` / TUI clásica: tres interfaces conviviendo por diseño

No hay congelamiento ni fecha de reemplazo. La lógica de negocio se escribe una sola vez
(`application/services/domain/database/models`), y cada interfaz decide, a su propio ritmo, si y
cuándo construye la presentación de una funcionalidad nueva. `comandos` sigue siendo hoy la
interfaz de uso diario y puede seguir recibiendo trabajo normalmente en paralelo a la GUI.

**Nota no bloqueante:** `docs/plan-gafetes.md` (aprobado 2026-08-22, sin implementar) está escrito
contra `src/tui/gafetes/...` (TUI clásica). Sus secciones de núcleo (schema, modelo, servicio,
validación en registrar ingreso) siguen siendo 100% válidas; sólo la sección de pantalla necesita
decidirse contra cuál(es) interfaz(es) construirla realmente antes de retomarlo.

## Empaquetado

`packaging/alacritty/` y `packaging/msix/` quedan exactamente como están, exclusivos del binario
de consola. La GUI usa su propio camino: `cargo install tauri-cli` + `npm install` en
`desktop/frontend` → `cargo tauri build` (corre el build de Vite y genera el instalador NSIS/MSI)
→ íconos reutilizando `assets/icon-master.png` vía `cargo tauri icon`. Este paso sí requiere
Node.js instalado en la máquina de build. CI no se toca hasta que el primer tramo esté estable.

## Riesgos y mitigación

| Riesgo | Mitigación |
|---|---|
| Unificación de features de Cargo si `desktop/src-tauri` comparte workspace | Raíz de Cargo independiente |
| Mapeo de errores duplicado una tercera vez | Extraer `error_messages` a módulo neutral en Fase 0 |
| `Usuario.password_hash` llegando al webview | DTO explícito (`UsuarioVista`), nunca `Serialize` directo sobre `Usuario` |
| Candado de instancia única (no correr consola+GUI a la vez contra la misma base) | `CONTROL_ACCESO_DB` apuntando la GUI en desarrollo a una base de pruebas separada |
| Comando de login bloqueando otros comandos durante Argon2 | Patrón de dos pasos: soltar el `Mutex<AppCore>` antes de calcular el hash |
| Tamaño real de la capa de presentación nueva (79% del código actual es interfaz) | Expectativa a sostener, no problema a resolver |
| Restaurar respaldo + ciclo de vida del proceso en app de ventana | Diseñarlo al llegar al grupo de Administración sensible |
| Node.js/npm es una cadena de herramientas nueva (hoy 100% Cargo) | Aceptado junto con React+TS+AG Grid; `package-lock.json` con la misma disciplina que `Cargo.lock` |
| AG Grid Enterprise requiere licencia paga | Community alcanza hoy; evaluar costo sólo si se necesita una función Enterprise específica |

## Verificación end-to-end (cuando se empiece a construir)

- **Fase 0**: comandos de arriba en verde, sin tocar `src/tui/` ni `src/comandos/` salvo el import
  mecánico de `error_messages`.
- **Primer tramo (login + contratistas)**: `cargo check` y `cargo tree -i ratatui` dentro de
  `desktop/src-tauri` (debe fallar/no encontrar el paquete), `cargo tauri dev`; a mano: login
  correcto e incorrecto, mover/redimensionar la ventana durante el cálculo de Argon2 (no debe
  congelarse), comparar una búsqueda de contratista contra el mismo filtro en `comandos`/
  `--tui-clasica` (con la GUI cerrada, por el candado de instancia).
- **Cada tramo siguiente**: `cargo check`/`cargo clippy` en `desktop/src-tauri`, recorrido manual
  de cada operación, y verificación cruzada: lo que la GUI escribe en SQLite debe verse
  correctamente desde la consola. La suite de ~375 tests existentes debe seguir en verde sin
  necesitar modificarse.

## Referencias

- `Cargo.toml`, `src/lib.rs` — Fase 0 (feature flag)
- `src/tui/app/error_messages.rs` — extracción a módulo neutral
- `src/application/mod.rs` y submódulos — API que consumen los comandos Tauri
- `src/services/autenticacion_service.rs`, `src/tui/app/auth_jobs.rs` — patrón de Argon2 en dos pasos
- `src/models/usuario.rs` — motivo del DTO obligatorio
- `docs/radiografia-dominio-comandos.md` — reglas de negocio (PRAIND/gafete) que la GUI consume sin reimplementar
- `docs/plan-gafetes.md` — pendiente, revisar contra qué interfaz antes de retomarlo
