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
Fase 2  — Breakpoints y foco mínimo. PARCIAL: Breakpoint::{Compact,Normal}
          hecho (DEC-037); FocusTarget deliberadamente sin construir (DEC-038,
          sin consumidor real todavía).
Fase 3  — Componentes visuales base (Composer, Surface, Selector, Field, Notice, Summary).
          PARCIAL: SurfaceActiva unifica el despacho de teclado (DEC-039);
          Composer/Selector/Field/Notice/Summary como tipos reales, sin empezar.
Fase 4  — Presentation Engine mínimo (reloj, scheduler, transición, easing, VisualQuality::{Off,Normal}).
Fase 5  — Primera mutación real: /nuevo. HECHO (DEC-040), extendido a
          Historial de una vez: el formulario y Historial ya funden sus
          mutaciones con el Presentation Engine, no sólo el login.
Fase 6  — Selector reusable (piloto: Empresa). Sigue sin generalizar como
          componente, pero ya muta en su lugar (DEC-026) y ya funde
          (DEC-040, vía form_campo) — funcionalmente cubierto.
Fase 7  — Resumen/Confirmación como transformación del formulario. HECHO
          (DEC-040): el título y la acción de la tarjeta de resumen funden
          al aparecer, mismo criterio minimalista que el login.
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
DEC-031  `/historial` (alias `/h`) construido: cuarta Surface enclavada
         (§5.2), la primera que usa Enter-aplica en vez de filtrado en vivo
         (DEC-024, cumplido). `src/comandos/query_lang.rs` (motor
         clave:valor sobre el crate `query-parser`, DEC-022) tiene por fin
         un consumidor real — reescrito desde `tui/ui_kit/query_lang.rs`,
         no importado (DEC-002/DEC-014). Claves: empresa, tipo, estado,
         gafete, ingreso, salida (con negación `-clave:valor`), desde/hasta
         (sin negación — no tiene un significado obvio para el límite de un
         rango).
DEC-032  `FiltroHistorial::hasta` es el límite exclusivo del rango (inicio
         del día siguiente al último incluido): al parsear `hasta:DD/MM/AAAA`
         tecleado por el operador se suma un día antes de convertir a UTC
         — mismo criterio que `tui::historial::filtros::construir`. Mostrar
         el rango de vuelta (resumen del filtro, "Rango actual") hace la
         resta inversa para no mostrarle al operador una fecha que no es
         la que escribió.
DEC-033  La paginación (PageUp/PageDown) fija `corte_id` en la primera
         consulta aplicada (Enter) y lo conserva entre páginas — ingresos
         nuevos registrados mientras el operador navega no corren las
         páginas ya vistas. Una nueva consulta (Enter tras volver a editar
         con Esc) resetea `corte_id` a `None`: es un corte distinto.
DEC-034  `ColumnaHistorial` (7 columnas: ingreso, nombre, empresa, tipo,
         gafete, salida, "da ingreso") implementa el mismo trait `Columna`
         que `ColumnaBusqueda`/`ColumnaActivos` para reusar
         `anchos_columnas`/`fila_columnas` tal cual. (Superado por DEC-035:
         el `F4` que acá se dejaba fuera a propósito se agregó después.)
DEC-035  `F4` sobre la tabla de Historial (sólo con resultado ya aplicado
         en pantalla): abre `edicion_columnas` igual que las otras dos
         tablas, anidado dentro de la Surface de Historial — `Esc` lo
         cierra y vuelve a mostrar el mismo resultado sin volver a
         consultar. `ObjetivoColumnas::Historial` es el tercer caso del
         mismo selector genérico; se persiste en `columnas_historial` del
         mismo archivo de preferencias (DEC-029).
DEC-036  `F5` exporta el filtro completo de Historial a XLSX (mismo atajo
         que la TUI clásica) — reusa `AppCore::exportar_historial` tal cual
         (ya existía, no era exclusivo de la TUI clásica). El destino se
         edita en un `tui_input::Input` propio de `HistorialState`
         (`exportacion_destino`), no en `app.input`, para no pisar el
         filtro que sigue congelado detrás mientras se exporta (DEC-024).
         Primera versión: todas las columnas del exportador
         (`historial::exportacion::ColumnaHistorial::ALL`, un enum de
         dominio con más detalle que el `ColumnaHistorial` de
         `comandos::columnas` usado para la vista) — elegir un subconjunto
         para exportar queda fuera de esta pasada, es todo o nada.
