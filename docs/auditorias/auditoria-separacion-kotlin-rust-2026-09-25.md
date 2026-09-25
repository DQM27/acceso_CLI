# Auditoría: separación de responsabilidades Kotlin ↔ núcleo Rust (Android)

Fecha: 2026-09-25 · Alcance: `mobile/android/app/src/main/java/com/brisas/controlacceso/*.kt`
contra `mobile/rust-core/src/lib.rs` (fachada UniFFI) y el crate raíz `src/`.

Objetivo medido: Kotlin como "cáscara" de UI y transporte; toda decisión de
dominio en Rust. Se excluye el binding generado
`uniffi/control_acceso_mobile/control_acceso_mobile.kt` (6 099 líneas).

> Nota técnica: la frontera no es JNI escrito a mano sino **UniFFI 0.32**
> (`#[uniffi::export]` sobre `Nucleo`). UniFFI genera el pegamento JNA/JNI y
> serializa los `Record`/`Enum` en un buffer; el costo por cruce sigue
> existiendo, por lo que las llamadas en bucle (ver hallazgo T‑1) cuentan.

---

## Resumen ejecutivo

- La base es sana: los ViewModels (`ActivosViewModel`, `RutasViewModel`,
  `ProveedoresViewModel`, `GafetesProvisionalesViewModel`, `LoginViewModel`,
  `NubeViewModel`, `PrimerArranqueViewModel`) son mayormente despachadores:
  llaman a `Nucleo`, guardan el resultado y muestran `excepcion.message`. Las
  reglas pesadas (acceso, PRAIND, gafete requerido, placa, ingreso activo,
  fecha de documento de ruta) ya viven en Rust.
- Hay **tres focos de fuga** de lógica de dominio hacia Kotlin:
  1. **Reglas duplicadas o exclusivas de Kotlin** en formularios que no usan
     ViewModel (`PantallaNuevoContratista`, `PantallaConfirmarIngreso`).
     Una de ellas — **bloquear el alta con PRAIND vencido** — existe *sólo*
     en Kotlin; Rust la acepta.
  2. **Orquestación de flujos de negocio** en Kotlin: "¿gafete ocupado en el
     sitio? → si no, registrar", "¿activo en otro sitio? → bloquear",
     fusión local+remota de activos, salida masiva por gafetes.
  3. **Todo el reconocimiento de documentos (OCR/MRZ)** — ~710 líneas de
     lógica: clasificación, extracción por regex, checksums MRZ, vigencia,
     mayoría de edad. Es lógica de dominio pura (texto → datos) que hoy no
     comparte ni iOS ni desktop.

---

## 1. Lógica condicional de negocio en Kotlin

