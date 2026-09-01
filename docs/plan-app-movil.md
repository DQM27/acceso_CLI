# Plan App Móvil — registro de entrada/salida (planeación, sin código todavía)

> Diseñado por conversación con el usuario (2026-09-01), sin código escrito aún. Documenta el
> recorrido completo de decisiones — incluidas las alternativas que se consideraron y por qué se
> descartaron — porque varias de ellas cambiaron a medida que aparecían datos nuevos (la red real
> del sitio, que sería un solo teléfono, que la búsqueda no aguanta ida y vuelta por internet).
> Antes de tocar código, este plan debe confirmarse con el cliente en los puntos marcados como
> abiertos (sección final).

## Contexto

El cliente pidió una app móvil. De esa conversación salieron **dos proyectos distintos, no relacionados entre sí**:

1. **Compartir información en línea, solo lectura** — ver el estado del sistema (activos, historial) desde afuera. No detallado en este documento; queda como idea suelta, sin alcance ni arquitectura definidos todavía.
2. **App mínima de registro de entrada/salida** — la que cubre este plan.

Este documento es sólo sobre el punto 2.

## Alcance decidido

**Qué SÍ hace la app:**
- Buscar/seleccionar un contratista.
- Registrar su entrada.
- Registrar su salida.
- Registrar contratistas nuevos (decisión ampliada 2026-09-01 — el alcance original no lo traía; se porta el mismo formulario de `desktop/src/pantallas/FormularioContratista.tsx`, alta solamente).
- Registrar empresas nuevas (misma ampliación 2026-09-01, alta solamente — sin edición ni desactivación).
- Ver Historial de movimientos (misma ampliación 2026-09-01 — revierte la exclusión original; últimos 6 meses por defecto, sólo lectura, sin exportar).
- Registrar usuarios nuevos (misma ampliación 2026-09-01 — revierte la exclusión original; alta solamente, sin edición/desactivación/reseteo de contraseña). Gateado por rol igual que el núcleo: sólo Root/Administrador ven la opción y sólo ellos pueden ejecutarla (`Operacion::GestionarUsuarios`, `domain/autorizacion.rs`) — un Operador ni ve el botón en el menú "+".
- **Fusión de "Buscar" dentro de "Activos" (2026-09-01).** La pestaña "Buscar" (paso 4 de la lista de abajo) se elimina como pestaña aparte: sólo quedan Activos e Historial. El mismo buscador vive ahora dentro de Activos y cambia de sentido según el campo esté vacío o no — mismo truco de "un solo campo, dos interpretaciones" que ya usa `desktop/src/pantallas/SalidaModal.tsx` (ahí con un checkbox "Por gafete"). Campo vacío: lista quién está adentro, tocar un nombre confirma su salida (comportamiento sin cambios). Con texto: busca en el catálogo completo de contratistas, tocar un resultado arranca `prepararIngreso` → confirmar entrada (comportamiento sin cambios, sólo cambia desde dónde se dispara) — inspirado en cómo `desktop/src/pantallas/NuevoIngresoModal.tsx` expande el panel de confirmación en el mismo lugar y, al confirmar, no navega ni cierra: limpia y queda listo para la siguiente persona. Con esto una sola vista administra el ciclo completo ingreso → permanencia → salida, igual que en el Tauri de escritorio.

**Qué NO hace (a propósito, para mantenerla mínima):**
- Sin edición de contratistas/empresas/usuarios ya creados (sólo alta).
- Sin Auditoría, Respaldos, ni Gafetes como catálogo administrable.

Si el alcance crece más adelante, es una decisión de producto nueva — no asumir que "ya que estamos" se agregan pantallas.

## Decisiones ya tomadas (no reabrir sin una razón nueva)

