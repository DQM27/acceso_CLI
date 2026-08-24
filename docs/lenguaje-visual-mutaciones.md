# Lenguaje visual de mutaciones — interfaz de comandos

> **Nota de nomenclatura:** el flag `--comandos` ya no existe — la interfaz
> de comandos es ahora la ruta por defecto al arrancar la aplicación (sin
> flags). La TUI clásica quedó detrás de `--tui-clasica`. Donde este
> documento diga "`--comandos`" léase "la interfaz de comandos" (el nombre
> del módulo `src/comandos/` no cambió). Ver DEC-017.

Este documento es la referencia técnica y visual oficial de la nueva capa
interactiva de comandos. Complementa a [`docs/auditoria-ratatui.md`](auditoria-ratatui.md),
que es la fotografía del estado inicial (qué existía) — este documento es
qué vamos a construir a partir de ahí, y se actualiza a medida que avanza el
trabajo.

Rama de trabajo: `lenguaje-visual-mutaciones` (creada desde `tui-comandos`,
que queda intacta).

## 1. Visión

La palabra central del diseño es **mutación**. No queremos:

```text
pantalla A desaparece → pantalla B aparece
```

Queremos:

```text
estado A → conserva lo que sigue siendo válido → transforma elementos existentes → estado B
```

La interfaz debe sentirse como una superficie viva y continua: elementos que
se desplazan, se expanden, se contraen, cambian de jerarquía, se convierten
en otros elementos, aparecen sólo cuando son necesarios, desaparecen cuando
dejan de ser útiles, y pueden revertir su transición si el usuario cambia de
intención.

La animación no es el objetivo. El objetivo es continuidad visual + fluidez
+ respuesta inmediata + practicidad operativa.

> La interfaz se transforma para explicar lo que está ocurriendo; nunca
> obliga al usuario a esperar para admirar la transformación.

## 2. Principios no negociables

1. La practicidad tiene prioridad sobre la estética.
2. El teclado responde inmediatamente — nunca depende de un tick de animación.
3. Una animación comunica un cambio de estado; no existe como decoración.
4. Toda operación funciona perfectamente con animaciones desactivadas.
5. Una misma acción produce una misma respuesta visual en toda la interfaz.
6. `Enter`, `Esc`, navegación, selección, confirmación, error y éxito tienen
   significado consistente en todos lados.
7. El color comunica estado o jerarquía, nunca decoración arbitraria.
8. Antes de un borde, ventana, popup o tabla nueva: ¿realmente hace falta?
9. `AppCore` sigue siendo la única autoridad de reglas de negocio; la UI
   vuelve a validar todo con él, nunca decide por su cuenta.

## 3. Restricciones de alcance

- El trabajo nuevo vive exclusivamente en `src/comandos/` y submódulos
  nuevos propios de esta interfaz.
- `src/tui/` (la TUI clásica) no se toca: no se migra, no se refactoriza,
  no se unifica con `--comandos`, no comparte `TerminalGuard` ni motor de
  presentación.
- El dominio, los servicios, los repositorios y `AppCore` no se tocan y
  nunca conocen Ratatui, Crossterm ni conceptos de animación.
- La capa visual debe poder eliminarse mañana sin destruir el sistema:

```text
Dominio / Services / AppCore / SQLite
                    ↑
               src/comandos
                    ↑
          Presentation Engine
                    ↑
           Ratatui / Crossterm
```

## 4. Gramática visual (conceptos iniciales)

| Concepto | Rol |
|---|---|
| `Composer` | Punto principal de entrada de comandos. |
| `Surface` | Superficie contextual que nace cuando una operación necesita espacio. |
| `Field` | Campo editable. |
| `Selector` | Selección contextual temporal. |
| `Notice` | Éxito, advertencia, error o información. |
| `Transition` | Transformación entre estados. |
| `Focus` | Representación universal del elemento activo. |
| `Summary` | Revisión previa a persistencia. |

Patrones reutilizables descubiertos al agrupar escenarios (no diseñar veinte
sistemas — agrupar comportamientos repetidos):

```text
Buscar contratista / empresa / usuario  → BUSCAR / SELECCIONAR
Nuevo / Editar contratista, empresa...  → FORMULARIO
Ingreso / Salida / Guardar cambio       → OPERACIÓN
Guardar / acción crítica                → CONFIRMACIÓN
Éxito / Advertencia / Error             → FEEDBACK
```

