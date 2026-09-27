# Auditoría app móvil: separación de responsabilidades (2026-09-27)

Documento pensado para que **un agente lo aplique** tal cual. Cada punto
trae: problema, archivos, qué hacer, criterio de terminado.

Código: `mobile/android/app/src/main/java/com/brisas/controlacceso/`
(47 archivos, ~10.700 líneas). Tests JVM:
`mobile/android/app/src/test/java/com/brisas/controlacceso/`.

## Reglas para el agente

- Leer `AGENTS.md` primero y respetarlo (español; nunca usar el término
  prohibido para el puesto de control: usar "puesto de control",
  "portería" o "punto de acceso"; commit bien documentado + push por cada
  cambio exitoso).
- **Nunca** tocar Supabase de producción (`control-acceso-nube`). Esta
  auditoría es sólo de código Kotlin; no hace falta ninguna base remota.
- **Refactor sin cambio de comportamiento.** Mismos textos, mismos
  mensajes de error, mismo orden de pasos. Si algo parece un bug, anotarlo
  aparte; no arreglarlo mezclado con el refactor.
- Un punto = un commit (o más), en el orden de la sección "Orden".
- Seguir las convenciones existentes (ver "Patrón a copiar").

## Diagnóstico general

La base está bien: la lógica de negocio vive en Rust (`Nucleo`, vía
UniFFI) y casi todas las pantallas tienen ViewModel con estado
`mutableStateOf` + `private set`, `dispatcherIO` inyectable y tests con
`NucleoDePrueba`. Los problemas son excepciones a ese patrón y
repetición.

| Pantalla | Llama a `Nucleo` directo | `Dispatchers` en la UI | Estados en la UI |
|---|---|---|---|
| `PantallaNuevoContratista.kt` | sí (5) | sí (4) | 15 |
| `PantallaConfirmarIngreso.kt` | sí (3) | sí (2) | 7 |
| resto | no | no | sólo estado de UI (menús, diálogos) |

## Patrón a copiar

Referencias: `RutasViewModel.kt`, `ActivosViewModel.kt`,
`RutasViewModelTest.kt`, `ActivosViewModelTest.kt`, `NucleoDePrueba.kt`,
`SecretoDispositivoStoreDePrueba.kt`.

```kotlin
class XViewModel(
    private val nucleo: Nucleo,
    private val secretoStore: SecretoDispositivoStore,          // si hace falta
    private val dispatcherIO: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    var error by mutableStateOf<String?>(null)
        private set
    // ...
    companion object {
        fun factory(nucleo: Nucleo, secretoStore: SecretoDispositivoStore): ViewModelProvider.Factory =
            viewModelFactory { initializer { XViewModel(nucleo, secretoStore) } }
    }
}
```

Tests: `StandardTestDispatcher` compartido entre `Dispatchers.setMain` y
`dispatcherIO`, `runTest`, `NucleoDePrueba.abrir(tmp, ...seedSql)`.

## Hallazgos

### M1 (alta): `PantallaNuevoContratista` sin ViewModel

- Llama a `nucleo.listarEmpresas()`, `nucleo.crearContratista(...)` y
  `nucleo.requierePraindParaFormulario(...)` desde el `@Composable`, con
  `withContext(Dispatchers.IO)`, `try/catch NucleoException` y
  `withContext(NonCancellable) { CambiosNube.solicitar() }` en la UI.
- La regla de "PRAIND vencido" (`praindVencido`, ~línea 390) es lógica de
  negocio en la vista.
- 15 `mutableStateOf` (cédula, nombre, empresa, tipo, fecha, error,
  mensaje, enviando...).

**Hacer:** crear `NuevoContratistaViewModel` con: carga de empresas,
estado del formulario (campos + `error`/`mensaje`/`enviando`),
`requierePraind`, `praindVencido`, `guardar()` (incluye
`CambiosNube.solicitar()` en `NonCancellable`, como hoy). En la pantalla
quedan sólo `menuEmpresaAbierto`, `menuTipoAbierto`, `escaneando` y el
scroll. Campos que hoy usan `rememberSaveable` → `SavedStateHandle` en el
ViewModel (o mantenerlos con el mismo comportamiento ante rotación).

**Terminado cuando:** la pantalla no importa `Nucleo`/`NucleoException`/
`Dispatchers`; existe `NuevoContratistaViewModelTest` que cubre: carga de
empresas, validación PRAIND vencido/no requerido, guardado exitoso y
error del núcleo.

### M2 (alta): `PantallaConfirmarIngreso` registra el ingreso ella misma

- Lee el secreto, llama a `nucleo.registrarIngresoConSecreto(...)` y
  atrapa `NucleoException`, `SecretoDispositivoStoreException` y
  `SecretoDispositivoNoEncontradoException` (~líneas 145-200).
- Recibe 3 ViewModels (`ActivosViewModel`, `GafetesProvisionalesViewModel`,
  `ProveedoresViewModel`).