DEC-037  `Breakpoint` (Fase 2, primera mitad) tiene sólo `Compact`/`Normal`,
         no `Wide` como sugería el bosquejo original de §10 — hoy no hay
         ningún componente que necesite distinguir "ancho" de "muy ancho";
         el único umbral real en todo `--comandos` es si `/activos` muestra
         la columna Empresa (antes `ANCHO_TABLA_COMPLETA`, un `const`
         suelto, ahora `Breakpoint::desde_ancho`). Mismo criterio que ya se
         aplicó a `VisualQuality::{Off,Normal}` (§11): no fabricar una
         variante sin un consumidor real.
DEC-038  `FocusTarget` (Fase 2, segunda mitad) se deja explícitamente sin
         construir — hoy ningún componente necesita preguntar "¿qué tiene
         el foco?" de forma genérica; cada Surface ya resuelve su propio
         foco (`Campo` en el formulario, el índice de selección en
         Historial/columnas). Su primer consumidor real llegaría con el
         motor de presentación animando algo más que el login (Fase 5) —
         se construye cuando eso pase, no antes. (Nota tras DEC-040: Fase 5
         ya pasó y no lo necesitó — cada Surface sumó su propia `Firma*`,
         el mismo patrón que `FirmaLogin` pero sin unificar en un tipo
         común. Sigue sin haber un consumidor real de `FocusTarget`.)
DEC-039  `SurfaceActiva` (`AppState::surface_activa()`) es el primer paso
         real de Fase 3: reemplaza los tres `if x.is_some() {...} else if
         y.is_some()...` que `operando.rs` encadenaba para decidir qué
         Surface tiene el teclado, con una sola función que responde eso en
         un solo lugar. Deliberadamente NO es la Fase 3 completa: sigue
         siendo tres campos separados de `AppState` con tres controladores
         separados, no una abstracción `Composer`/`Surface`/`Selector`/
         `Field`/`Notice`/`Summary` de verdad — ese rediseño es mucho más
         grande y arriesga romper tres funciones que hoy trabajan bien; se
         hace aparte, no de paso.
DEC-040  Fase 5 extendida a formulario e Historial, mismo mecanismo que el
         login (`Firma*` + `presentacion.aparecer(id, calidad)`), sin
         generalizar en un tipo común (ver nota en DEC-038):
         `FirmaFormulario` (campo activo, selector de empresa, resumen,
         presencia de error) y `FirmaHistorial` (resultado aplicado, total,
         exportando) en `estado.rs`; `actualizar_presentacion_formulario`/
         `_historial` en `mod.rs`, junto a la del login (ahora
         `actualizar_presentacion_login`). Nunca se anima tecla a tecla:
         ninguna firma incluye texto tecleado. Elementos que funden: el
         campo activo del formulario (marcador + etiqueta + valor si no es
         de texto), los `×` de error (todos juntos, no por campo), el
         título y la acción de la tarjeta de resumen, el encabezado del
         resultado de Historial (aparece o cambia de página/consulta) y la
         pantalla de exportación. El desplegable de empresa y las filas de
         la tabla de Historial quedan sin fundir a propósito — extender
         `estilo_seleccion()` (`Modifier::REVERSED`) con una opacidad
         interpolada es una combinación visual no verificada todavía en
         runtime real, se deja para cuando se pueda confirmar cómo se ve.
DEC-041  El prompt de Operando (`render_prompt_linea` — línea de comandos,
         campos del formulario, filtro de Historial, destino de
         exportación) usa un cursor propio, nunca el cursor real del
         terminal — mismo criterio que ya usaba sólo el login. Reportado en
         runtime real: el cursor real parpadeaba y desaparecía de forma
         inconsistente (comportamiento del emulador, fuera de nuestro
         control). A diferencia del login (que sólo escribe al final), acá
         el cursor puede quedar a mitad del texto (←/→/Home/End de
         `tui_input`), así que en vez de un "_" insertado se resalta
         (`Modifier::REVERSED`) el carácter que está bajo el cursor — un
         espacio reversado si no hay carácter ahí (fin de línea).