Ninguno de estos patrones se comparte con `tui/contratistas`, `tui/usuarios`
ni la TUI clásica en general — viven y se reutilizan sólo dentro de
`--comandos`.

## 5. Gramática de teclado

```text
Enter → aceptar / entrar / continuar / confirmar
Esc   → cancelar / volver / cerrar contexto
↑ ↓   → navegación vertical
›     → foco / selección
✓     → éxito
!     → advertencia
×     → error / rechazo
```

No inventar excepciones gratuitamente por pantalla.

## 5.1 Gramática de comandos

Una línea de `--comandos` se descompone en 5 piezas, cada una con un rol
fijo:

```text
/comando            → líder explícito de acción (autocompletable vía /)
--letra | --palabra → modificador de acción sobre un resultado de búsqueda
clave:valor          → parámetro con valor (admite listas clave:a,b,c y
                       comillas para valores con espacios)
-clave:valor          → negación de un parámetro
texto libre           → sujeto de la búsqueda (acción implícita, sin comando)
```

**Búsqueda es la acción por defecto**: texto sin `/` inicial ya es la
consulta — es la acción más frecuente y no necesita comando propio.

**`/comando` (líder) y `--modificador` son gramáticas mutuamente
excluyentes por línea.** Si hay un `/comando` explícito, cualquier `--x`
que aparezca después se trata como texto libre (mismo criterio que ya
existe para una clave `clave:valor` no reconocida: se degrada a texto en
vez de aplicarse a medias o dar error). Nunca compiten dos intenciones en
la misma línea.

**`--modificador` sólo es válido en comandos "de ítem"** — los que actúan
sobre el resultado ya encontrado (`ingreso`, `salida`, `editar`). Los
comandos globales (`nuevo`, `activos`, `ayuda`, `cerrarsesion`) no tienen
sujeto sobre el cual aplicarse como sufijo, así que sólo existen como
`/comando` líder.

**Guion simple vs. doble guion — no es longitud, es significado.** El
guion simple (`-clave:valor`) queda reservado exclusivamente para negación
(ver §14, motor de query). El doble guion identifica un modificador de
acción, sea de una letra (`--e`) o palabra completa (`--editar`) — no se
distingue por cantidad de guiones según longitud (a diferencia de la
convención POSIX `-v`/`--verbose`), porque el guion simple ya está tomado
por la negación del motor de query y crearía una colisión real de parseo,
no sólo de estilo.

Dos rutas de sintaxis llegan al mismo destino interno (misma
`Entrada::Comando`) — no son dos comandos, es el mismo con dos formas de
invocarlo: `/editar Ana` y `Ana --editar` (o `Ana --e`) producen el mismo
resultado. Esto es deliberado (progressive disclosure): el operador nuevo
descubre por `/` con autocompletado; el operador con práctica comprime la
misma acción en una sola línea sin cambiar de modo ni activar ninguna
configuración.

Motor de query (`clave:valor`, listas, negación) documentado en §14
(Registro de decisiones) y en el análisis de `query_lang.rs`.

## 5.2 Enclavado de Surfaces

Un `Surface` (§4) que necesita su propia gramática de input **enclava** el
teclado: mientras está activa, el input deja de interpretarse con la
gramática de nivel superior (`/comando`, `--modificador`, búsqueda) y pasa
a la gramática propia de esa Surface. No es un mecanismo nuevo — es el
mismo que ya usa el formulario de `/nuevo` (`app.formulario.is_some()`
capturando el teclado en `operando.rs`), generalizado a cualquier Surface
futura.

**Caso guía: Historial** (aún no construido, primer consumidor real del
motor de query de §5.1/DEC-022):

```text
/h  Enter                                → enclava, abre la Surface de
                                            Historial
desde:10/12/2025 hasta:12/02/2026  Enter → aplica la consulta una vez,
                                            muestra resultados navegables
                                            (↑↓ para moverse, Enter entra
                                            al detalle de un resultado)
Esc                                      → vuelve a editar la MISMA
                                            consulta, sin perder lo ya
                                            escrito
Esc de nuevo (sin filtro activo)          → sale de la Surface, vuelve a
                                            Operando
```

