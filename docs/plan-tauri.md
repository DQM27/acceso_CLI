# Plan Tauri

## Objetivo

Agregar una interfaz grafica de escritorio sin duplicar la logica de negocio ni reemplazar las interfaces existentes de consola.

La GUI debe ser otra entrada al mismo nucleo:

```text
CLI / TUI clasica / Tauri
        |
        v
AppCore
        |
        v
services / queries / repositories / SQLite
```

## Decision principal

No crear otro repo. El proyecto Tauri puede vivir en este mismo repositorio y depender del nucleo Rust existente.

El nucleo actual ya esta en `src/lib.rs`, por lo que Tauri puede consumir el crate `control_acceso` como dependencia local.

## Estructura recomendada

Mantener el binario actual para consola y agregar la app Tauri como paquete aparte dentro del mismo repo:

```text
control_acceso/
  Cargo.toml
  src/
    lib.rs
    main.rs                  # CLI / TUI clasica
    application/
    services/
    database/
    domain/
    models/
    comandos/
    tui/

  desktop/
    src-tauri/
      Cargo.toml             # binario GUI
      tauri.conf.json
      src/
        main.rs
    frontend/
      package.json
      src/
```

En `desktop/src-tauri/Cargo.toml`:

```toml
[dependencies]
control_acceso = { path = "../.." }
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Con esto, Tauri usa el mismo `AppCore`, pero su binario se compila desde `desktop/src-tauri`, no desde el `src/main.rs` actual.

## Separacion de binarios

Hay dos binarios, pero no dos logicas:

```text
target/release/control_acceso.exe           # consola: CLI / TUI clasica
desktop/src-tauri/target/release/...exe     # GUI Tauri
```

Ambos dependen del mismo `src/lib.rs`.

Cambios de negocio, por ejemplo soporte para visitas, se hacen una vez en:

```text
src/application/
src/services/
src/database/
src/domain/
src/models/
```

Luego cada interfaz solo implementa su forma de mostrar y capturar datos.

## Como evita Tauri cargar la CLI

Tauri no depende del binario `src/main.rs`; depende de la biblioteca `src/lib.rs`.

Cargo compila:

- `src/lib.rs` cuando otro paquete depende de `control_acceso`.
- `src/main.rs` solo cuando se compila el binario `control_acceso`.

Por eso, si `desktop/src-tauri` depende de `control_acceso = { path = "../.." }`, Tauri reutiliza el nucleo pero no ejecuta ni enlaza el flujo de arranque de consola definido en `src/main.rs`.

Punto importante: hoy `src/lib.rs` exporta tambien `comandos` y `tui`. Si el crate Tauri depende de `control_acceso`, Cargo puede compilar esos modulos porque forman parte de la biblioteca publica. Para evitar que el binario GUI arrastre dependencias de terminal, conviene esconderlas detras de una feature.

## Features recomendadas

En el `Cargo.toml` principal:

```toml
[features]
default = ["terminal-ui"]
terminal-ui = []
dev-auth = []
```

En `src/lib.rs`:

```rust
pub mod application;
pub mod database;
pub mod domain;
pub mod historial;
pub mod instancia;
pub mod interfaz_preferida;
pub mod models;
pub mod services;
pub mod texto;
pub mod tiempo;

#[cfg(feature = "terminal-ui")]
pub mod comandos;

#[cfg(feature = "terminal-ui")]
pub mod tui;
```

El binario de consola usa las features por defecto, asi que conserva `comandos` y `tui`.

Tauri depende del nucleo sin features por defecto:

```toml
control_acceso = { path = "../..", default-features = false }
```

Asi el binario GUI no compila `ratatui`, `crossterm`, `tui-input`, ni los modulos de consola.

## Como compilar

Consola:

```powershell
cargo build --release
```

GUI Tauri:

```powershell
cd desktop
npm run tauri build
```

O desde `desktop/src-tauri`:

```powershell
cargo build --release
```

si solo se quiere validar el lado Rust de Tauri.

## Estado compartido en Tauri

Mantener `AppCore` sin async por ahora. Tauri puede manejarlo como estado bloqueante:

```rust
struct GuiState {
    core: std::sync::Mutex<AppCore>,
}
```

Las operaciones actuales son compatibles con este modelo:

- login
- busquedas paginadas
- consultas de historial
- crear y editar catalogos
- registrar ingresos y salidas
- respaldos y exportacion

No meter Tokio como requisito de arquitectura. Si luego hay operaciones largas, usar tareas bloqueantes puntuales sin convertir el nucleo a async.

## Primer prototipo

1. Crear `desktop/` con Tauri.
2. Agregar dependencia local a `control_acceso`.
3. Exponer comandos Tauri minimos:
   - `requiere_configuracion_inicial`
   - `login`
   - `buscar_contratistas`
4. Crear una pantalla con login y tabla de contratistas.
5. Medir empaquetado, tamano de binario y experiencia de doble click en Windows.

Si el prototipo funciona, migrar pantalla por pantalla.