DEC-042  El estado de cada campo del formulario vive en un solo glifo a la
         izquierda (`›` en edición, `×` con error, `✓` completo, nada si
         ninguno aplica todavía) en vez de dos indicadores separados como
         antes (foco a la izquierda, validez a la derecha) — son estados
         del mismo lugar, nunca simultáneos, y por eso comparten slot. `›`
         gana mientras el campo está activo (es la información más útil en
         ese momento); `×`/`✓` sólo aparecen al alejarse. Reportado en
         runtime real: el `✓` aparecía a la derecha mientras se seguía
         escribiendo, antes de confirmar nada — confuso, parecía validado
         cuando sólo estaba "no vacío".
DEC-043  Cédula y Nombre filtran caracteres al teclear, no sólo largo
         máximo — mismo criterio que ya tenía Fecha PRAIND (un carácter que
         no corresponde no se inserta, sin aviso ni error, simplemente no
         aparece). Reportado en runtime real: Cédula aceptaba letras,
         Nombre aceptaba dígitos y símbolos — ninguna de las dos lo filtraba
         nunca, ni acá ni en la TUI clásica (mismo hueco, nunca se había
         notado). Cédula: sólo dígitos ASCII. Nombre: letras (con acentos y
         ñ vía `char::is_alphabetic`), espacios, guion y apóstrofo (nombres
         compuestos).
DEC-044  Cédula se verifica contra duplicados de forma proactiva, no sólo
         al guardar: al dejar el campo (↓) o al confirmar con Enter desde
         cualquier campo, `formulario_controller.rs` consulta
         `AppCore::buscar_contratistas` (comparación exacta, la búsqueda en
         sí es difusa) y marca el error en el campo si ya existe otro
         contratista con esa cédula — en modo edición se excluye al propio
         contratista. La restricción `UNIQUE` de la base sigue siendo la
         autoridad final (condición de carrera con otra terminal creando la
         misma cédula al mismo tiempo); esto es sólo para que el operador
         se entere antes de llenar el resto del formulario, no un
         reemplazo. `formulario.rs` sigue sin tocar `AppCore` — la consulta
         vive en el controlador, el modelo puro no gana una dependencia
         nueva.
DEC-045  `/nuevo` gana un argumento posicional (`contratista` por defecto,
         `empresa`/`em`, `usuario`/`u`) en vez del `--c`/`--e`/`--u`
         propuesto originalmente. Dos razones: (1) `/nuevo` es un comando
         global, y DEC-021 ya estableció que `--modificador` es sólo para
         comandos de ítem (Ingreso/Salida/Editar) — usarlo acá rompería esa
         regla sin necesidad; (2) con el parser actual `/n --c` ni siquiera
         funcionaría como se esperaba, caería a texto libre. El positional
         además ya tiene precedente (`/editar <nombre>`). Se evita el alias
         corto `e` (reservado en la cabeza para "editar") — la forma corta
         de empresa es `em`.
DEC-046  El formulario de Empresa (un solo campo, nombre) no tiene paso de
         Resumen — a diferencia de Contratista y Usuario. Con un solo campo,
         una segunda pantalla de revisión es fricción sin valor (§2.8,
         "¿realmente hace falta?"); Enter desde el campo guarda directo.
DEC-047  El formulario de Usuario sí conserva el paso de Resumen (mismo
         patrón que Contratista, DEC-025: Enter intenta confirmar el
         formulario completo desde cualquier campo y sólo entonces avanza a
         Resumen) — a diferencia de Empresa, acá hay varios campos con
         consecuencias reales (contraseña, asignación de rol), y una
         revisión antes de guardar sí aporta.
DEC-048  Crear un usuario hashea la contraseña de forma síncrona
         (`AppCore::crear_usuario`), no en un hilo aparte como el login
         (`login.rs`, con su propio canal y estado pendiente). El login
         hashea en cada intento de autenticación — el camino más frecuente
         de toda la app — y por eso justifica esa plomería. Crear un usuario
         es una acción administrativa poco frecuente: el bloqueo es de
         cientos de ms, una sola vez, en una acción explícita del operador,
         no tecla a tecla. Threading esto exigiría un tipo de estado
         pendiente propio y tocar `mod.rs`/`operando.rs`/`manejar_tecla` sin
         beneficio real todavía.