**Por qué Enter-aplica en vez de filtrado en vivo** (a diferencia del resto
de `--comandos`, que sí recalcula en cada tecla): Historial consulta contra
el histórico completo de movimientos, no contra una lista corta como
"activos ahora" — repetir esa consulta en cada tecla es gasto real de
recursos, no sólo un matiz estético. Decisión explícita, no un descuido de
consistencia (principio 1: la practicidad tiene prioridad sobre la
estética).

**Esc nunca borra** — mismo comportamiento que ya existe en
`formulario_controller.rs::manejar_resumen_formulario`: `Esc` desde el
resumen del formulario vuelve a `Subfase::Editando` conservando todos los
campos, nunca los vacía. Historial reusa esa misma regla en vez de inventar
una tercera.

## 6. Arquitectura de interacción

```text
Terminal Event → Action → Update State → Transition/Animation State → Render
```

Render es puro respecto al estado: lee estado, animaciones, geometría y
tema; nunca muta selección, formulario, permisos, datos o navegación.

## 7. Event loop y scheduler (Fase 0 — implementada)

Estado real encontrado al auditar `src/comandos/mod.rs` antes del cambio:

- `terminal.draw()` se llamaba **sin condición** en cada vuelta del loop,
  incluso cuando el `poll(80ms)` no traía ningún evento — redibujaba el
  mismo frame ~12 veces por segundo en reposo, gastando CPU sin necesidad.
- La latencia de tecla en sí **no era el problema real**: como `draw()` se
  ejecutaba al inicio de cada vuelta, antes del `poll`, una tecla procesada
  se pintaba en la siguiente vuelta casi de inmediato, no atada a un tick
  fijo de 80ms.
- No había manejo explícito de `Event::Resize` (funcionaba por accidente,
  porque siempre redibujaba).
- No existía ningún tipo de debounce (a diferencia de `ui_kit/debounce.rs`
  en la TUI clásica): `recomputar` corre síncrono en cada tecla.

Cambio aplicado: **redraw-on-demand con espera dinámica**.

```text
Aplicación quieta       → poll bloquea ~1h (no hay nada que revisar)
Usuario pulsa tecla     → update inmediato → redibuja en la misma vuelta
Argon2 corriendo        → poll corto (30ms) para revisar el canal seguido
Feedback transitorio    → poll acotado al tiempo restante hasta que expire
Resize                  → se captura explícitamente y fuerza redibujo
```

Implementación en `src/comandos/mod.rs::run` + `proxima_espera`, y
`AppState::expirar_feedback`/`feedback_restante` en `estado.rs`. Sin
scheduler de animación todavía — este es el piso sobre el que se construye
el `PresentationEngine` en fases posteriores.

`recibir_autenticacion` y `expirar_feedback` ahora devuelven `bool` (si de
verdad cambiaron algo) para que el loop sólo marque redibujo cuando
corresponde.

Verificado: `cargo fmt --check`, `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings` y `cargo test` (suite completa)
en verde tras el cambio.

## 8. Motor de presentación (pendiente — Fase 4)

Capa nueva, exclusiva de `--comandos`, responsable de: saber si hay una
transición activa, calcular tiempo transcurrido, programar el próximo
frame, administrar animaciones y easing, conocer la calidad visual, permitir
interrupciones, medir rendimiento, priorizar input y evitar redraw
innecesario. No administra reglas de negocio, permisos, SQLite ni
autorización.

Animaciones basadas en tiempo, nunca en número de frames:

```text
progress = elapsed / duration
valor = interpolate(start, end, easing(progress))
```

