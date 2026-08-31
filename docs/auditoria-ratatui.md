# Auditoría del uso de ratatui en control_acceso

Fecha: 2026-08-23
Rama en la que se hizo la auditoría: `tui-comandos` (working tree limpio, `origin/tui-comandos` al día).
Propósito: evaluar el estado actual de la TUI como base para diseñar un nuevo lenguaje visual de "mutaciones" (transiciones, foco explícito, layout responsivo por breakpoints, patrones reutilizables). No se modificó código para este reporte.

Nota posterior: este reporte es histórico. La pantalla puente para elegir entre TUI
clásica y `--comandos` fue eliminada; tras el login la TUI clásica entra directo al
menú, y el cambio a comandos se hace desde el Menú Principal.

## 0. Aclaración sobre `examples/brisas_cli/*_v2.rs`

Esos archivos (`empresas_v2.rs`, `usuarios_v2.rs`, `historial_v2.rs`, `configuracion_inicial_v2.rs`, `login_v2.rs`, `menu_v2.rs`, `activos_v2.rs`, `contratistas_v2.rs`, `ingreso_v2.rs`) existieron entre los commits `1250a3e` → `e0a5f7f` ("Crea piloto visual reutilizable de BRISAS CLI" → "Completa los pilotos visuales _v2 de las nueve pantallas de BRISAS CLI", 16 ago 2026), pero **fueron eliminados por completo** en el commit `4d2eb20` ("En proceso de actualizacion", 18 ago 2026), junto con `examples/brisas_cli.rs`, `app.rs`, `terminal.rs`, `login.rs`, `menu.rs`, `ingreso.rs`, `quick_exit.rs` (14 761 líneas borradas de golpe). Ese mismo commit ya tocaba `src/tui/menu_principal` y `src/tui/ui_kit/theme.rs`.

Lo que ocurrió: el piloto `_v2` se **absorbió hacia adentro**. El propio comentario en `src/tui/ui_kit/mod.rs` lo dice:

> "El piloto `brisas_cli` fue su primer consumidor; las 9 pantallas de producción ya usan el shell visual (`ScreenShell`/`Theme`) y, desde la unificación de atajos, también la convención de teclado (`standard_command`)."

Es decir: el plan de integración UI v2 ya se ejecutó, no copiando archivos desde `examples/`, sino migrando `ScreenShell`, `Theme`, `TextInput`, etc. directamente a `src/tui/ui_kit/` y adoptándolos en las 9 pantallas de producción. Los `_v2` cumplieron su función de piloto desechable y se borraron. **Hoy no existe carpeta `examples/` en el repo.** Son recuperables solo vía `git show e0a5f7f:examples/brisas_cli/usuarios_v2.rs`, si hiciera falta consultarlos.

## 1. Dependencias (Cargo.toml)

```toml
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"] }
crossterm = "0.29.0"
tui-input = { version = "0.15.4", default-features = false, features = ["crossterm"] }
```

- **ratatui 0.30.2**, sin default-features, solo backend `crossterm`. Versión razonablemente reciente.
- **Backend**: crossterm exclusivamente. No hay termion. `ratatui::backend::TestBackend` se usa solo en tests (`ui_kit/shell.rs`).
- **`tui-input` 0.15.4**: crate de terceros para edición de línea con cursor, usada tanto en `ui_kit/text_input.rs` como directamente en `comandos/mod.rs`. No es parte de ratatui.
- **`query-parser` 0.2.0**: no es de rendering, es para el lenguaje `clave:valor` de búsquedas (`ui_kit/query_lang.rs`).
- **No hay ninguna crate de animación/easing** (nada tipo `tachyonfx`, `interpolation`, `keyframe`). Cero dependencias de motion design.
- No hay `tui-realm`, `throbber-widgets-tui`, ni widgets de terceros — todo lo "extra" está hecho a mano en `ui_kit`.

## 2. Estructura de `src/tui/`

```
src/tui/mod.rs              — declara los módulos de pantalla + ui_kit
src/tui/app.rs (875 líneas) — struct App (estado raíz), enum Vista, loop principal (delegado a app/)
src/tui/app/actions.rs + actions/{accesos,catalogos,admin}.rs — despachadores de acciones por dominio
src/tui/app/auth_jobs.rs    — hilos Argon2 (login, alta ROOT, crear usuario, cambio de password)
src/tui/app/error_messages.rs
src/tui/terminal.rs         — TerminalGuard (raw mode/alt screen) + run()/run_sin_core()
src/tui/preferences.rs      — persistencia de tema elegido
src/tui/visual_tests.rs     — snapshots con insta (cfg(test))
```