DEC-049  El selector de Rol en el formulario de Usuario (Space/←/→) sólo
         cicla entre los roles que `puede_gestionar_usuario(rol_actor, _)`
         permite — un Administrador no puede llegar a "Root" ni por
         accidente, la opción simplemente no está en la lista que recorre.
DEC-050  La contraseña nunca se muestra en texto plano en ningún punto de la
         UI: en el campo del área de contenido y en la barra de prompt se
         enmascara con `•` (recalculado por longitud real, no un placeholder
         fijo); en el Resumen se muestra "(definida)" en vez del valor.
DEC-051  Existen dos gramáticas `clave:valor` distintas bajo la misma
         sintaxis visual: la de parámetros de ítem (`G:`/`M:` en
         `parser.rs`, un solo valor, sin lista ni negación) y la de
         Historial (`query_lang.rs`, listas `a,b,c` y negación `-clave:`).
         Deliberadamente no se unifican — `G:`/`M:` son dos parámetros
         fijos con un único valor válido cada uno, no una búsqueda abierta;
         darles lista/negación sería complejidad sin caso de uso real. En
         cambio se hace explícita la diferencia en `/ayuda` (línea aparte
         bajo "Claves:") para que no dependa de que el operador la infiera
         probando `-G:27` y viendo que simplemente no pasa nada.
DEC-052  `/editar` gana el mismo argumento posicional que `/nuevo`
         (DEC-045): `/editar <sujeto> <consulta>`, sujeto opcional
         (contratista por defecto, `empresa`/`em`/`emp`, `usuario`/`u`).
         Sólo el primer token de la consulta se interpreta como sujeto —
         a diferencia de `/nuevo`, acá siempre queda texto después (la
         búsqueda), así que no se agrega alias corto para "contratista":
         con `/nuevo` no había ambigüedad posible (no admite consulta),
         acá "c" seguido de una búsqueda de una sola palabra sí la
         generaría — se deja "sin prefijo" como el único camino a ese
         sujeto (ya era el comportamiento antes de que existieran los
         otros dos). `empresa`/`usuario` abren una búsqueda en vivo nueva
         (`ContextState::CoincidenciasEmpresas`/`CoincidenciasUsuarios`,
         mismo patrón que `Coincidencias` de contratista) que al confirmar
         con Enter abre el mismo `FormularioEmpresa`/`FormularioUsuario`
         de alta, en modo `Editar { id }`, precargado con la búsqueda
         (`AppCore::actualizar_empresa`/`actualizar_usuario`). Sin
         selector de columnas (F4): son listas de paso (elegir y entrar a
         editar), no reportes — `EmpresaResumen`/`UsuarioResumen` ya
         tienen pocos campos. Buscar usuarios exige
         `Operacion::GestionarUsuarios`, mismo gate que crearlos.
DEC-053  Editar un usuario deja Contraseña/Confirmar en blanco al abrir —
         blanco significa "no cambiarla", distinto de alta donde siempre
         es obligatoria. `FormularioUsuario::validar` sólo exige el
         mínimo de 8 caracteres y que coincidan si el operador escribió
         algo en cualquiera de los dos campos; si no, `DatosUsuario::
         Actualizar` viaja con `password: None` y `AppCore::
         actualizar_usuario` ni se entera de que hubo un campo de
         contraseña en pantalla. Activar/desactivar el usuario queda
         fuera de este formulario a propósito (mismo corte de alcance que
         Empresa): son acciones con consecuencia propia, no un campo más
         para tocar de pasada.
DEC-054  El borde del recuadro del prompt cambia a acento (cian) mientras
         hay una Surface enclavada (§5.2) — formulario, columnas o
         Historial — y vuelve a `muted()` en cuanto se cierra. Antes la
         única señal de "el teclado ya no es de comandos" era el
         contenido de la pantalla y la pista de abajo; el borde da la
         misma información de un vistazo, sin tener que leer nada.