Easing inicial: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`. Aparición/
movimiento a destino → `EaseOut`; desaparición → `EaseIn`.

Duraciones de referencia (no ley):

```text
Microinteracción   80–150 ms
Transición normal  150–250 ms
Transición grande  200–350 ms
```

Toda transición debe poder interrumpirse: si el usuario pulsa `Esc` mientras
algo se abre, no se espera a que termine — se invierte desde el punto
temporal actual.

## 9. Foco (pendiente — Fase 2)

`--comandos` tiene poca profundidad deliberadamente. Empezar simple, sin
árbol de foco general:

```rust
enum FocusTarget {
    Composer,
    FormField(FieldId),
    Selector(SelectorId),
    Summary,
}
```

Sólo complicarlo si aparece una necesidad real.

## 10. Breakpoints (pendiente — Fase 2)

Reemplazar los umbrales ad-hoc (90, 100, 60×22 dispersos) por una
abstracción explícita local a `--comandos`:

```rust
enum Breakpoint {
    Compact,
    Normal,
    Wide,
}
```

Los límites numéricos se determinan viendo las escenas reales, no se asumen
de antemano. Los componentes reciben el breakpoint y deciden composición.

## 11. Calidad visual

Primera implementación (no antes):

```rust
enum VisualQuality {
    Off,
    Normal,
}
```

`Off`: mismo estado funcional final, transición instantánea — todas las
funcionalidades deben funcionar igual. `Reduced`/`High`/`Auto` y adaptación
por rendimiento observado quedan para fase futura explícita.

## 12. Dependencias

Estado verificado en esta rama (`cargo tree -p crossterm`, `Cargo.toml`):

```text
ratatui 0.30.2 (default-features = false, features = ["crossterm"])
crossterm 0.29.0 — una sola versión en el árbol, sin duplicados
tui-input 0.15.4 — ya integrado, se reutiliza
insta 1.48.0 — ya integrado, se reutiliza para snapshots de --comandos
tui-big-text 0.8.8 — nueva (ver justificación abajo)
```

**`tui-big-text`** (nueva, §14.1): renderiza texto grande con glifos de
bloque (fuente `font8x8` vía la crate `font8x8`) directamente como widget de
Ratatui. Verificado antes de añadirla (`cargo tree -p ratatui`/`-p
crossterm`): no duplica ninguna versión mayor de ninguna de las dos. Se
prefirió sobre dibujar ASCII art a mano porque siete letras bien
proporcionadas a mano es fácil que salgan desalineadas o ilegibles en
terminales angostas; la crate ya resuelve el layout carácter por carácter y
respeta el `Style` (por lo tanto el fundido de `estilo_fundido` funciona
igual que con texto normal). Se usa en un único lugar (el título "Brisas
CLI"), con `PixelSize::Quadrant` — el tamaño más chico que sigue usando sólo
glifos de cuadrante ampliamente soportados (`ThirdHeight`/`Sextant`/
`QuarterHeight`/`Octant` avisan en su propia documentación que pueden verse
mal según la fuente del terminal).

No están en el proyecto y no se agregan preventivamente: `tachyonfx`,
`tracing`/`tracing-subscriber`, `criterion`, `sysinfo`, `windows`. Cada una
se evalúa sólo cuando haya algo concreto que justifique incorporarla (ver
DEC-006, DEC-013).

## 13. Fases

```text
Fase 0  — Fundación de latencia: redraw-on-demand + scheduler mínimo. HECHO.
Fase 1  — Este documento.
Fase 2  — Breakpoints y foco mínimo.
Fase 3  — Componentes visuales base (Composer, Surface, Selector, Field, Notice, Summary).
Fase 4  — Presentation Engine mínimo (reloj, scheduler, transición, easing, VisualQuality::{Off,Normal}).
Fase 5  — Primera mutación real: /nuevo.
Fase 6  — Selector reusable (piloto: Empresa).
Fase 7  — Resumen/Confirmación como transformación del formulario.
Fase 8  — Métricas.
Fase 9  — TachyonFX experimental (prototipo aislado, subordinado al scheduler propio).
Fase 10 — Optimización basada en medición.
Fase 11 — Calidad adaptativa (Auto), sólo si aporta valor real.
```

## 14. Registro de decisiones

```text
DEC-001  El nuevo lenguaje visual sólo aplica a --comandos.
DEC-002  La TUI clásica no se modifica.
DEC-003  Input tiene prioridad absoluta sobre animación.
DEC-004  Las animaciones dependen del tiempo, no del número de frames.
DEC-005  PresentationEngine es dueño único del scheduler de frames.
DEC-006  TachyonFX, si se adopta, queda subordinado al PresentationEngine.
DEC-007  Primera versión de VisualQuality: Off / Normal.
DEC-008  No Tokio hasta existir necesidad async real.
DEC-009  TerminalGuard de comandos permanece independiente del de tui::terminal.
DEC-010  Hardware aporta información; el rendimiento observado decide.
DEC-011  Render nunca modifica estado.
DEC-012  Toda transición debe seguir funcionando perfectamente sin animaciones.
DEC-013  No añadir dependencias sin justificar su necesidad real.
DEC-014  La TUI clásica y --comandos no comparten abstracciones por obligación.
DEC-015  Fase 0: redraw-on-demand con espera dinámica (poll largo en reposo,
         corto durante Argon2, acotado al vencimiento del feedback).