- **Plataforma: Android únicamente** por ahora. iOS no está en el alcance de este plan.
- **Un solo teléfono**, no varios. Es la premisa que simplifica todo el diseño: sin eso, haría falta resolver sincronización con conflictos entre múltiples escritores — un problema real y grande (mismo tipo que "V3: concurrencia multi-terminal" en `docs/pendientes.md`), no proporcional a una app mínima. Si el cliente pide un segundo teléfono más adelante, este plan debe revisarse desde cero en esa parte.
- **Motivo de negocio del teléfono:** el guardia necesita moverse, no puede estar sentado esperando en la PC. Esa es la razón real de que exista la app — no "porque se puede", sino porque la PC fija no le da esa flexibilidad.
- **Mismo usuario en ambos lados** — no hace falta un modelo de autenticación nuevo ni un "usuario dedicado al celular"; se loguea con el mismo usuario que ya existe en el sistema.
- **El núcleo de Rust se reutiliza tal cual, nunca se traduce a otro lenguaje.** Se evaluó explícitamente "traducir toda la lógica a Kotlin" y se descartó: duplicar las reglas de negocio (PRAIND vencido, exclusividad de gafetes, fechas/zona horaria Costa Rica-UTC) en dos lenguajes distintos es un riesgo real de que ambas copias diverjan con el tiempo, y se pierde de un plumazo la cobertura de los +500 tests que ya validan esa lógica en Rust. Reusar el núcleo (sea cual sea la interfaz encima) es la opción más robusta, no la más simple de las tres evaluadas.
- **Interfaz elegida: Kotlin + Jetpack Compose, nativo, con el núcleo de Rust reutilizado vía `uniffi-rs`** (la herramienta de Mozilla que genera el puente Kotlin↔Rust; la usan en Firefox Android). Se descartó Tauri Mobile para esto específicamente — ver la comparación en la sección siguiente.
- **Toda la lógica de negocio corre en el teléfono**, con su propia base SQLite local — no es un cliente que le pregunta todo a la PC en tiempo real (ese diseño se consideró primero y se abandonó, ver más abajo). Como es un solo teléfono, esa base local ES la fuente de verdad de lo que ese teléfono registra — no hay conflicto posible porque no hay un segundo escritor.
- **El rol de la PC cambia:** deja de validar en vivo. Pasa a ser receptora de reportes/copias que le llegan del teléfono — el mecanismo exacto de "cómo llegan esos reportes" queda **abierto** (ver sección final).

## Comparación que llevó a la decisión de interfaz

Con el núcleo de Rust corriendo completo en el teléfono (no un cliente liviano), la pregunta relevante no es "Kotlin vs. Rust" — la lógica siempre queda en Rust, reusada, no importa la interfaz. La pregunta real fue **qué envoltorio de interfaz usar sobre ese mismo núcleo**:

| | Tauri Mobile | Kotlin + Compose + `uniffi` |
|---|---|---|
| Reusa el núcleo de Rust sin duplicar lógica | Sí | Sí |
| Fricción de arranque | Baja — reusa el mismo patrón de comandos que ya funciona en `desktop/src-tauri` | Alta — hay que integrar `uniffi`, el NDK de Android, y Gradle desde cero |
| Interfaz | WebView embebido | Widgets nativos de Android |
| Fluidez | Buena, con una brecha pequeña pero real frente a nativo | La máxima posible en Android |

Ambas opciones son legítimas y ninguna es "la incorrecta". Se eligió Kotlin nativo porque el teléfono va a ser el **dispositivo principal** de uso diario de un guardia en movimiento — ahí la fluidez nativa pesa más que la menor fricción de armar Tauri. Si en algún punto el ritmo de desarrollo importa más que la fluidez (por ejemplo, si hay presión de tiempo real), Tauri Mobile sigue siendo la alternativa de respaldo razonable, no una que se descartó por mala.

## Por qué NO quedó como "cliente liviano hablándole a la PC en vivo" (diseño anterior, superado)

Antes de saber que sería un solo teléfono cuya razón de ser es la movilidad, se diseñó una primera versión: el teléfono sin lógica propia, preguntándole todo a la PC (que sería la única fuente de verdad) a través de un túnel de Cloudflare (gratis, sin necesitar que TI abra puertos ni dé acceso a la red del sitio — dato real del contexto: el desarrollador no tiene ese acceso).

Ese diseño se abandonó al aparecer dos problemas reales, no hipotéticos:

1. **El buscador se sentía roto.** Cada tecla escrita hubiera disparado una consulta por internet (~150-400ms estimados de ida y vuelta); como se escribe más rápido que eso, los resultados llegaban desordenados — se probó conceptualmente con el ejemplo real "escribo Jen de Jenna Ortega y ya me aparece el resultado completo". La búsqueda necesita ser instantánea, y eso exige datos locales, no un viaje de red por cada letra.
2. **La PC como único punto de decisión en vivo no encajaba con que el guardia necesita moverse** — si la razón de ser del teléfono es la movilidad, depender de que la PC esté prendida y conectada en cada confirmación le quita justo el beneficio que se buscaba.

La infraestructura de red (túnel de Cloudflare) sigue siendo relevante — no se descarta —, sólo que ahora se usa para que el teléfono le mande reportes a la PC de vez en cuando, no para validar cada acción en vivo.

## Piezas técnicas concretas