DEC-055  El buscador principal (texto sin `/`, y por extensión `/ingreso`,
         `/salida`, `/editar` — todos comparten `buscar_contratistas`)
         deja de buscar por nombre de empresa: sólo cédula y nombre del
         contratista. Encontrar a alguien tecleando el nombre de su
         empresa era una coincidencia del filtro compartido con
         `empresa:` de Historial (mismo `FiltroContratistas`), no un
         criterio que el operador esperara del buscador — reportado en
         runtime real. El cambio vive en `construir_where`
         (`database/queries/contratistas.rs`), la capa compartida por
         ambas interfaces: la TUI clásica hereda la misma acotación.
         `empresa:` sigue existiendo tal cual dentro de `/historial`
         (`FiltroHistorial::empresa_id`, consulta separada) — ahí sí es
         un criterio explícito que el operador pidió a propósito.
DEC-056  `/activos` navega con ↑↓ y Enter sobre una fila lleva a
         `ResumenSalida` — antes era una tabla de sólo lectura (comentario
         explícito en el código: "esta vista no navega ítem por ítem",
         decisión que se revierte acá porque en la práctica sí hacía
         falta: el operador mira `/activos`, reconoce a alguien y quiere
         darle salida ahí mismo, sin repetir la búsqueda en `/salida`).
         Reutiliza el mismo `ContextState::ResumenSalida` y el mismo
         marcador `›` que ya usa `CoincidenciasActivos` — misma fuente de
         datos (`IngresoActivoResumen`), sólo cambia de dónde se llega.
DEC-057  `/gafete` (alias `/g`) abre un modo enclavado dedicado a la salida
         más frecuente de la portería: alguien entrega el gafete y se va,
         sin que siempre se sepa el nombre. A diferencia de `/salida`
         (texto libre O `G:`, puede devolver varias coincidencias por
         cédulas que contienen el número), acá sólo hay dígitos — uno o
         varios separados por coma (`2, 25, 85`, para un grupo que sale
         junto) — y el gafete es único entre ingresos activos: match
         exacto, sin ambigüedad. Enter confirma directo, **sin** una
         segunda pantalla de "¿está seguro?": la vista previa en vivo
         (nombre de cada gafete mientras se teclea) ya cumple ese papel —
         reducir esto a "encolar y confirmar" fue pedido explícito, no
         sólo conveniencia de implementación. A diferencia de toda otra
         Surface, no se cierra sola tras confirmar — el campo se limpia y
         el modo se queda abierto para el siguiente gafete (o grupo); sólo
         Esc lo cierra. Si `/gafete 2, 25` llega con la lista ya escrita
         antes del primer Enter, se procesa de una vez en el mismo paso
         (`ContextState::AbrirSalidaGafete { texto }` la lleva) — no hace
         falta un segundo Enter sobre la Surface recién abierta y vacía
         para algo que el operador ya escribió.
DEC-058  La paleta de comandos (la lista que aparece bajo el input al
         escribir `/algo`, antes sólo visual) ahora navega con ↑↓ y
         Tab/Enter completan con la fila resaltada — no la primera
         alfabética. Motivación funcional, no cosmética: comandos sin
         alias de una letra (`/ay` para Ayuda, `/c...` para
         CerrarSesion — no existe alias "c") ya mostraban la coincidencia
         en la paleta pero Enter no la usaba, porque `parser::parsear`
         nunca llega a `Comando::desde_texto` con un prefijo parcial sin
         alias exacto — el operador tenía que terminar de escribir el
         nombre completo. `AppState::paleta_comandos()` (movido desde
         `render.rs`, antes privado ahí) es ahora el único punto de
         verdad de qué muestra la paleta — lo consultan tanto el render
         como `operando.rs`, que antes no tenían forma de saberlo.
         Enter con la paleta visible completa el texto (mismo resultado
         que Tab) en vez de confirmar — el Enter que de verdad ejecuta la
         acción es el siguiente, ya con el comando completo: mismo
         patrón de "un paso más" que Coincidencias → Resumen, no una
         excepción nueva.