DEC-016  mod.rs sólo orquesta el loop y despacha teclas por fase. Cada
         controlador (login, operando, formulario) vive en su propio
         archivo — evita que la capa de mutación se apoye sobre un único
         archivo con cuatro responsabilidades mezcladas.
DEC-017  La interfaz de comandos es la ruta por defecto de la aplicación
         (sin flags). El flag --comandos se eliminó; la TUI clásica quedó
         detrás de --tui-clasica. La configuración inicial (creación del
         ROOT) sigue siendo exclusiva de la TUI clásica.
DEC-018  Gramática de comandos de 5 piezas: /comando (líder), --letra o
         --palabra (modificador de acción), clave:valor (parámetro, admite
         listas y comillas), -clave:valor (negación), texto libre (ver
         §5.1).
DEC-019  Guion simple reservado exclusivamente para negación de parámetro
         (-clave:valor); doble guion identifica un modificador de acción,
         sin distinguir por longitud (--e y --editar son ambos doble
         guion). Un modificador de acción con guion simple colisionaría de
         verdad con la negación del motor de query, no es sólo preferencia
         de estilo.
DEC-020  /comando líder y --modificador de acción son gramáticas
         mutuamente excluyentes por línea: si hay líder explícito,
         cualquier --x posterior se trata como texto libre, igual que una
         clave:valor no reconocida.
DEC-021  --modificador de acción sólo aplica a comandos "de ítem" (ingreso,
         salida, editar) — actúan sobre el resultado ya encontrado. Los
         comandos globales (nuevo, activos, ayuda, cerrarsesion) sólo
         existen como /comando líder.
DEC-022  El motor clave:valor de --comandos se construye sobre el crate
         `query-parser` (ya vetado en Cargo.lock vía la TUI clásica,
         src/tui/ui_kit/query_lang.rs), con wrapper propio dentro de
         src/comandos/ — no se importa el módulo de la TUI clásica
         directamente (DEC-002/DEC-014). El pilar de uso de este motor es
         Historial (filtrado por tipo/empresa/fecha/etc., aún no
         construido); activos/ingreso/salida son consumidores secundarios
         de parámetros con valor (G:/M:). No se crea el archivo hasta que
         Historial exista de verdad — sin eso sería código sin llamador.
DEC-023  Toda Surface que necesita su propia gramática de input enclava el
         teclado (§5.2): mientras está activa, el input deja de
         interpretarse como /comando, --modificador o búsqueda y pasa a la
         gramática propia de esa Surface. Generaliza el mecanismo ya
         existente de app.formulario.is_some() en operando.rs.
DEC-024  Historial (Surface, aún no construida) aplica su filtro clave:valor
         con Enter explícito, no en vivo tecla por tecla como el resto de
         --comandos — consulta contra el histórico completo, no contra una
         lista corta, así que recalcular en cada tecla es gasto real. Esc
         nunca borra la consulta ya escrita, vuelve a editarla — mismo
         comportamiento que Esc en el resumen del formulario de /nuevo.
DEC-025  El formulario de contratista no tiene un campo "Confirmar": Enter
         intenta guardar desde cualquier campo de Editando (igual que la
         TUI clásica), no sólo al llegar a un campo-botón al final — un
         campo-botón obligaba a navegar hasta él para confirmar, violando
         que Enter significa lo mismo en toda la interfaz (§2 principio 6).
         Space/←/→ abren el selector de empresa además de alternar
         tipo/booleanos, dejando Enter 100% libre para "confirmar".
DEC-026  El desplegable de empresa muta justo debajo de su propio campo
         (como ya hacía la TUI clásica insertando una fila en el layout),
         no al final del formulario desconectado de lo que lo originó.
DEC-027  El formulario aplica la gramática de glifos de §5/`glifo_feedback`
         en vez de marcadores propios: `›` foco (reemplaza el `▸` que
         también usaban las listas de coincidencias y el selector de
         empresa, ahora unificados), `×` error (reemplaza `✗`), `✓` junto a
         los campos que admiten quedar vacíos/inválidos y ya tienen un
         valor válido (`Campo::admite_estado`) — es el indicador de estado
         del proceso que reemplaza al texto "Confirmar — revisar y
         guardar".