- **Núcleo compartido:** `control_acceso` (este mismo repo) compilado para Android vía `cargo-ndk` (targets `aarch64-linux-android` como mínimo; evaluar si hace falta `armv7` para equipos viejos). `uniffi-rs` genera los bindings Kotlin sobre `AppCore`.
- **Base de datos:** SQLite local en el teléfono, mismo esquema y migraciones que ya existen — `rusqlite` con la feature `bundled` ya usada hoy no depende de nada externo al binario.
- **Interfaz:** Kotlin + Jetpack Compose. Pantallas mínimas: login, buscar/seleccionar contratista, confirmar entrada, confirmar salida.
- **Búsqueda rápida:** sobre los datos ya locales de SQLite en el teléfono — no hay problema de latencia de red porque no hay red de por medio para esto (el teléfono ya tiene su propia copia completa).
- **Asistencia de captura — reconocimiento de cédula (OCR):** ML Kit Text Recognition (Google, on-device, gratis, sin internet) lee el número de cédula impreso en el carnet físico del contratista, para no tener que escribirlo a mano. Es más lento que un QR (aprox. un segundo de cámara quieta contra prácticamente instantáneo) pero funciona con el carnet que ya existe hoy, sin pedirle nada a nadie.
  - Se investigó usar el QR que ya traen los carnets de inducción (sistema "Prime Logic" de un tercero): decodificado en la práctica, apunta a `https://prd.pw/X95c...` — un token opaco de un acortador, sin la cédula ni ningún dato reutilizable adentro, y sin acceso a su API. **Descartado como fuente de datos automática** — no hay forma de enlazarlo con nuestra base sin depender de scrapear un sistema ajeno. Queda como posible mejora *manual* a futuro (escanear ese QR sólo para que el guardia vea con sus propios ojos si la inducción sigue vigente — no para autocompletar nada).
  - Mejora futura evaluada y no implementada: generar QR propios por contratista (velocidad real de QR, no de OCR) — requiere imprimir y distribuir etiquetas físicas, una decisión operativa del cliente, no sólo de software.
- **Conectividad:** Cloudflare Tunnel desde la PC (gratis, sin necesitar cambios de red/firewall de parte de TI) — usado para que el teléfono le mande reportes a la PC, no para validaciones en vivo.
- **Prioridad de esfuerzo: el buscador, no el OCR.** El OCR de cédula (ML Kit) es un acelerador *opcional* del paso de búsqueda — la vía primaria con la que el guardia va a encontrar al contratista siempre es escribir en el buscador, no la cámara. Si hay que elegir dónde invertir tiempo de pulido primero, es en que la búsqueda local sea instantánea y tolerante a como la gente realmente escribe (typos, orden de nombre/apellido, etc.), no en el reconocimiento de cédula. El OCR puede llegar después o quedar fuera del piloto sin que eso bloquee nada.

## Abierto — confirmar antes de empezar a programar

- **Mecanismo exacto de "reportar a la PC".** No se definió todavía: ¿el teléfono manda un archivo/reporte periódico (¿cada cuánto?), ¿un endpoint que reciba los registros nuevos desde la última sincronización y los inserte en la base de la PC, ¿algo más simple como exportar y mandar por correo/WhatsApp? Esto determina si hace falta un servidor nuevo del lado de la PC (como el que se había diseñado para el modelo anterior) o si alcanza con algo más simple dado que ya no valida nada en vivo.
- **Confirmar con el cliente que de verdad es un solo teléfono, ahora y a futuro cercano** — es la premisa que evita el problema de sincronización con conflictos. Si la respuesta cambia, hay que rediseñar esta parte antes de escribir código, no después.
- **Qué pasa si el teléfono se pierde, se rompe, o se resetea** — con toda la lógica y los datos localmente, hay que decidir una política de respaldo del lado del teléfono (¿se manda un reporte apenas se registra cada movimiento, para no perder nada si el equipo falla antes de sincronizar?).
- **Alcance de "reportes"** — ¿la PC necesita los movimientos completos (para que Historial/Auditoría los vean como si hubieran pasado por la garita física), o sólo un resumen para control administrativo? Esto cambia bastante el diseño del lado de la PC.

## Orden sugerido cuando se apruebe

1. Confirmar los puntos abiertos de arriba con el cliente.
2. Armar el toolchain: `cargo-ndk` + `uniffi-rs` generando bindings desde `control_acceso` tal cual existe, sin tocar su lógica — probar que compila y corre en un emulador/dispositivo antes de escribir una sola pantalla.
3. Pantalla de login (reusa `autenticacion_service` sin cambios).
4. Buscar contratista (lista local + filtro).
5. Confirmar entrada / confirmar salida (reusa `RegistroIngresoService` sin cambios).
6. Reconocimiento de cédula (ML Kit) como acelerador del paso 4.
7. Mecanismo de reporte hacia la PC, una vez resuelto el punto abierto correspondiente.