DEC-059  El área de contexto (`ContextState`, lo que se ve arriba del
         prompt) funde al cambiar de "tipo de pantalla" — mismo mecanismo
         de Fase 5 (Firma comparada tick-a-tick) que ya usaban login,
         formulario e Historial, aplicado acá por fin. Dos decisiones que
         lo distinguen de esos tres:
         (1) La firma es sólo `std::mem::discriminant(&contexto)`, sin un
         struct `FirmaContexto` dedicado — `ContextState` tiene más de 15
         variantes y compararlo completo dispararía una aparición en cada
         tecla (`items`/`consulta` cambian todo el tiempo); el
         discriminante ignora eso y sólo reacciona al cambio real de
         pantalla (Inicio → resultados, resultados → tarjeta…).
         (2) En vez de enhebrar un parámetro de opacidad por cada función
         de `lineas_contexto` (15+ funciones, reescribir cada
         `Span::styled`), `render.rs::atenuar()` re-interpola el color que
         cada línea ya trae hacia `FADE_FONDO` — un solo punto de cambio.
         `color_a_rgb` traduce el `Color` con nombre (`Cyan`, `DarkGray`…)
         de vuelta a su constante `FADE_*`; con `opacidad >= 1.0` no toca
         nada, así que en reposo el color sigue siendo exactamente el
         original — la aproximación de `color_a_rgb` sólo se nota (si acaso)
         durante los ~200-400ms de la propia aparición, nunca en reposo.
DEC-060  El `> ` de la línea de comandos (sin ninguna Surface abierta)
         muta al símbolo del feedback vigente (✓/!/×, mismo vocabulario de
         siempre) mientras dure — no sólo el color, el símbolo mismo — y
         funde al aparecer (asimétrico: al expirar vuelve a `> ` sin
         fundido, igual criterio que el resto de esta fase). Patrón
         explícitamente pedido tras revisar precedente ("success
         checkmark replaces label, then reverts" — Pencil & Paper,
         pencilandpaper.io/articles/success-ux): confirmación inline en
         el mismo lugar donde ya está la atención del operador, sin
         toast ni modal aparte. Deliberadamente NO se aplica a ↑↓ sobre
         listas (Coincidencias, `/activos`…): es una interacción repetida
         como el propio tecleo, no una transición de estado — mismo
         espíritu de DEC-004 ("nunca animes el input"). Tampoco se
         extiende a las demás etiquetas del prompt (`gafete › `,
         `historial › `…): ésas ya llevan texto descriptivo propio, no
         hay un símbolo suelto de una posición que reemplazar sin perder
         esa etiqueta.
DEC-061  `/ayuda` se reorganiza en 5 secciones (FRECUENTES, GESTIÓN,
         HISTORIAL, SISTEMA, SINTAXIS Y ATAJOS) en vez de una lista plana
         de 18 filas seguida de 6 líneas sueltas de sintaxis avanzada, sin
         jerarquía entre "comando" y "regla de gramática". Agrupar por
         categoría lógica y separar la sintaxis avanzada de los comandos
         en sí es la práctica de referencia en ayudas de CLI ("progressive
         disclosure" — clig.dev, bettercli.org/design/cli-help-page).
         Las categorías no son nuevas: son las mismas que ya usa el resto
         del diseño (frecuente/ocasional de §5.1, gestión vs. exploración
         vs. sistema) — la ayuda no las inventa, sólo por fin las muestra.
         `seccion_ayuda()` arma encabezado (negrita) + filas (sintaxis en
         acento, descripción en muted) con el mismo ancho de columna en
         todas las secciones, para que quede alineado de punta a punta.

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

**Segunda revisión tras feedback visual** (captura real de login →
contraseña, corriendo con credenciales reales): `espaciar_texto` (un
espacio entre cada letra, pensado para el nombre del operador — ver arriba)
se había filtrado a dos lugares donde no correspondía: el **valor tecleado**
del prompt (`valor_espaciado = espaciar_texto(&valor_prompt(...))` —
espaciaba los dígitos de la cédula y los puntos de la contraseña
enmascarada, justo lo que el operador necesita releer con más precisión,
no menos) y el **nombre del operador** mismo, donde se sentía impostado en
un nombre real ("D A N I E L"). Se sacó de ambos; la etiqueta fija del
prompt ("Identificación"/"Contraseña") conserva el espaciado — ahí sí es
deliberado (texto decorativo fijo, no dato que el operador necesita leer
exacto).

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