DEC-028  Selector de columnas (`F4`), mismo mecanismo que la TUI clásica
         (lista con `[✓]`/`[ ]`, guardrail de "al menos una visible") pero
         reescrito en src/comandos/columnas.rs — no se importa
         src/tui/contratistas ni ningún otro módulo de la TUI clásica
         (DEC-002/DEC-014). Es la tercera Surface enclavada (§5.2), junto al
         formulario y al futuro Historial. Un conjunto de columnas por
         tabla (búsqueda de contratistas y activos), no uno genérico —
         mismo criterio de separación que ya usaba la TUI clásica
         (`activos_columns` ≠ `contratistas_columns`).
DEC-029  Las columnas visibles se persisten en un archivo propio de
         --comandos (`src/comandos/preferencias.rs`,
         `comandos-preferencias.conf`, mismo directorio de datos de la app
         pero nombre distinto), no en `src/tui/preferences.rs` — igual
         criterio de independencia que DEC-028. Se carga al arrancar y se
         guarda sólo si cambió algo, al salir de la app.
DEC-030  ColumnaBusqueda y ColumnaActivos tienen las mismas 7/8 columnas que
         `tui::contratistas::Columna`/`tui::activos::Columna` (Praind, Ruta,
         Acceso, Medio, "Da ingreso"...), no un subconjunto reducido — los
         datos ya viven en `ContratistaResumen`/`IngresoActivoResumen`, así
         que no ofrecerlos en el selector era una limitación arbitraria del
         primer corte, corregida tras revisar la TUI clásica.
