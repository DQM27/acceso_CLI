# Arquitectura de la app móvil — criterio para mantener en el tiempo

> Diagnóstico honesto (2026-09-02): la app se armó como MVP/piloto, rápido y
> funcional, sin capa de presentación separada de la lógica de negocio. Sí
> hizo bien lo importante — reusar el núcleo de Rust sin duplicar reglas,
> mantener las pantallas simples — pero mezcló responsabilidades dentro de
> cada `@Composable` de una forma que no escala si el equipo crece o si el
> alcance crece más allá del piloto actual. Este documento fija el criterio
> a seguir de ahora en adelante — no es una reescritura inmediata, es la
> regla contra la que se mide código nuevo y contra la que se refactoriza
> código viejo la próxima vez que se toque.

## El problema concreto de hoy

Cada pantalla (`Pantalla*.kt`, `MainActivity.kt`) hace **las tres cosas a la
vez**:

1. Dibuja UI (lo único que un `@Composable` debería hacer).
2. Guarda y muta estado de negocio (`remember { mutableStateOf(...) }`
   sosteniendo resultados de búsqueda, sesión, errores).
3. Llama directo a `Nucleo` (la capa de datos) y decide qué hacer con el
   resultado o la excepción.

Consecuencias reales, no hipotéticas:

- **`PantallaActivos.kt` pasa de 550 líneas** porque mete 3 modos de
  búsqueda + llamadas async + manejo de error + UI en una sola función.
  Un archivo así dejó de ser "una pantalla", es varias responsabilidades
  disfrazadas de una.
- **Ningún estado sobrevive rotación de pantalla ni que el sistema mate la
  Activity en background** (`remember` sin `rememberSaveable`, ver más
  abajo) — un formulario a medio llenar se pierde solo.
- **Nada de esto es testeable sin un emulador/dispositivo**, porque la
  lógica vive pegada a Compose en vez de en una clase de Kotlin normal.
- **`catch (excepcion: Exception)` genérico** en casi todos lados — un bug
  real (`NullPointerException`, por ejemplo) se confunde con un error de
  negocio esperado y se le muestra al guardia como si fuera lo mismo.

Ninguno de estos puntos es un problema del núcleo de Rust (`rust-core/`,
ya con lints estrictos y tests) — es específicamente cómo Kotlin organiza
su propio código alrededor de ese núcleo.

## La regla: tres capas, una responsabilidad cada una

```
Pantalla (@Composable)  →  ViewModel  →  Nucleo (Rust vía uniffi)
     UI pura              dueño del estado      lógica de negocio real
   sin lógica            y de las llamadas         (no tocar aquí)
```

### 1. `Pantalla*.kt` — sólo UI

- Recibe el estado ya resuelto (un `data class`/`sealed class` inmutable) y
  funciones lambda para reportar eventos (`onBuscar: (String) -> Unit`,
  `onConfirmar: () -> Unit`). No conoce `Nucleo`, no tiene `try/catch`, no
  decide qué es un error vs. qué es un dato válido.
- Si una pantalla necesita más de ~150-200 líneas o mezcla más de un modo
  de interacción (como los 3 modos de `PantallaActivos.kt`), es la señal
  de partirla en sub-`@Composable`s más chicos — cada uno con un solo
  trabajo (la fila de un resultado, el selector de modo, el diálogo de
  confirmación), no una función gigante con `when`/`if` anidados.

### 2. `*ViewModel.kt` — dueño del estado y de las llamadas a `Nucleo`

- Una clase Kotlin normal (`androidx.lifecycle.ViewModel` — falta agregar
  la dependencia `androidx.lifecycle:lifecycle-viewmodel-compose` a
  `app/build.gradle.kts` cuando se haga el primer ViewModel real) por
  pantalla, o compartido entre pantallas que de verdad comparten estado
  (ej. la sesión activa).
- Expone el estado como `StateFlow`/`State` de solo lectura; sólo el
  ViewModel lo muta. Las llamadas a `Nucleo` corren en `viewModelScope`,
  no en un `rememberCoroutineScope()` atado al ciclo de vida del
  Composable — así sobreviven una recomposición o una rotación de
  pantalla en vez de cancelarse a medio camino.
- Aquí sí va el `try/catch`, pero **nunca sobre `Exception` genérico** —
  capturar específicamente `NucleoException` (la única excepción que
  `Nucleo` puede lanzar legítimamente por una regla de negocio) y mapearla
  a un estado de error tipado y en español para la UI. Cualquier otra
  excepción (`NullPointerException`, `IllegalStateException`, etc.) es un
  bug real y debe propagarse, no esconderse detrás de un mensaje de
  "error" genérico — así se nota en desarrollo en vez de aparecer como un
  mensaje confuso en producción.