| # | Archivo:línea | Decisión que toma | Veredicto |
|---|---|---|---|
| C‑1 | `PantallaConfirmarIngreso.kt:61` `puedeContinuar` | Combina `tieneIngresoActivo`, `activoEnOtroSitio` y `ResultadoAcceso.Denegado` para decidir si se deja continuar. | **Mover a Rust**: que `PreparacionIngreso` traiga un `bloqueo: Option<MotivoBloqueo>` (o `puede_continuar: bool`) ya resuelto. Hoy la misma regla está también en `desktop/src/api/ingresos.ts` (duplicación triple). |
| C‑2 | `PantallaConfirmarIngreso.kt:66` `mensajeBloqueo`, `:107` `mensajeMotivoDenegacion` | Prioriza motivos y redacta el texto. | **Mover a Rust** (`src/mensajes.rs` ya tiene `mensaje_ingreso`); con C‑1 resuelto, Kotlin sólo muestra `bloqueo.mensaje`. |
| C‑3 | `ActivosViewModel.kt:303‑311` (`elegir`) | "Sólo si los chequeos locales pasaron, consultar si está activo en otro sitio" y luego parchear `preparacion.copy(activoEnOtroSitio=…)`. | **Mover a Rust**: `preparar_ingreso_con_secreto(secreto, id)` que haga el chequeo cruzado internamente (mejor esfuerzo, igual que `comandos/ingresos.rs` en desktop). Kotlin hace hoy 2‑3 cruces UniFFI + la decisión. |
| C‑4 | `PantallaActivos.kt:599‑633` (`FilaContratista`) | Decide qué etiqueta roja mostrar: `!tieneAcceso` → "ACCESO DENEGADO", si no, compara `fechaVencimientoPraind < LocalDate.now()` → "PRAIND VENCIDO". | **Mover a Rust**: `ContratistaResumen.estado_acceso: Option<MotivoDenegacion>` calculado con `domain::acceso`. Hoy Kotlin reimplementa parcialmente `verificar_acceso` con el reloj del teléfono (ignora empresa inactiva, `PraindNoRegistrado`). |
| C‑5 | `PantallaNuevoContratista.kt:63` `requierePraind` | `personalRuta || tipo == PRAIND || tipo == IN_HOUSE`. | **Duplicado exacto** de `src/domain/contratista.rs:8`. Exponer `requiere_praind(tipo, es_personal_ruta)` vía UniFFI (función libre) o un `Record` de "reglas del formulario". |
| C‑6 | `PantallaNuevoContratista.kt:392` `praindVencido` + `:453` | Bloquea el guardado si el PRAIND está vencido. | **Regla sólo en Kotlin** — `ContratistaService::construir_*` (`src/services/contratista_service.rs:248`) sólo exige que haya fecha, no que esté vigente. Decidir la regla y moverla a Rust (`ContratistaServiceError::PraindVencido`) para que desktop/web/CLI la respeten igual. |
| C‑7 | `PantallaNuevoContratista.kt:357‑379` | `POR_CORREO`/`SWAT` ⇒ `personalRuta = false` y se oculta el check. | Regla de dominio ("personal de ruta sólo aplica a PRAIND/IN HOUSE"). **Mover a Rust** como validación en `Contratista::nuevo` (normalizar o rechazar). Ocultar el check sí es UI. |
| C‑8 | `PantallaNuevoContratista.kt:471` `tieneAcceso = true` | Política "todo alta móvil tiene acceso". | Aceptable como decisión de producto del canal móvil, pero mejor como parámetro implícito de un `crear_contratista_movil` en la fachada para que no dependa de que la UI lo recuerde. Prioridad baja. |
| C‑9 | `PantallaConfirmarIngreso.kt:103` `placaSiCorresponde`, `:217‑233`, `:380‑402` | Placa sólo si `VEHICULO`; gafete requerido/válido; placa obligatoria. | **Duplicado** de `RegistroIngresoServiceError::{GafeteRequerido, PlacaRequerida, PlacaNoAplica}` (`src/services/registro_ingreso_service.rs:264‑295`). Como *pre‑chequeo de UX* para habilitar el botón es tolerable; el problema es que los textos ("El gafete es requerido", "Ingrese un número de gafete válido") están copiados. Propuesta: `Nucleo::validar_formulario_ingreso(...) -> Vec<ErrorCampo>` barato y sin efectos. |
| C‑10 | `PantallaRutas.kt:146` `fechaVencida` | Compara la fecha del documento (texto DD‑MM‑AAAA) con hoy para avisar "requiere correo". | **Duplicado** de `domain/resultado_salida_ruta.rs:25 verificar_fecha_documento`. Exponer una función pura en la fachada y usar su resultado. |
| C‑11 | `ActivosViewModel.kt:469` `contratistaEscaneadoClaro`, `RutasViewModel.kt:152/197/244` | Heurística de "coincidencia única y exacta ⇒ autoseleccionar" tras escanear. | Límite difuso. Es una decisión de flujo, pero depende de reglas de igualdad de cédula/placa. Recomendado: `buscar_*_exacto(texto) -> Option<T>` en Rust. Prioridad media‑baja. |
| C‑12 | `PantallaPrincipal.kt:121` `resumen.sesionExpulsada` → cerrar sesión | Reacciona a un flag que Rust calculó. | **Legítimo** (Kotlin sólo reacciona). |

## 2. Validaciones hechas en Kotlin