**Hacer:** mover el registro a un `ConfirmarIngresoViewModel` (o a
`ActivosViewModel` si sólo se usa desde Activos: revisar llamadores).
La pantalla recibe estado + callbacks (`onRegistrar`, `onRegistrado`).
Revisar si de verdad necesita los 3 ViewModels o sólo datos puntuales;
pasar datos, no ViewModels.

**Terminado cuando:** la pantalla no importa `Nucleo`, `Dispatchers` ni
las excepciones del secreto; test del ViewModel cubre registro exitoso,
gafete ocupado (`NucleoException`) y secreto ausente.

### M3 (media): manejo de errores repetido en todos los ViewModels

Mismo bloque copiado ~45 veces (Activos 11, Proveedores 9, Rutas 9,
KOF 8, Login 3, Nube 3, PrimerArranque 2):

```kotlin
} catch (excepcion: NucleoException) { error = excepcion.message
} catch (excepcion: SecretoDispositivoNoEncontradoException) { error = excepcion.message
} catch (excepcion: SecretoDispositivoStoreException) { error = excepcion.message }
```

**Hacer:** un helper en un archivo nuevo (p. ej. `OperacionNucleo.kt`):
`suspend fun <T> ejecutarNucleo(dispatcher, bloque): Result<T>` (o
`mensajeDeError(Throwable): String?`) que convierta **sólo** esas 3
excepciones; cualquier otra se relanza (nunca atrapar `Exception` ni
`CancellationException`). Una variante que lea el secreto del store antes
de llamar. Reemplazar uso por uso sin cambiar mensajes.

**Terminado cuando:** los tests existentes siguen pasando sin tocarlos;
test unitario del helper (convierte las 3, relanza otras, no traga
cancelación).

### M4 (media, **ver dependencia OCR**): `PantallaEscanearCedula` mezcla cámara y lógica OCR

750 líneas: además de la vista contiene `analizarCedula`,
`extraerCedulaDeTexto`, `construirAnalizadorOcr`, `iniciarCamara`,
`recortarParaOcr`.

**Hacer:** mover las funciones no-`@Composable` a
`AnalizadorCedula.kt` (lógica pura/testeable) y `CamaraOcr.kt`
(CameraX), igual que ya existen `LectorCarnetKof.kt` y
`LectorVehiculoRuta.kt`. Mover sus tests a `AnalizadorCedulaTest`.

> **Dependencia:** las mejoras de OCR (MRZ, estabilizador, recorte NV21,
> 1080p) **no están en `main`**; están en la rama
> `claude/mejoras-sin-realtime` y tocan `PantallaEscanearCedula.kt`,
> `RecorteImagenOcr.kt`, `EstabilizadorLectura.kt` y `MrzParser.kt`.
> Hacer M4 **después** de fusionar esa rama a `main` (o partir de ella),
> si no el refactor choca con esos cambios.

### M5 (baja): navegación con strings y banderas sueltas

`PantallaPrincipal.kt` elige pestaña con `"Rutas"`, `"KOF"`,
`"Proveedores"`, `else -> Activos`, más banderas como
`mostrarNuevoContratista`.

**Hacer:** `enum class Seccion { Activos, Proveedores, Rutas, Kof }` (con
su etiqueta) y un `sealed interface Destino` para pantallas superpuestas
(nuevo contratista, confirmar ingreso, escáneres). Un solo `when`
exhaustivo. Sin agregar librería de navegación.

### M6 (baja): pantallas grandes

`PantallaProveedores.kt` (840), `PantallaRutas.kt` (752),
`PantallaActivos.kt` (645). Ya están divididas en `private fun` por
dentro; separar formularios/diálogos a archivos propios
(p. ej. `FormularioIngresoProveedor.kt`, `DialogosProveedores.kt`).
Sólo mover código, sin cambios.

### M7 (baja): tests faltantes

Sin tests: `GafetesProvisionalesViewModel`, `PrimerArranqueViewModel`,
`AplicacionViewModel`. Agregarlos con el patrón de arriba (M1 y M2 traen
los suyos).

## Orden

1. M3 (helper de errores): base para M1/M2, bajo riesgo.
2. M1, 3. M2 (con sus tests).
4. M7.
5. M5, 6. M6.
7. M4 sólo con las mejoras OCR ya en la rama base.

## Verificación (antes de cada push)

```bash
export ANDROID_HOME=/opt/android-sdk   # o el SDK local
cd mobile/android
./gradlew testDebugUnitTest            # mismo comando que ci.yml
./gradlew assembleDebug
```

Los tests JVM llaman al Rust real (compilan `mobile/rust-core` para el
host). Número de tests nunca debe bajar; hoy en `main` pasan todos.
Si se toca `mobile/rust-core` (no debería hacer falta), regenerar los
bindings UniFFI y correr `cargo test` + `cargo clippy` ahí.