- Testeable sin Android: un `ViewModelTest` normal de JUnit puede
  instanciar el ViewModel con un `Nucleo` de prueba (base SQLite temporal,
  mismo patrón que ya usan los tests de `rust-core/src/lib.rs`) y verificar
  el estado que produce, sin inflar UI ni tocar un emulador.

### 3. `Nucleo` (Rust vía uniffi) — la única fuente de verdad de negocio

- No se toca desde Kotlin más que para llamarlo. Ninguna regla de PRAIND,
  exclusividad de gafetes, ni fechas se reimplementa acá — eso ya está
  decidido y probado en `rust-core`/`control_acceso`, y así debe seguir
  (ver `mobile/README.md` y `docs/plan-app-movil.md`).

## Otras reglas concretas, no sólo la separación en capas

- **`rememberSaveable` en vez de `remember`** para cualquier campo de
  formulario o búsqueda en curso que el usuario esperaría no perder si el
  teléfono rota o la Activity se recrea. Una vez que el estado vive en un
  ViewModel (que ya sobrevive rotación por diseño), esto deja de ser
  necesario ahí — pero sigue aplicando a cualquier estado puramente visual
  que se quede en el Composable (ej. si un diálogo está abierto).
- **Nunca loguear ni mostrar el mensaje crudo de una excepción interna al
  usuario final.** El ViewModel traduce a un mensaje pensado para el
  guardia, no reenvía `excepcion.message` tal cual.
- **Un archivo, una responsabilidad.** Si al agregar algo un archivo ya
  hace dos cosas que no comparten una sola razón para cambiar, es momento
  de partirlo — no de agregar un parámetro más a una función que ya hace
  demasiado.
- **Comentarios sólo para el porqué, nunca para el qué.** Un nombre bien
  elegido ya dice qué hace algo; un comentario vale cuando explica una
  decisión no obvia (por qué se descartó una alternativa, una restricción
  real del dominio) — mismo criterio que ya se sigue en `rust-core` y en
  `docs/plan-app-movil.md`.

## Cómo aplicar esto sin parar el piloto

No hace falta reescribir las 7 pantallas de una sentada. La regla práctica:
**la próxima vez que se toque un archivo por cualquier motivo (un bug, una
pantalla nueva, un pedido del cliente), se extrae su ViewModel como parte
de ese mismo cambio** — no se agrega código nuevo sobre el patrón viejo.
Cada archivo de pantalla existente tiene una nota al inicio marcando esto
explícitamente (ver los propios `.kt`).

Orden sugerido si se decide hacer el refactor de una vez en vez de
incremental: `PantallaActivos.kt` primero (el más grande y el que más se
beneficia), después el resto en cualquier orden — no hay dependencias
reales entre los ViewModels de cada pantalla.

## Ejemplo ya aplicado — usar como plantilla

`PantallaActivos.kt` + `ActivosViewModel.kt` (2026-09-02) ya siguen este
patrón — úsense como plantilla concreta al extraer el resto en vez de
reinventar la forma en cada pantalla nueva:

- El estado vive en propiedades `by mutableStateOf(...)` con `private set`
  dentro del `ViewModel`; el Composable las lee como `viewModel.campo`, de
  sólo lectura desde afuera.
- Cada evento de UI (`cambiarTexto`, `elegir`, `confirmarSalida`, …) es una
  función pública del `ViewModel` que hace su propio `viewModelScope.launch`
  — el Composable nunca abre un `rememberCoroutineScope()` ni encierra un
  `try/catch` propio.
- El `catch` es siempre sobre `NucleoException`, nunca sobre `Exception` —
  ver el doc-comment de `ActivosViewModel` sobre por qué (evita tragarse un
  `CancellationException` de paso, algo que sí le pasaba a la versión
  vieja de este mismo archivo).
- El `ViewModel` se instancia en el Composable con
  `viewModel(factory = XyzViewModel.factory(nucleo))` — requiere la
  dependencia `androidx.lifecycle:lifecycle-viewmodel-compose` (ya
  agregada a `app/build.gradle.kts`; fijada en `2.9.4` porque `2.10+` pide
  `compileSdk 37` y el proyecto sigue en `36` — subir esa versión sólo si
  se sube `compileSdk` a la vez).
- Verificado compilando de verdad (`./gradlew :app:assembleDebug`), no sólo
  a ojo — cualquier refactor de este tipo debe pasar por ese mismo comando
  antes de darse por terminado.

`MainActivity.kt` (2026-09-02) es el segundo ejemplo, con un matiz nuevo:
`LoginViewModel` **no** usa `viewModelScope.launch` — `Nucleo.autenticar`
es una llamada síncrona a SQLite local, no hace falta corrutina para eso.
No copiar el `viewModelScope.launch` de `ActivosViewModel` por reflejo
donde no hace falta. También muestra que **no todo estado necesita
ViewModel**: `PantallaPrincipal` (qué pestaña se ve, si el menú "+" está
abierto) se quedó en `remember` a propósito — es navegación pura, sin
llamada a `Nucleo` ni regla de negocio detrás, así que un ViewModel ahí no
protegería nada real.