```

## 14.1 Escena de login (implementada)

Primera escena real del lenguaje visual, en `src/comandos/render.rs`
(`render_login` y funciones asociadas) + `src/comandos/login.rs`. Sin cajas,
sin bordes, todo centrado — composición propia que no comparte layout con la
interfaz operativa (`render()` bifurca al entrar: `Fase::Operando` sigue el
camino de siempre, cualquier otra fase va a `render_login`).

**Gramática de glifos** (`● › ✓ ! ×`, ver `glifo_feedback` en `render.rs`):
ya es compartida entre login y el resto de la app a través de
`AppState::feedback` — el canal de aviso transitorio construido en Fase 0
(con su auto-expiración ya integrada al scheduler) se reutilizó tal cual
para los errores de login, en vez de inventar un temporizador nuevo.

**Título que muta**: `titulo_grande` resuelve "grande y con personalidad"
con espaciado entre letras + negrita + acento, sin ASCII art multilínea ni
dependencia nueva — se rompe menos en terminales angostas y es la solución
más mantenible. La misma función pinta tanto "Brisas CLI" como el nombre del
operador: es la misma ranura visual mutando de contenido (`titulo_identidad`
decide qué texto según la fase).

**Cursor**: un `_` con estilo, nunca el cursor real del terminal —
`render_login` deliberadamente no llama a `frame.set_cursor_position` (a
diferencia del resto de la app, que sí lo hace) para que el terminal no
dibuje su propio bloque parpadeante encima.

**Flujo implementado**: `LoginCedula` → (Enter resuelve la cédula contra
SQLite, lectura rápida) → `LoginPassword { nombre }` con el título ya mutado
a la identidad → Enter arma el hilo de Argon2 → `Verificando { nombre }`
(`● Verificando`) → éxito entra a `Operando` o error vuelve a
`LoginPassword` con la misma identidad y un aviso `×` autoexpirable.

**Dos partes del pedido original que se dejaron fuera, deliberadamente**:

1. *Spinner de arranque (`●` antes de que "todo esté listo")*: no hay hoy
   ninguna espera real que mostrar ahí — `AppCore::abrir` (SQLite) ocurre en
   `main.rs` antes de que exista terminal donde dibujar nada, y es
   consistentemente rápido. Fabricar un spinner sin trabajo real detrás
   viola el principio propio del documento ("no debe ser una animación
   falsa", sección 5). La escena arranca directo en `LoginCedula`.
2. *Destello de éxito (`✓` solo antes de pasar a idle)*: retrasar la entrada
   a la app unos cientos de ms por una confirmación decorativa choca con
   "la velocidad del operador tiene prioridad sobre la animación" (sección
   15 del prompt original). En su lugar se reusó el mecanismo ya construido:
   al entrar a `Operando` aparece de inmediato el feedback
   `✓ Bienvenido, <nombre>` (el mismo canal transitorio de toda la app),
   sin bloquear el primer comando que el operador quiera escribir.

**Revisión tras feedback visual** (captura real de `cargo run`): el título
pasó de texto espaciado a `BigText` con glifos de bloque de verdad (ver
dependencias, abajo) — sólo para "Brisas CLI"; el nombre del operador quedó
deliberadamente en texto normal, más chico y sin el acento de la marca, para
no competir en jerarquía. El punto de anclaje del prompt se recalculó para
centrarse con el mismo eje que el título (antes usaban dos cálculos de
centrado distintos y quedaban visualmente desalineados). La duración de
aparición subió de 180ms a 320ms (extremo alto de "transición grande") por
sentirse demasiado rápida.

## 14.2 Presentation Engine mínimo (Fase 4, implementada)

`src/comandos/presentation/` — reloj, easing, calidad y motor de aparición.
Sólo lo que hace falta para que el login tenga una mutación *animada* real
(no sólo de contenido): sin foco, sin breakpoints, sin métricas, sin calidad
adaptativa — eso sigue siendo de fases posteriores.

```text
presentation/
├── mod.rs        — re-exporta Engine y VisualQuality
├── quality.rs     — VisualQuality::{Off, Normal} (DEC-007)
├── easing.rs      — Linear, EaseIn, EaseOut, EaseInOut
├── animation.rs   — Animacion: valor interpolado por tiempo, nunca por frame
└── engine.rs       — Engine: HashMap<id, Animacion> + aparecer()/opacidad()/activo()
```

**Cómo se dispara**: en el loop (`mod.rs::run`), entre actualizar estado y
renderizar, `actualizar_presentacion` compara `AppState::firma_login()`
(título/tipo de prompt/presencia de aviso — nunca el texto tecleado) contra
la firma de la vuelta anterior. Cada diferencia real arranca una aparición
(`Engine::aparecer`) con `EaseOut` en ~180ms. Escribir no dispara nada: la
firma es ciega al contenido del input, así que tecla a tecla sigue siendo
instantáneo (cumple "nunca animes el input").

**Cómo se ve**: `render.rs` lee `app.presentacion.opacidad(id)` y funde el
color desde un fondo oscuro (`FADE_FONDO`) hacia el color final de cada
elemento (`estilo_fundido`). Con `VisualQuality::Off` la animación queda
resuelta en el valor final desde el primer frame — mismo resultado
funcional, sin diferencia visual (DEC-012 verificable a simple vista).

**Scheduler**: `proxima_espera` ahora también pregunta `app.presentacion
.activo()` — con una animación en curso el poll baja a ~33ms (30 fps, de
sobra para un fundido de texto); en reposo vuelve a esperar casi
indefinidamente. El teclado sigue respondiendo en la misma vuelta en que
llega, nunca atado al tick de animación.

**Qué falta, deliberadamente**:
- Sólo hay *aparición*. La *desaparición* (p. ej. el aviso `×`
  desvaneciéndose en vez de cortar seco cuando expira) no está cableada —
  requeriría que el motor recuerde el último contenido visible incluso
  después de que `AppState` ya lo limpió, que es más máquina de la que
  hacía falta para esta primera pasada. `Easing::EaseIn` ya existe en el
  vocabulario (documentado para ese caso) pero no se usa todavía.
- No hay interrupción/reversión de una animación a mitad de camino (DEC-003
  la exige en general, pero hoy sólo hay apariciones de ~180ms que rara vez
  se alcanzan a interrumpir de forma perceptible; se revisará si aparece un
  caso real que lo necesite).
- `VisualQuality` no tiene todavía ninguna tecla que la cambie en runtime —
  el campo existe y el motor ya la respeta, falta sólo la superficie de UI
  para alternarla.

## 15. Decisiones pendientes

- Nombre del motor/marca de la nueva experiencia visual — sin decidir
  todavía; no bloquea el trabajo técnico.
- Valores numéricos concretos de los breakpoints (`Compact`/`Normal`/`Wide`)
  — se fijan al construir las primeras escenas reales (Fase 3), no antes.

`TerminalGuard` ya vive en `src/comandos/terminal.rs` propio (independiente
de `tui::terminal`, conforme a DEC-009); `mod.rs` sólo lo importa.

`mod.rs` ya no concentra los cuatro controladores que tenía mezclados
(loop, login, operando, formulario). Se dividió en `login.rs`, `operando.rs`
y `formulario_controller.rs` — ver DEC-016. `mod.rs` quedó en ~200 líneas:
sólo el loop, el scheduler y el despacho de teclas por fase.