Cada pantalla sigue el mismo patrón de 3-4 archivos: `{pantalla}/{mod.rs, state.rs, render.rs, tests.rs}` (algunas con extras: `contratistas/{form.rs, query.rs}`, `usuarios/{form.rs, password.rs}`, `historial/filtros.rs`).

Pantallas: `login`, `configuracion_inicial`, `menu_principal`, `activos`, `historial`, `contratistas`, `empresas`, `usuarios`, `cambio_password`, `auditoria`, `configuracion` (respaldos), `nuevo_ingreso`, `salida_rapida`.

`src/tui/ui_kit/` (el "design system" interno):

| archivo | rol |
|---|---|
| `theme.rs` | 3 presets de color (`Classic`/`Brisas`/`Negro`) + helpers de `Style` semánticos |
| `shell.rs` | `ScreenShell` — marco común de cabecera/pestañas/status/comandos |
| `layout.rs` | `master_detail_areas()` responsivo por breakpoint, `render_terminal_too_small` |
| `fields.rs` | `render_form_field`, `render_choice_field`, `empty_state`, `panel_vacio` |
| `text_input.rs` | wrapper sobre `tui-input` con foco/cursor (`TextInputFocus`) |
| `keyboard.rs` | `StandardCommand`/`standard_command()` — convención de atajos (F1 ayuda, F2 salida rápida, F7 tema, etc.) |
| `seleccion.rs` | `mover_seleccion()`, `MARCADOR_SELECCION` (▶) |
| `debounce.rs` | temporizador de "esperar N ms sin tecla nueva" para búsquedas |
| `query_lang.rs` | parser del lenguaje `clave:valor` |
| `details.rs` | `detail_line` (par etiqueta/valor) |

**No hay ningún `impl Widget for ...` ni `impl StatefulWidget for ...` en todo el repo** — cero widgets custom en el sentido estricto de ratatui. Todo es composición de `Paragraph`/`Table`/`Tabs`/`Block` estilizados vía funciones helper.

## 3. `examples/brisas_cli/*_v2.rs`

No existen en el árbol actual (ver sección 0). Su contenido ya fue destilado hacia `src/tui/ui_kit/`; no hay una versión "más avanzada" paralela esperando portarse.

## 4. `src/comandos/` y el flag `--comandos`

```
src/comandos/mod.rs (777 líneas)  — loop propio, TerminalGuard propio, dispatch de teclas
src/comandos/estado.rs (203)      — AppState { input, fase, contexto, formulario, sugerencias, feedback, salir }
src/comandos/parser.rs (436)      — parsea texto libre → Entrada/Comando
src/comandos/resolver.rs (386)    — Entrada → ContextState (consulta a AppCore)
src/comandos/render.rs (1047)     — dibuja según ContextState
src/comandos/formulario.rs (673)  — FormularioContratista (alta/edición embebida en esta UI)
```

**Activación**: `src/main.rs` decide entre `--tui-clasica`, `--comandos` o la
preferencia guardada. Desde la TUI clásica se cambia con "Modo comandos" en el Menú
Principal, que guarda la preferencia y relanza el proceso.

**Integración con lógica de negocio**: completa. `comandos::run(core: AppCore, sesion_inicial: Option<UsuarioSesion>)` consume el mismo `AppCore` que la TUI clásica — mismas queries, mismo `services::autenticacion_service` con hilo Argon2 en background. No hay lógica de negocio duplicada ni divergente.

**Lo que le falta visualmente**: no usa `ScreenShell`, `Theme` ni ningún helper de `ui_kit` — render totalmente aislado en `comandos/render.rs`, con su propio `TerminalGuard` duplicado (líneas 70-91 de `mod.rs`, literalmente el mismo código que `tui::terminal::TerminalGuard`, con un comentario que lo admite: *"el de `tui::terminal` es privado y la restricción del proyecto es no tocar la TUI clásica, así que se replica lo mínimo"*). Deuda de duplicación arquitectónica ya reconocida en el propio código.

## 5. Patrones de renderizado actuales