| # | Archivo:línea | Qué valida / normaliza | Veredicto |
|---|---|---|---|
| V‑1 | `ProveedoresViewModel.kt:420‑430` `cambiarCedula` + `:614` | Pre‑chequea contra la lista local `activos` si la cédula ya tiene ingreso de proveedor; copia literal del texto de `IngresoProveedorServiceError::IngresoActivo`. | **Duplicado** (lo admite el propio comentario). Reemplazar por `Nucleo::proveedor_con_ingreso_activo(cedula) -> Option<String>` que devuelva el mensaje real. |
| V‑2 | `ProveedoresViewModel.kt:437, 445, 476` | `placa.uppercase()`, nombre de empresa proveedora `trim().uppercase()`. | **Regla sólo en Kotlin**: Rust no normaliza a mayúscula (`empresa_proveedor_service`), así que desktop/web pueden crear la misma empresa en minúscula. Mover la normalización al servicio. |
| V‑3 | `PantallaNuevoContratista.kt:189, 269, 284` | Nombre `uppercase()`, cédula sólo dígitos. | Mover la normalización a `ContratistaService` (el filtro de teclado puede quedar como máscara de UI). |
| V‑4 | `ActivosViewModel.kt:85‑95` `sanearGafetesTexto` / `gafetesDeTexto` | Parseo "2, 25, 85" → lista de enteros. | **Duplicado** de `sanearGafetes`/`gafetesDe` en `desktop/src/api/ingresos.ts`. Mover a Rust junto con T‑1. |
| V‑5 | `ActivosViewModel.kt:410`, `PantallaConfirmarIngreso.kt:217` | Extrae dígitos del gafete escaneado. | Queda resuelto si el lector de gafete (O‑1) pasa a Rust y devuelve `i64`. |
| V‑6 | `PantallaNuevoContratista.kt:84‑106`, `PantallaRutas.kt:338` | Conversión DD‑MM‑AAAA ↔ ISO (duplicada en dos archivos). | Formato de presentación: puede quedarse en Kotlin, pero **unificar** en `Tiempo.kt`. Si Rust aceptara DD‑MM‑AAAA directamente, desaparece. |
| V‑7 | `LoginViewModel.kt:273`, `PrimerArranqueViewModel.kt:33`, `ProveedoresViewModel.kt:511` | `isBlank()` antes de llamar. | **Legítimo** como guarda de UX (Rust valida de nuevo). |

## 3. Estado de negocio en Kotlin

| # | Archivo:línea | Estado | Veredicto |
|---|---|---|---|
| E‑1 | `ActivosViewModel.kt:242, 261`, `ProveedoresViewModel.kt:409`, `GafetesProvisionalesViewModel.kt:83` | Construye la lista "activos" fusionando locales + remotos (`FilaActiva`, `FilaProveedorActiva`, `FilaGafeteProvisionalActiva`). | **Mover a Rust**: la unión local/remota es un concepto de dominio (qué está "adentro" del sitio). Un `listar_activos_sitio(filtro) -> Vec<FilaActiva>` con enum `Origen{Local(id), Remota(uuid)}` elimina 3 sealed classes y 3 fusiones. |
| E‑2 | `ActivosViewModel.kt:260`, `PantallaProveedores.kt:80‑89, 186` | Filtra remotos/proveedores por texto (`contains(ignoreCase)`) en Kotlin, mientras los locales se filtran en SQL. | **Inconsistente**: los remotos no pasan por la normalización/`LIKE` de Rust (acentos, cédula). Resolver con E‑1. |
| E‑3 | `ActivosViewModel.kt:127` `coincidenciasGafete` | Mapa gafete → activo construido en Kotlin. | Se resuelve con T‑1. |
| E‑4 | `LoginViewModel.kt:261` `cambioObligatorio` guarda la contraseña temporal en memoria Kotlin para reenviarla. | Sensible. Rust ya cachea la sesión Supabase; preferible que `Nucleo` retenga el estado "cambio pendiente" y `completar_cambio_obligatorio(nueva)` no requiera reenviar la temporal desde Kotlin. |
| E‑5 | `texto*`, `seleccion*`, `cargando`, `error`, `mensaje`, `registrando`, `modo` en todos los VM | Estado de UI/formulario. | **Legítimo.** |

## 4. Superficie FFI: llamadas Kotlin → Rust

Leyenda: **Antes/Después** = líneas de lógica *de dominio* (no try/catch, no
asignar a estado ni `withContext`) alrededor de la llamada.