`PantallaConfirmarIngreso.kt` (2026-09-02) es el tercer ejemplo — y el
importante para no aplicar la regla en automático: **esta sí llama a
`Nucleo` y sí tiene validación real, y aun así se queda sin ViewModel**.
La razón es específica de esta app: como no usa Navigation-Compose, todos
los `viewModel()` de cualquier pantalla comparten el mismo dueño (la
Activity) — un `ViewModel` sobrevive aunque el Composable se desmonte. Acá
eso es un problema, no una ventaja: `ActivosViewModel` desmonta esta
pantalla por completo al cancelar o confirmar (vuelve a
`SeleccionIngreso.Ninguna`), así que cada vez que se entra es un intento
nuevo — un `ViewModel` sin key por intento arrastraría el error/gafete de
un contratista fallido al formulario del siguiente. `remember`/
`rememberSaveable` ya dan ese estado "fresco por entrada" gratis, que es
justo lo que hace falta acá. La regla real no es "ViewModel siempre" — es
"ViewModel cuando el estado debe sobrevivir más que el Composable actual
y no hay otro ya dueño de esa decisión (rotación, cambio de pestaña); acá
`ActivosViewModel` ya es quien decide cuándo esta pantalla vive o muere".

`PantallaNuevaEmpresa.kt`, `PantallaNuevoContratista.kt` y
`PantallaNuevoUsuario.kt` (2026-09-02) confirman la misma regla: las tres
son formularios de alta que `PantallaPrincipal` abre desde el menú "+" y
desmonta por completo al volver ("← Volver") — exactamente el mismo caso
que `PantallaConfirmarIngreso`, así que tampoco llevan ViewModel. Lo que sí
se corrigió en las tres: `catch(Exception)` → `catch(NucleoException)`, y
los campos de texto/selección a `rememberSaveable` — **con una excepción
real**: `empresaSeleccionada` (`Empresa?`) en `PantallaNuevoContratista.kt`
se quedó en `remember` porque `Empresa` es un `data class` generado por
uniffi sin `Serializable`; `rememberSaveable` sobre un tipo no guardable
falla en tiempo de ejecución, no en compilación — antes de aplicar
`rememberSaveable` a cualquier tipo que no sea `String`/`Boolean`/un enum
generado por uniffi (los enums sí son `Serializable`, heredado de
`java.lang.Enum`), confirmar que el tipo realmente se puede guardar.

`PantallaNube.kt` + `NubeViewModel.kt` (2026-09-03) — sincronización con la
nube (`docs/plan-persistencia-nube.md`) — es el sexto ejemplo y aporta dos
matices nuevos:

- **Split de autorización dentro de un mismo ViewModel.** A diferencia de
  todos los casos anteriores (donde toda la pantalla es de un rol o de
  todos), acá `Operacion::GestionarNube` (guardar/leer si hay secreto) es
  exclusivo de Root y `Operacion::UsarNube` (sincronizar, listar y cerrar
  ingresos remotos) es de cualquier rol — dos operaciones del dominio
  reales, no una preferencia de UI. `NubeViewModel` no oculta esto: expone
  los cuatro métodos tal cual, y es `PantallaNube` quien decide, con
  `sesion.rol`, qué botones dibujar — el mismo criterio que ya usa el menú
  "+" de `PantallaPrincipal` para "Nuevo usuario". La consecuencia práctica
  es que `NubeViewModel` **no llama nada en `init`**: ni siquiera comprobar
  si ya hay un secreto guardado es seguro hacerlo a ciegas, porque para un
  Operador esa llamada fallaría por permiso antes de que la pantalla
  decida no mostrarle esa sección. `actualizarEstadoSecreto()` se dispara
  desde un `LaunchedEffect(Unit) { if (esRoot) ... }` en el Composable, no
  desde el ViewModel.
- **Nube es pestaña, no pantalla detrás del "+".** Aunque a primera vista
  se parece a un formulario de alta esporádico, su estado
  (`ingresosRemotos`, `secretoGuardado`, `ultimoResumen`) tiene el mismo
  requisito que `ActivosViewModel`/`HistorialViewModel`: sobrevivir que el
  usuario cambie de pestaña y vuelva, no reiniciarse en cada entrada como
  sí necesitan `PantallaConfirmarIngreso` o los tres formularios de alta.
  Por eso entra como tercera pestaña en `PantallaPrincipal`
  (`Activos`/`Historial`/`Nube`), no como una cuarta opción del menú "+".