**Event loop**: estándar de ratatui. En `tui::app::run_internal` es **redraw-on-demand** (solo redibuja si algo cambió: tecla, resize, hilo terminado, debounce vencido, reloj de cabecera). En `src/comandos/mod.rs` el loop es más simple y redibuja cada vuelta (poll fijo de 80ms) — divergencia de patrón entre las dos interfaces.

**State machine**:
- TUI clásica: `enum Vista` — máquina de estados explícita por pantalla completa. Cada `state.rs` tiene sus propios sub-enums (`ModoFormulario`, `CampoFormulario`, `Subfase`, etc.).
- `--comandos`: sin enum de pantallas — el `ContextState` se deriva del input en cada cambio (input → parser → resolver → contexto). Conceptualmente más cercano a "estado como función pura", pero acoplado 1:1 al parseo de texto.

**Animación/transición/easing**: no existe nada. Los usos de `Instant`/`Duration`/`tick` (21 archivos) son: reloj de cabecera, debounce de búsqueda (binario, no interpola), expiración de feedback transitorio (aparece/desaparece de golpe, `DURACION_FEEDBACK = 4s`), timeouts de hilos Argon2. Ningún interpolador, ningún easing, ningún widget que cambie de tamaño/posición gradualmente. Todo cambio de estado se refleja instantáneamente en el siguiente `draw()`.

**Widgets**: 100% stock de ratatui — `Paragraph` (65 usos), `Table` (9+), `Tabs` (1, en `ScreenShell`), `Block` (base de todos los paneles). No hay `List` (las listas se hacen a mano con `Table`/`Paragraph` + estilo de fila seleccionada), ni `Gauge`, `Chart`, `Sparkline`, `BarChart`, `Scrollbar`.

## 6. Manejo de foco / navegación

Existe, pero es manual y booleano, no centralizado:

- `ui_kit/shell.rs::panel(title, theme, focused: bool)` / `auxiliary_panel(...)` — colorean borde/título según un `bool` que cada pantalla calcula comparando su propio enum de campo activo.
- `ui_kit/text_input.rs::TextInputFocus { focused, cursor_visible }` — controla si se muestra el cursor.
- Cada pantalla mantiene su propio enum de "campo actual" (`CampoFormulario`, `CampoUsuario`, `Campo` en `comandos/formulario.rs`) con su propio `mover_campo(delta: isize) -> bool`. No hay trait común `Focusable` ni árbol de foco — mismo patrón reimplementado archivo por archivo.
- Navegación de nivel superior es por `Vista`. En vistas maestro-detalle tampoco hay foco federado entre master y detail — uno de los dos es "el activo" por convención de pantalla, no por estado de foco explícito.

## 7. Manejo de layout

Mixto, con una pieza responsiva real:

- La mayoría son layouts estáticos: `Layout::vertical([...]).split(area)` con constraints fijos inline en cada `render.rs`.
- **Una** utilidad responsiva reutilizada: `ui_kit/layout.rs::master_detail_areas()`, breakpoint fijo (`MASTER_DETAIL_BREAKPOINT: u16 = 100`), con tests unitarios que verifican el cambio de orientación.
- `ScreenShell::render_header` tiene una rama responsiva simple: `if area.width < 90 { 2 columnas } else { 3 columnas }` — un solo breakpoint hardcodeado, no un sistema con nombres reutilizable.
- No existe una noción explícita `wide/normal/compact` como enum compartido — cada sitio define su propio umbral numérico ad-hoc (100, 90...), sin relación declarada entre ellos.
- `MIN_TERMINAL_WIDTH`/`MIN_TERMINAL_HEIGHT` (60×22) sí es constante compartida, con `render_terminal_too_small()` como guardia común.

## 8. Identidad visual

Consistente y deliberada, no defaults de ratatui:

- 3 presets de tema completos (`Classic`, `Brisas`, `Negro`) con paleta semántica (`background/text/muted/accent/success/warning/danger/border/selection_foreground/selection_background`) — nunca colores sueltos.
- Ciclo de tema con F7 (`ThemePreset::next()`), persistido (`tui/preferences.rs`).
- Convención de atajos unificada (`ui_kit/keyboard.rs::StandardCommand`) — F1 ayuda, F2 salida rápida, F7 tema, Ctrl+←/→ pestañas — aplicada vía `ScreenShell.commands`.
- Marcador de selección no cromático (▶, `MARCADOR_SELECCION`) para no depender solo del color.
- Bordes siempre `BorderType::Plain` + `Borders::ALL` vía `styled_panel()`, foco indicado por color de borde + glifo ▶ en el título — consistente en todo el código.
- `selected_tab()` como variante no cromática para pestañas, con comentario explícito sobre terminales que no distinguen bien los colores del preset.