| Función `Nucleo` | Llamada desde | Antes | Después | Comentario |
|---|---|---|---|---|
| `abrir` | `AplicacionViewModel.kt` | 0 | 0 | Ciclo de vida. OK. |
| `requiereConfiguracionInicial` | `AplicacionViewModel.kt` | 0 | 0 | OK. |
| `configurarDispositivoInicialConSecreto` | `PrimerArranqueViewModel.kt:44` | 1 (guardar secreto primero) | 0 | OK (orden de persistencia = infraestructura). |
| `autenticarConSecreto` | `LoginViewModel.kt:281` | 1 | 2 (`debeCambiarPassword`) | OK. |
| `cambiarPasswordSupabase` | `LoginViewModel.kt:334` | 0 | 0 | Ver E‑4. |
| `cerrarSesion` | `LoginViewModel.kt:358` | 0 | 0 | OK. |
| `sincronizarConNubeConSecreto` | `LoginViewModel`, `NubeViewModel`, `SincronizacionPeriodica` | 1 | 1 (`sesionExpulsada`) | OK. |
| `sesionRealtimeNubeConSecreto` | `NubeRealtime.kt` | 0 | 0 | Transporte. OK. |
| `buscarContratistas` | `ActivosViewModel.kt:186, 245` | 0 | 3 (C‑11) | |
| `prepararIngreso` | `ActivosViewModel.kt:296` | 0 | **~8** (C‑1, C‑2, C‑3) | Principal candidato. |
| `contratistaActivoEnOtroSitioConSecreto` | `ActivosViewModel.kt:307` | **2** | 1 | Absorber en `prepararIngreso` (C‑3). |
| `gafeteOcupadoEnSitioConSecreto` | `PantallaConfirmarIngreso.kt:192` | 2 | 1 (lanza excepción propia) | Ver T‑2. |
| `registrarIngreso` | `PantallaConfirmarIngreso.kt` | **~10** (C‑9) | 0 | |
| `listarIngresosActivos` | `ActivosViewModel.kt:239, 257, 272, 428` | 3 (V‑4) | 4 (fusión, E‑1) | **En bucle** en `:270` (T‑1). |
| `listarIngresosRemotos` | `ActivosViewModel.kt:223` | 0 | 2 (filtro E‑2) | |
| `registrarSalida` | `ActivosViewModel.kt:350, 387, 435` | 2 | **~6** (armado de mensaje agregado) | **En bucle** en `:380` (T‑1). |
| `cerrarIngresoRemotoConSecreto` | `ActivosViewModel.kt:354` | 1 | 0 | OK (despacho por variante). |
| `buscarEncargadosRuta` / `buscarRutas` / `buscarVehiculosRuta` | `RutasViewModel`, `GafetesProvisionalesViewModel` | 0 | 0–2 (C‑11) | |
| `registrarSalidaRuta` | `RutasViewModel.kt:257` | ~3 (C‑10, formato fecha) | 0 | |
| `registrarRetornoRuta` / `listarRutasActivas` | `RutasViewModel` | 0 | 0 | OK. |
| `listarGafetesProvisionalesActivos` + `listarPrestamosGafeteProvisionalRemotos` | `GafetesProvisionalesViewModel.kt:80` | 0 | 2 (fusión E‑1) | 2 cruces → 1. |
| `gafeteProvisionalOcupadoEnSitioConSecreto` + `entregarGafeteProvisional` | `GafetesProvisionalesViewModel.kt:134‑137` | 2 | 0 | T‑2. |
| `registrarDevolucionGafeteProvisional` / `cerrarPrestamoGafeteProvisionalRemotoConSecreto` | `GafetesProvisionalesViewModel.kt:170‑174` | 1 | 0 | OK. |
| `buscarEmpresasProveedor` / `crearEmpresaProveedor` | `ProveedoresViewModel.kt:456, 480` | 2 (V‑2) | 1 (arma `EmpresaProveedor` a mano) | Que `crear_*` devuelva el `Record` completo. |
| `listarProveedoresActivos` + `listarIngresosProveedorRemotos` | `ProveedoresViewModel.kt:407` | 0 | 2 (E‑1) | |
| `gafeteDeProveedorOcupadoEnSitioConSecreto` + `registrarIngresoProveedor` | `ProveedoresViewModel.kt:522‑531` | 3 (V‑1, placa) | 0 | T‑2. |
| `registrarSalidaProveedor` / `cerrarIngresoProveedorRemotoConSecreto` | `ProveedoresViewModel.kt:584‑588` | 1 | 0 | OK. |
| `listarEmpresas` | `PantallaNuevoContratista.kt:160` | 0 | 0 | |
| `crearContratista` | `PantallaNuevoContratista.kt:461` | **~12** (C‑5…C‑8, V‑3, V‑6, emparejar empresa del carnet) | 0 | Segundo candidato. |
| `guardarSecretoDispositivo` / `secretoDispositivoGuardado` / `cargar/borrarSecretoDispositivoLegado` | `SecretoDispositivoStore.kt` | 0 | 0 | Infraestructura (Keystore). OK. |