Detalle relevante: el campo `navegacion_pestanas: bool` en `Theme` decide que **solo** el preset `Negro` navega por pestañas tras iniciar sesión — el tema hoy también controla estructura de navegación, no solo paleta. Esto es un acoplamiento a tener en cuenta para el nuevo lenguaje visual.

## Brecha hacia el lenguaje visual de mutaciones

1. **Estado como fuente de verdad mutable con transiciones**
   Hoy: `App` (TUI clásica) tiene ~13+ structs de estado por pantalla, mutados directamente por `actions/*.rs`. `AppState` (comandos) se re-deriva del input — más cerca del ideal, pero acoplado 1:1 al parseo de texto. Falta un modelo de estado unificado entre ambas interfaces (hoy son dos árboles independientes sin struct ni trait compartido) y, sobre todo, un concepto de estado con historial/interpolación — hoy toda mutación es atómica e instantánea, nunca "a mitad de camino" entre A y B.

2. **Animación interrumpible**
   Hoy: cero. El `Debounce` es lo más cercano a temporización y solo dispara una acción binaria, no interpola ni es cancelable a mitad de una transición. Falta: reloj de frame (delta-time, no solo timestamps), tipo de valor animable, función de easing, y la semántica de interrumpir una animación en curso y arrancarla desde el punto actual. El loop actual es redraw-on-demand; para animación hay que invertir esa lógica a "redibujar mientras haya una transición viva" — cambio de fondo en `run_internal`, no aditivo.

3. **Sistema de foco explícito**
   Hoy: booleano pasado a mano widget por widget, un enum de "campo actual" por pantalla sin trait común, sin árbol de foco ni federación entre paneles hermanos. Falta un trait/struct `Focus` compartido (id de nodo, navegación genérica por Tab/Shift+Tab) en vez de match manual de flechas por pantalla.

4. **Geometría/layout responsivo por breakpoints**
   Hoy: 1 breakpoint reutilizado (100 cols) + 1 ad-hoc (90 cols) + un piso absoluto (60×22). No hay enum `Breakpoint::{Compact, Normal, Wide}` central. Falta ese sistema nombrado para que cada pantalla declare su layout como función del breakpoint, no de un literal propio.

5. **Reutilización de patrones (SELECTOR, FORMULARIO, OPERACIÓN)**
   Hoy hay fragmentos reutilizables (`render_form_field`, `render_choice_field`, `master_detail_areas`, `mover_seleccion`) pero no un patrón compuesto y nombrado. **Tres implementaciones independientes** del mismo concepto de formulario: `tui/contratistas/state.rs`, `tui/usuarios/state.rs`, `comandos/formulario.rs`. Falta elevar SELECTOR y FORMULARIO a componentes parametrizados reales donde cada pantalla concreta sea solo datos + labels. OPERACIÓN tiene indicios (`Subfase::Resumen` en comandos, tarjetas de confirmación) pero tampoco está generalizada ni compartida entre interfaces.

## Resumen ejecutivo

La base es sólida para una TUI convencional: theming consistente, layout responsivo básico, separación limpia de state/render/tests por pantalla, sin acoplamientos raros a I/O dentro del render. Pero es una base **estática**: cero infraestructura de tiempo continuo (animación), foco centralizado, o breakpoints declarativos.

Construir el lenguaje visual de mutaciones no es una extensión incremental de `ui_kit` — requiere una capa nueva por debajo (reloj de frame, tipo de valor animable, árbol de foco, breakpoints nombrados) sobre la cual SELECTOR/FORMULARIO/OPERACIÓN se reimplementarían como componentes genéricos, en vez de seguir copiándose por pantalla como hoy.

También hay que decidir qué pasa con la duplicación ya existente entre la TUI clásica y `--comandos` (dos `TerminalGuard`, dos árboles de estado, dos loops) antes de que el nuevo lenguaje visual tenga que vivir en ambos a la vez.