Funciones exportadas **sin uso** desde Kotlin (candidatas a limpieza):
`autenticar`, `configurarDispositivoInicial`, `sincronizarConNube`,
`sesionRealtimeNube`, `gafeteOcupadoEnSitio`, `cerrarIngresoRemoto`,
`cerrarIngresoProveedorRemoto`, `cerrarPrestamoGafeteProvisionalRemoto`
(variantes sin `*_con_secreto`), `listarUsuarios`, `crearUsuario`,
`crearEmpresa` — verificar antes de borrar (pueden usarse en tests o iOS).

### Patrones de transporte a corregir

- **T‑1 — Llamadas en bucle.** `ActivosViewModel.kt:270` hace un
  `listarIngresosActivos` por cada gafete tecleado y `:380‑393` un
  `registrarSalida` por cada coincidencia, armando el mensaje agregado en
  Kotlin. Propuesta: `registrar_salidas_por_gafetes(Vec<i64>) ->
  Vec<ResultadoSalidaGafete>` (y `vista_previa_gafetes(texto)` que además
  absorba V‑4). Un cruce en vez de N, y la política de "éxito parcial" queda
  en Rust.
- **T‑2 — "Chequear y luego escribir" desde Kotlin** en tres lugares
  (`PantallaConfirmarIngreso`, `GafetesProvisionalesViewModel.entregar`,
  `ProveedoresViewModel.registrarIngreso`), cada uno con su
  `GafeteOcupadoEnSitioException` Kotlin. Es la misma regla ("el gafete no
  puede estar activo en otro dispositivo del sitio") aplicada tres veces.
  Propuesta: `registrar_ingreso_con_secreto`, `entregar_gafete_provisional_con_secreto`,
  `registrar_ingreso_proveedor_con_secreto` que hagan el chequeo en vivo y
  devuelvan `NucleoError::GafeteOcupadoEnSitio`.

## 5. Duplicación Kotlin ↔ Rust (y ↔ desktop)

| Regla | Kotlin | Rust | Otros |
|---|---|---|---|
| `requiere_praind` | `PantallaNuevoContratista.kt:63` | `src/domain/contratista.rs:8` | — |
| Bloqueo de ingreso / mensaje | `PantallaConfirmarIngreso.kt:61‑120` | `src/domain/acceso.rs`, `src/mensajes.rs` | `desktop/src/api/ingresos.ts` |
| PRAIND vencido (fila de búsqueda) | `PantallaActivos.kt:613` | `src/domain/acceso.rs:46` | — |
| Gafete requerido / placa según medio | `PantallaConfirmarIngreso.kt:103, 380‑402` | `registro_ingreso_service.rs:264‑295` | `NuevoIngresoModal.tsx` |
| Fecha de documento de ruta ≠ hoy | `PantallaRutas.kt:146` | `domain/resultado_salida_ruta.rs:25` | — |
| Cédula con ingreso de proveedor activo | `ProveedoresViewModel.kt:422, 614` | `ingreso_proveedor_service.rs:81` | — |
| Parseo de lista de gafetes | `ActivosViewModel.kt:85‑95` | — | `desktop/src/api/ingresos.ts` |
| Fecha DD‑MM‑AAAA ↔ ISO | `PantallaNuevoContratista.kt:84`, `PantallaRutas.kt:338` | — | `desktop/src/tiempo.ts` |

## 6. Módulo OCR / documentos (el bloque más grande)

| Archivo | Líneas de lógica aprox. | Qué decide |
|---|---|---|
| `MrzParser.kt` | ~200 | Dígito verificador 7‑3‑1, TD1/TD3, siglo de fechas (`anioCompleto`, `:57`), separación de nombres. |
| `LectorDocumentosIdentidad.kt` | ~330 | `clasificarTipoDocumento` (`:201`), 20+ regex por tipo, `estaVencida` (`:103`), **`reclasificarPorEdad` (`:133`, regla de mayoría de edad = 18)**. |
| `EstabilizadorLectura.kt` | ~150 | Qué tipo de documento es válido para cada modo (`:238`), cuándo se confirma, mensaje "DOCUMENTO VENCIDO" (`:170`). |
| `LectorCarnetKof.kt`, `LectorComprobanteRuta.kt`, `LectorVehiculoRuta.kt` | ~200 | Reconocer carnet KOF, comprobante de carga, formatos de placa CR. |

Todo esto es **texto → datos de dominio**, sin dependencia de Android
(ya se testea en JVM puro: `MrzParserTest`, `LectorDocumentosIdentidadTest`,
etc.). Recomendación:

- **Mover a Rust** (`src/documentos/` o un crate `lectura_documentos`) y
  exponer `leer_documento(texto: String, modo) -> ResultadoLectura` y
  `procesar_frame(estado, texto)` (o un objeto `Estabilizador` UniFFI).
  Ganancias: iOS (hoy vacío) lo obtiene gratis, desktop podría leer
  documentos con la misma lógica, las reglas de vigencia/mayoría de edad
  quedan junto al resto del dominio y con el mismo reloj que usa el núcleo.
- **Queda en Kotlin**: CameraX, ML Kit (la llamada que produce el texto),
  `RecorteImagenOcr.kt` / `construirNv21` (manejo de buffers de imagen),
  vibración, colores del marco.
- Costo FFI: 1 cruce por frame con un `String` de algunos cientos de bytes —
  despreciable frente al propio OCR.

## 7. Lo que sí es legítimamente de Kotlin

`MainActivity`, `AplicacionViewModel` (abrir/cerrar `Nucleo`),
`SecretoDispositivoStore` (Android Keystore), `MetadatosDispositivoLocal`,
`SincronizacionPeriodica` / `NubeRealtime` / `CambiosNube` (programación del
pulso y reconexión; la sincronización en sí es Rust), `GestorTema`, `Tema`,
`DisenoMovil`, `ControlesBrisas`, `MarcoGuiaCedula`, `RecorteImagenOcr`,
navegación en `PantallaPrincipal` (doble "atrás", pestañas), permisos de
cámara, debounce de búsquedas y cancelación de `Job`s, `Mutex` de
mutaciones (serializar escrituras desde la UI).

---

## Resumen cuantitativo

Líneas no vacías ni de comentario, sin tests ni código generado:

| Capa | Líneas | De ellas, lógica de dominio |
|---|---|---|
| Rust (`src/` + `mobile/rust-core/src/lib.rs`) | ~22 000 | ~22 000 (núcleo completo, incluye nube/sync) |
| Kotlin escrito a mano | ~7 500 | **~900**: ~710 OCR/MRZ + ~190 reglas en VM/pantallas |

**Del total de lógica de dominio del proyecto, ≈ 96 % vive en Rust y ≈ 4 %
en Kotlin.** Si se excluye el OCR (que hoy nunca estuvo en Rust), las reglas
de negocio "clásicas" filtradas a Kotlin son ≈ 1 %: pocas, pero justamente
en el punto más sensible (decidir si alguien entra).

## Lista priorizada de qué mover

1. **C‑6 · PRAIND vencido en el alta** — regla que sólo existe en Kotlin.
   Decidir si es regla de negocio y, si lo es, agregarla a `ContratistaService`
   (afecta desktop/web/CLI). *Riesgo de inconsistencia real hoy.*
2. **V‑2 · Mayúsculas en empresa proveedora (y placa)** — otra regla sólo en
   Kotlin; mover la normalización al servicio para evitar duplicados por
   capitalización entre canales.
3. **C‑1/C‑2/C‑3 · Decisión de bloqueo de ingreso** — `PreparacionIngreso`
   con `bloqueo` ya resuelto y chequeo cruzado entre sitios adentro de Rust.
   Elimina `puedeContinuar`/`mensajeBloqueo` aquí y en desktop.
4. **T‑2 · Chequeo "gafete ocupado en sitio" + escritura** — tres funciones
   `*_con_secreto` en la fachada; borra tres copias del flujo y la excepción
   Kotlin.
5. **E‑1/E‑2 · Fusión local+remota y filtrado** — un listado unificado por
   entidad desde Rust; corrige además la búsqueda inconsistente de remotos.
6. **T‑1/V‑4 · Salida por gafetes en lote** — un solo cruce FFI, política de
   éxito parcial en Rust.
7. **C‑4/C‑5/C‑7/C‑10/V‑1 · Reglas duplicadas de presentación** —
   `estado_acceso` en `ContratistaResumen`, `requiere_praind` y
   `verificar_fecha_documento` expuestas, chequeo de proveedor activo.
8. **Módulo OCR/MRZ a Rust** — el bloque más grande, pero hoy bien aislado
   y testeado; conviene hacerlo cuando se active iOS o se quiera lectura de
   documentos en desktop.
9. **Limpieza** — funciones exportadas sin uso, E‑4 (contraseña temporal en
   memoria Kotlin), unificar formato de fechas (V‑6).
