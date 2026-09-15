# Control de rutas — plan (borrador, sin código todavía)

> **Estado: planeado, en ejecución (mobile primero).** Sesión de
> planificación (2026-09-15) seguida, en la misma fecha, de la primera
> etapa de implementación -- **orden invertido a pedido explícito del
> usuario**: esta vez la app (mobile) va primero, y el núcleo real
> (schema/dominio/servicios/UniFFI) queda para el final, con la pantalla
> corriendo sobre datos mock mientras tanto, para poder mostrar un MVP
> navegable al jefe cuanto antes. No usar este orden como precedente para
> otros módulos -- es la excepción, no la regla.

## Documento real: "Comprobante de Carga de Ruta" (fotos 2026-09-15)

El usuario compartió 4 fotos de comprobantes reales (Coca-Cola FEMSA) y
aclaró exactamente qué campos importan capturar acá:

- **Ruta / No. de Carga** (ej. `CRR079/ 00001`): el número antes de la
  barra es la ruta (`CRR079`). El sub-número después de la barra indica el
  tipo -- `0001` = ruta **PRINCIPAL**; `0002`, `0003`, `0004`... son
  **H2, H3, H4** (recargas) -- cada sub-número trae su propio documento.
- **Repartidor**: se ignora como fuente de identidad -- el dato impreso no
  es confiable (lo puede retirar una persona distinta a la impresa). El
  encargado real sale del carnet KOF escaneado en persona, no de este
  comprobante.
- **Usuario**: se ignora por completo, no aporta nada acá.
- **Transporte** (código que cambia por página, ej. `700101452`-`455`):
  este es el dato clave -- es el número de documento de cada ruta/carga
  (mapea a `numero_documento` en el plan de esquema).
- **Centro, Camión (impreso), Estatus, tabla de productos**: se omiten,
  no se capturan en el flujo de registro.
- **Fecha de Entrega**: clave -- si no coincide con la fecha actual, se
  activa el bloqueo transitorio salvo correo de autorización (ya estaba
  contemplado en el diseño de abajo).

### Procedimiento real de registro (aclarado con el usuario)

1. Llega la unidad → se solicita el carnet KOF → se escanea → ahí sale el
   **encargado**.
2. Se piden los documentos de carga → ahí están **ruta/sub-número
   (tipo)**, **transporte (número de documento)** y **fecha**.
3. Se pide la **placa** (o el número de unidad) → cierra los datos del
   vehículo.

Idea del usuario a explorar en la UI: una especie de **checklist guiado
en cámara** -- 3 pasos numerados (carnet KOF → documento → placa), cada
uno con su propio campo + botón de escaneo, en vez de un único formulario
plano. Implementado como primer corte de la pantalla mobile (ver más
abajo), con captura simulada (mock) mientras no hay OCR real conectado.

### Investigación de referencias (2026-09-15) y mejoras aplicadas al MVP

Búsqueda de patrones de UX ya probados en check-in de patio/yard
management, KYC onboarding y escaneo guiado de documentos/placas, para
enriquecer la idea del checklist:

- Los sistemas de yard/gate management apuntan a un check-in supervisado
  "medido en segundos, no minutos" combinando OCR + entrada manual, y
  reportan bajar el tiempo de gate de 15-20 min a menos de 8 con
  check-in por app + reconocimiento de placa.
  ([GPX -- yard management software 2026](https://gpx.co/blog/yard-management-software/),
  [Route4Me -- plate scanner](https://support.route4me.com/route4trucks-license-plate-scanner/))
- Los flujos de onboarding KYC insisten en: **secuenciar** los pasos,
  mostrar el dato leído **editable** antes de aceptar (nunca confiar
  ciegamente en el OCR), guía visual clara y manejo de error/reintento --
  y que **70% abandona flujos que se sienten complejos**, así que menos
  pasos y menos pantallas es mejor que más precisión con más fricción.
  ([Anyline -- KYC mobile OCR](https://anyline.com/news/kyc-mobile-ocr),
  [The Skins Factory -- KYC drop-off](https://www.theskinsfactory.com/uiux-design-blog/kyc-onboarding-drop-off),
  [Maskwel -- KYC UX 2026](https://maskwelholdingsltd.com/latest-news/master-the-essentials-with-kyc-onboarding-ux-best-practices-2026-for-a-frictionless-user-experience/))
- Escaneo de carnet/placa desde el celular del guardia (sin hardware
  dedicado) es un patrón ya establecido en visitor/yard management.
  ([EvTrack Guard](https://evtrack.com/evtrack-guard-n/),
  [Lobbytrack -- scan driver license](https://www.lobbytrack.com/visitor-management/scan-driver-license))

Mejoras que esto sugirió, ya aplicadas al primer corte mobile
(`PantallaRutas.kt`):

1. **Un solo checklist en una tarjeta, no 3 pantallas** -- 3 "pasos"
   apilados con círculo numerado que se marca ✓ al completarse, todo
   visible de un vistazo.
2. **El dato "escaneado" queda editable** antes de confirmar la salida --
   el botón de cámara simulado rellena un valor de ejemplo, nunca se
   acepta a ciegas (igual patrón que ya usa el alta de contratista con
   OCR de carnet PRAIND).
3. **El bloqueo por fecha vencida vive dentro del paso 2** (no aparte):
   si la fecha no es hoy, aparece ahí mismo el aviso + checkbox "Tengo el
   correo de autorización" -- sin eso, "Confirmar salida" queda
   deshabilitado.
4. **Atajo para H2/H3/H4**: en vez de repetir los 3 pasos completos para
   cada documento adicional de una unidad que ya salió hoy, la fila de
   "rutas activas" tiene un botón "+ Documento (H)" que sólo pide
   sub-número + número de documento + fecha, y lo anexa a la salida ya
   abierta -- evita re-escanear carnet y placa, que no cambian entre
   H2/H3/H4 de una misma salida.
5. **"Confirmar retorno" de un toque** con diálogo mínimo (mismo patrón
   que `DialogoConfirmarSalida` de `PantallaActivos.kt`), lista de rutas
   activas con el mismo desvanecido superior (`ListaConDesvanecido`) que
   ya usan Activos e Historial.

## Estado de implementación (mobile, 2026-09-15)

- `mobile/android/app/.../RutasViewModel.kt`: estado en memoria (mock),
  sin `Nucleo` todavía -- `SalidaRutaActiva`/`DocumentoRuta`,
  `registrarSalida`/`agregarDocumento`/`confirmarRetorno`.
- `mobile/android/app/.../PantallaRutas.kt`: checklist de 3 pasos +
  lista de rutas activas + diálogos de agregar documento y confirmar
  retorno.
- `PantallaPrincipal.kt`: nueva pestaña **"Rutas"**, entre "Activos" e
  "Historial" (orden pedido explícitamente por el usuario: Activos →
  Rutas → Historial).
- Pendiente: todo lo de la sección "Diseño del núcleo" de este documento
  sigue sin tocar (a propósito, va al final); el OCR real (carnet KOF,
  documento, placa) tampoco está conectado -- el checklist funciona hoy
  con captura simulada y entrada manual.
- **Refinado del parser de "Transporte" (2026-09-15, tercera ronda):** el
  usuario reportó que seguía confundiendo el número de transporte con un
  código de material de la tabla. Causa: el regex aceptaba 6-15 dígitos, y
  los códigos de material de las 4 fotos reales son *siempre* de 6 dígitos
  (mismo mínimo admitido) mientras que "Transporte:" es *siempre* de 9
  (`700101452`-`455`) -- subido el mínimo a 7 dígitos, lo que excluye
  estructuralmente cualquier SKU observado. El usuario confirmó además que
  en el papel real el valor de "Transporte:" siempre queda pegado a su
  etiqueta (mismo renglón o el siguiente) y por encima del código de
  barras -- consistente con el diseño ya vigente (regex tolera como máximo
  un salto de línea entre etiqueta y valor). Fixtures de test ampliadas
  con la tabla de materiales real de las otras 3 fotos (no sólo la
  principal reutilizada por sustitución) + caso explícito de SKU pegado
  directo a "Transporte:" sin valor real (debe rechazar, no adivinar). Ver
  `LectorComprobanteRuta.kt` y `LectorComprobanteRutaTest.kt` (14 tests,
  todos verdes vía `gradlew testDebugUnitTest`).
- **Sondeo del código de barras (2026-09-15, exploratorio):** el usuario
  preguntó qué codifica el código de barras que aparece debajo de
  "Transporte:" en las 4 fotos -- no se puede saber por lectura visual
  (Code128 necesita un decodificador real, no adivinar mirando las
  barras). Conectado ML Kit Barcode Scanning (`com.google.mlkit:
  barcode-scanning:17.3.0`) **sólo en builds DEBUG**, corriendo en
  paralelo al OCR de texto en `VistaCamaraComprobanteRuta` y mostrando el
  valor crudo decodificado en el mismo overlay de debug que ya existía
  para el texto de ML Kit. Todavía no se sabe qué dato trae -- pendiente
  probarlo contra el papel real; si resulta ser el mismo "Transporte" (lo
  más probable por convención de este tipo de documento), decodificarlo
  directo sería más preciso y rápido que el regex sobre texto OCR (sin
  ambigüedad de dígitos, con checksum), y se podría evaluar reemplazar o
  complementar `REGEX_TRANSPORTE`. Camino de producción (release) no
  cambia -- sigue usando sólo el reconocedor de texto, sin el costo extra
  de correr dos detectores por frame.
- **Comparación automática texto vs. barcode (2026-09-15):** el usuario
  probó el APK debug contra el papel real y el barcode "aparentemente
  dice lo mismo" que "Transporte:". En vez de comparar a ojo dos bloques
  de texto crudo, se agregó en `VistaCamaraComprobanteRuta` una
  comparación automática entre el `numeroDocumento` ya parseado del texto
  y el `rawValue` del barcode: apenas hay un valor de cada lado, se
  muestra un aviso claro de una sola línea (✓ coincide / ✗ no coincide,
  con ambos valores si difieren) y se dispara un `Toast` una sola vez (no
  por frame). Sigue siendo sólo DEBUG -- no cambia el camino de
  producción ni reemplaza `REGEX_TRANSPORTE` todavía; falta que el
  usuario confirme el resultado exacto antes de decidir si el núcleo pasa
  a usar el barcode como fuente primaria.
- APK debug regenerado en el escritorio del usuario
  (`control-acceso-debug.apk`, sideload manual) cada vez que se toca este
  archivo -- no hay dispositivo conectado en modo debug en esta máquina,
  así que ésta es la forma de iterar con el usuario probando contra
  papel real.
- **Nota metodológica:** el usuario aclaró que había estado probando el
  sondeo de barcode apuntando la cámara a una foto en pantalla, no al
  papel físico -- eso mete artefactos (brillo, muaré, aliasing) que no
  pasan con el documento real, así que esa comparación de precisión no
  fue representativa. Pendiente repetir contra papel físico.

## Paso 1 (Gafete KOF) y paso 3 (Placa/unidad) -- activados (2026-09-15)

Se activaron los dos perfiles de OCR que quedaban simulados:

- **`LectorVehiculoRuta.kt`** (nuevo) -- un solo perfil cubre los dos
  casos del paso 3: número de unidad (calcomanía roja/blanca pegada al
  camión, ej. `22906`, **foto real** 2026-09-15) o placa (camión de
  apoyo/particular). [extraerVehiculo] prueba placa primero (patrón más
  específico) y cae a número de unidad si no hay placa reconocible.
  Formato de placa investigado por web (no de una foto real todavía):
  carga/comercial usa prefijo `C`/`CL` + dígitos (largo exacto NO
  confirmado por ninguna fuente oficial consultada -- se admite rango
  4-6, **pendiente validar contra una placa real** como se hizo con
  `REGEX_TRANSPORTE`), particular usa 3 letras + 3 dígitos (formato sí
  documentado). Fuentes: [practicatest.cr](https://practicatest.cr/blog/normativa-vial/cuales-son-las-letras-que-identifican-a-un-vehiculo-por-clase-en-costa-rica),
  [registronacional.com](https://registronacional.com/costarica/vehiculos_placa_clases.htm),
  [matriculasdelmundo.com](https://matriculasdelmundo.com/en/costa-rica.html).
- **`LectorCarnetKof.kt`** (nuevo) -- activa el perfil que estaba
  aislado a propósito en `docs/arquitectura/muestras-ocr-aisladas.md`
  ("Carnet KOF rojo" / "Carnet Coca-Cola FEMSA frontal", fotos del
  2026-09-08): nombre del frente (2 líneas), código de empleado de 7
  dígitos del reverso. A diferencia del comprobante, estos fixtures NO
  vienen de una foto fresca transcrita línea por línea -- son una
  reconstrucción razonable de esas notas guardadas, así que es un primer
  corte heurístico, más débil que el resto del perfil OCR de este
  documento. Bug real atrapado por su propio test antes de llegar al
  usuario: el teléfono de emergencia del reverso (`800-2256327`) esconde
  una corrida de 7 dígitos que un `\b\d{7}\b` suelto confirmaba como
  código de empleado -- se corrigió exigiendo que el código ocupe una
  línea completa por sí solo.
- **`PantallaEscanearCarnetKof.kt`** y **`PantallaEscanearVehiculoRuta.kt`**
  (nuevas) -- mismo esqueleto de cámara que
  `PantallaEscanearComprobanteRuta.kt` (overlay debug de texto crudo
  incluido). El carnet KOF sólo confirma con **nombre** -- si sólo se ve
  el reverso (código sin nombre) sigue esperando, porque el campo que
  llena es "Nombre del encargado" en texto libre.
- `PantallaRutas.kt`: los pasos 1 y 3 ya abren estas pantallas reales en
  vez de rellenar un valor de ejemplo.
- **Todavía sin validar contra el objeto físico real** (carnet KOF real,
  placa real) -- a diferencia del comprobante (3 rondas de ajuste contra
  el papel real), estos dos perfiles son el primer corte y muy
  probablemente necesiten ronda de ajuste igual que el comprobante
  cuando el usuario los pruebe con la cámara.
- **Corrección de largo del código de empleado (2026-09-15):** el usuario
  compartió la base real de empleados (`empleados_costa_rica.sql`, 1438
  filas, sociedad TICA) -- reveló que `numero_empleado` NO es fijo en 7
  dígitos (la única muestra de carnet vista), varía 5-7 (23 casos de 5,
  61 de 6, 1354 de 7). Corregido en `LectorCarnetKof.kt`. El archivo
  SQL quedó guardado en la raíz del repo pero **sin commitear** -- el
  harness lo bloqueó por política de datos personales; el usuario lo
  confirmó a propósito ("obvio que no puede subir a git... irá a la DB
  más tarde").

## Núcleo: primer corte (2026-09-15) -- vuelta de orden explícita

El usuario pidió parar el trabajo de mobile y pasar al núcleo ("me cansé
de estar peleando con la app mobil, vamos a trabajar al núcleo... más
técnico, más organizado") -- ya no aplica el orden invertido del inicio
de este documento para lo que sigue. Antes de escribir nada se le
preguntó explícitamente por 2 decisiones (pedido suyo: "no asumas sin
consultar"):

1. **`vehiculo_id`/`encargado_id` en `salidas_ruta`: opcionales**
   (confirmado) -- a diferencia de `contratista_id` en `registro_ingresos`
   (obligatorio, bloquea si no está en el catálogo), acá el snapshot de
   texto (`vehiculo_placa`, `encargado_nombre`) es la fuente real; el
   link al catálogo es sólo un enriquecimiento si hay match.
2. **`tipo_ruta`/H2-H4: pospuesto** (confirmado, "primero dejemos las
   rutas principales montadas") -- `salidas_ruta` es UNA fila por salida
   (un solo documento, ruta PRINCIPAL), no header+detalle todavía. El
   atajo "+ Documento (H)" que ya existe en el mock de `PantallaRutas.kt`
   no tiene tabla propia hasta que se decida esto -- migración aparte
   cuando se retome.

Modelo de referencia pedido explícitamente por el usuario:
`registro_ingresos`/`RegistroIngresoService` ("el que está más fino y
mejor ajustado"), no `movimientos_visita` (primer borrador que se probó
antes de pedir el cambio).

**Implementado y verificado (`cargo test-plano`, 248/248 -- incluye
`nube` y todo lo demás del núcleo, no sólo lo nuevo; también
`cargo test-mobile-plano`, 12/12; `cargo clippy --all-targets` limpio;
`cargo fmt` aplicado):**

- `MIGRACION_36` (`src/database/schema.rs`, `SCHEMA_VERSION = 36`):
  - `vehiculos_ruta` / `encargados_ruta`: catálogos livianos, mismo
    molde que `empresas` (sin FTS).
  - `salidas_ruta`: espejo de `registro_ingresos` -- apertura/cierre con
    un trío nullable todo-o-nada (`fecha_hora_retorno`/
    `usuario_retorno_id`/nombre), `CHECK` cronológico, `resultado`
    (`PERMITIDO`/`PERMITIDO_CON_AUTORIZACION`) + `motivo_resultado`
    correlacionado (`DOCUMENTO_FECHA_DISTINTA` -- renombrado desde
    `DOCUMENTO_FECHA_ANTERIOR` durante la implementación: la regla real
    es "no coincide con hoy", no sólo "es anterior"), 4 triggers
    (no-eliminar, apertura inmutable, cierre único, fechas UTC),
    `numero_documento` único, índice único de "un vehículo no puede
    tener dos salidas abiertas a la vez" por placa (no por id, para que
    aplique aunque no haya match de catálogo).
  - `cola_salida.entidad` CHECK ampliado con `vehiculo_ruta`,
    `encargado_ruta`, `salida_ruta` (mismo patrón que `MIGRACION_30`).
- `domain::resultado_salida_ruta`: `ResultadoSalidaRuta` (mismo molde
  que `resultado_acceso.rs`) + `verificar_fecha_documento` (la regla de
  negocio central: fecha del documento vs. hoy, con o sin correo de
  autorización) + `salida_es_cronologicamente_valida`.
- Modelos (`vehiculo_ruta.rs`, `encargado_ruta.rs`, `salida_ruta.rs`) y
  repositorios (`VehiculoRutaRepository`, `EncargadoRutaRepository`,
  `SalidaRutaRepository`, cada uno con su implementación `Sqlite*` y
  tests).
- `RutaService` (`services/ruta_service.rs`): `registrar_salida`
  (valida campos obligatorios, vehículo no duplicado en ruta activa,
  documento no duplicado, regla de fecha; hace match de catálogo por
  placa/código de empleado -- **nunca por nombre**, para no confundir
  personas homónimas), `registrar_retorno`, `listar_activas`.

**Pendiente (siguiente corte):** capa UniFFI (`mobile/rust-core/src/lib.rs`),
conectar `PantallaRutas.kt` al núcleo real (hoy sigue en memoria/mock),
reporte PDF, y todo lo que quedó explícitamente pospuesto arriba
(`tipo_ruta`/H2-H4, catálogo desde `empleados_costa_rica.sql`).

## Sync con Supabase -- primer corte (2026-09-15)

El usuario notó que sin sync mobile y escritorio no comparten datos
("nos falta la sync del backend, sino como conectamos mobile y
computadora") y preguntó si el cliente siquiera quiere estos datos en
internet -- **sin decisión del cliente todavía**, pero el usuario
confirmó explícitamente seguir el mismo patrón que el resto de la app
(contratistas/ingresos/citas-visitas ya sincronizan hoy), reversible si
el cliente después dice que no.

Antes de tocar la base real (`control-acceso-nube`, project_id
`xidaepyaljzkpbsxrqsm`) se confirmó explícitamente con el usuario que sí
procediera contra producción (no hay ambiente de prueba separado).

**Aplicado a Supabase** (migración `control_de_rutas_vehiculos_encargados_salidas`,
verificado sin hallazgos nuevos en `get_advisors` security/performance):
- `salidas_ruta`, acotada por `sitio_id` del JWT del dispositivo -- igual
  que `ingresos`/`movimientos_visita` (el registro operativo real sí es
  de un sitio puntual). RLS de 3 políticas (leer propio sitio o
  admin_global; crear/actualizar propio sitio y `tipo <> 'visor'`),
  triggers de apertura inmutable + cierre único (espejo exacto de
  `ingresos`/`movimientos_visita`, sólo con "retorno" en vez de "salida"
  para no chocar con la terminología del núcleo Rust), broadcast
  (`emitir_cambio_nube_sitio`), `updated_at`.
- `encargados_ruta`/`vehiculos_ruta`: **ambos GLOBAL** (migraciones
  `encargados_ruta_global_como_contratistas` y
  `vehiculos_ruta_global_como_contratistas`, 2026-09-15) -- el usuario
  aclaró que ni el personal KOF ni las unidades (camiones) están atados a
  un sitio: una unidad puede prestar servicio en otro sitio ("recurso
  compartido"), y placa/`numero_unidad`/`codigo_empleado` ya son
  identificadores únicos de por sí, sin riesgo de choque al hacerlos
  globales -- mismo espíritu que `contratistas`/`empresas`. Lectura/
  actualización sin restricción de sitio; el `INSERT` se queda igual,
  sigue estampando el sitio del dispositivo que crea la fila (mismo
  criterio que "crear empresas del propio sitio").
  `vehiculo_id`/`encargado_id` en `salidas_ruta` son nullable (mismo
  criterio ya confirmado del lado local).

**Rust (`src/nube/sincronizacion.rs`), con tests (252/252, clippy
limpio):** push de `vehiculo_ruta`/`encargado_ruta` (upsert simple, mismo
molde que `enviar_empresa`, `on_conflict=placa`/`codigo_empleado`) y de
`salida_ruta` (apertura + cierre, mismo molde que
`enviar_movimiento_visita`/`enviar_cierre_movimiento_visita`, "primero en
llegar gana" en el cierre). Registrado en el dispatcher de `cola_salida`
(`destino_lote`, `construir_cuerpo`, `procesar_fila_individual`).

**Pendiente (deliberadamente fuera de este corte):** pull -- todavía no
hay forma de que un dispositivo vea las rutas activas que registró OTRO
dispositivo del mismo sitio (mismo patrón que `ingresos_remotos`/
`recibir_historial_visitas_del_sitio`, pendiente de construir). Se
priorizó dejar el push funcionando primero porque hoy no existe ninguna
pantalla de escritorio que consuma el pull todavía -- no había con qué
probarlo de punta a punta. Retomar cuando exista esa pantalla o cuando
dos dispositivos mobile necesiten verse entre sí.

**Alta de vehículos/encargados (catálogo):** todavía no hay ninguna
pantalla (mobile ni desktop) para insertar filas en `vehiculos_ruta`/
`encargados_ruta` -- confirmado con el usuario que se resuelve cuando se
ataque la parte de UI, no antes. Hasta entonces el catálogo queda vacío
y el match de `RutaService` (`buscar_por_placa`/`buscar_por_codigo_empleado`)
simplemente no encuentra nada -- comportamiento esperado, no un bug (el
catálogo es consultivo, nunca bloquea).

**Import real de `encargados_ruta` (2026-09-15):** el usuario pidió subir
el catálogo de KOF directo a Supabase, aclarando explícitamente que sólo
va nombre + código de empleado -- **la cédula NO se carga** (indicación
externa que ya le habían dado). Cargado vía `execute_sql` directo (mismo
criterio ya documentado en `supabase/scripts/poblar_catalogo.sql` para
"datos que todavía no tienen la herramienta de import real" -- a
diferencia de contratistas/empresas, que sí tienen
`cargo run --example importar_catalogo_limpio`, acá no existe ese camino
todavía porque no hay un dispositivo local real desde el que correrlo en
este entorno). **1438 filas** de `empleados_costa_rica.sql`, `sitio_id`/
`dispositivo_origen_id` = "Brisas" (cualquier dispositivo real del
sitio sirve de origen, mismo criterio que gafetes), `on conflict
(codigo_empleado) do update set nombre` (idempotente, se puede volver a
correr si la fuente cambia). Verificado: `count(*) = 1438`,
`count(cedula) = 0`.

## Contexto

Brisas controla hoy el acceso de contratistas (`registro_ingresos`) y visitas
(`movimientos_visita`), pero no tiene ningún concepto de **rutas de
distribución**: camiones (algunos de la flota roja de Coca-Cola, otros de
apoyo/particulares) que **salen** del sitio con mercadería y **regresan**
horas después el mismo día. Hoy ese control se lleva en papel (un documento
físico "salida de ruta") y se avisa a central por WhatsApp con una foto/PDF
armado a mano.

El objetivo es llevar ese ciclo (salida → retorno) a la app, con el mismo
nivel de rigor que ya tiene el ciclo de ingreso/salida de contratistas
(inmutabilidad, trazabilidad de quién operó cada paso, auditoría), agregando
un reporte diario en PDF listo para imprimir y compartir por WhatsApp. Es un
módulo nuevo y aparte -- no reemplaza ni modifica el control de acceso
existente -- pero comparte el mismo núcleo Rust, así que debe seguir sus
mismos patrones en vez de inventar una capa propia.

**Orden de trabajo pedido explícitamente por el usuario: núcleo primero,
UI después.**

## Quiénes participan (aclarado con el usuario, no mezclar)

| Rol | Qué es | Ya existe hoy |
|---|---|---|
| **Personal de ruta / ayudante** | Contratista tercero que sube al camión y no vuelve por esta puerta -- por eso no se le asigna gafete (`contratistas.es_personal_ruta`) | Sí, tal cual, **no se toca** |
| **Encargado de ruta (KOF)** | Personal **interno** de Coca-Cola FEMSA, con su propio carnet permanente (no un gafete del catálogo compartido). Presenta el documento de salida | Parcial: hay un perfil de OCR ya analizado y **aislado a propósito** para este caso (`docs/arquitectura/muestras-ocr-aisladas.md`, "Carnet KOF rojo") -- nunca se activó porque no había proyecto que lo necesitara. Ahora sí. El usuario además tiene una DB propia de KOF para precargar el catálogo |
| **Vehículo** | Camión que hace la ruta. Fijo por vehículo: **número de unidad** (interno, sólo camiones rojos de la flota) + **placa** (pública, siempre). El camión que cubre una ruta puede variar día a día | Nuevo |

## Datos del ciclo (aclarados con el usuario)

- **Salida**: se pide el carnet al encargado (KOF) → se registra sus datos →
  se escanea la placa → se escanea el documento de salida (número de ruta,
  tipo, número de documento). **Todo también editable a mano** -- el OCR es
  ayuda opcional en los tres puntos, nunca obligatorio (mismo patrón que ya
  usa `CampoBusquedaActivos` en `PantallaActivos.kt` con el ícono de
  cámara al lado del campo de texto).
- **Retorno**: sólo se escanea (o se escribe) la placa o el número de
  unidad -- cierra el ciclo abierto por esa salida.
- **Tipo de ruta**: `PRINCIPAL` vs rutas de apoyo ("H"/recargas, tratadas
  por ahora como el mismo tipo no-principal hasta confirmar con los
  documentos). Las rutas de apoyo son camiones particulares -- **no tienen
  número de unidad**, sólo placa.
- **Fecha**: es global al día -- todas las rutas salen y regresan el mismo
  día, no hay que modelar viajes multi-día.
- **Bloqueo transitorio por documento vencido**: si el documento de salida
  no es de la fecha actual, el sistema debe frenar la salida por defecto,
  salvo que exista un correo de autorización -- ahí el guardia sólo marca
  "tiene correo" y puede continuar. **No hay ningún patrón parecido hoy en
  el código** (se buscó "autorizacion"/"correo"/"override" en `src/` -- el
  único `domain::autorizacion` que existe es sobre permisos de usuario,
  nada que ver). Se diseña desde cero, inspirado en cómo ya funciona
  `ResultadoAcceso::PermitidoConAdvertencia` para PRAIND por vencer.
- **Trazabilidad dual**: quién opera la salida y quién opera el retorno son
  dos usuarios distintos y ambos quedan grabados -- igual que
  `usuario_ingreso_id`/`usuario_salida_id` en `registro_ingresos`. Confirma
  una práctica estándar de la industria (segregación de responsabilidades
  "quien despacha" vs "quien recibe" en sistemas de control de patio/muelle
  -- ver fuentes abajo).
- **Reporte "salida de rutas"**: PDF con todas las rutas del día (hora de
  salida y de regreso), pensado para imprimir y mandar por WhatsApp a
  central.

## Fuera de alcance (explícitamente)

- `contratistas.es_personal_ruta` -- se queda igual, no se edita ni se
  elimina.
- El catálogo de `gafetes` (CONTRATISTA/VISITA/PROVEEDOR) -- el carnet KOF
  del encargado **no** sale de ahí, es su propio carnet permanente.
- Cualquier cambio a `usuarios`/`administradores_panel`/`anfitriones`
  (identidades ya existentes y separadas a propósito, ver
  `docs/planes-implementados/plan-control-visitas.md`).

## Diseño del núcleo (primero)

### Esquema (`src/database/schema.rs`)

Mirroring dos patrones que ya conviven en el archivo:

- **`registro_ingresos`** (línea ~1830 en la migración vigente): snapshot
  desnormalizado (nombre/empresa en texto, no sólo FK), apertura/cierre con
  un trío nullable + `CHECK` todo-o-nada, `CHECK` cronológico
  (`salida >= entrada`), `resultado`/`motivo` correlacionados con `CHECK`,
  `reglas_version` para poder auditar qué regla aplicaba al momento.
- **`movimientos_visita`** (línea ~2233): versión más liviana del mismo
  ciclo -- sin PRAIND, con `uuid` para sync, índice único parcial
  `WHERE fecha_hora_salida IS NULL` para impedir dos registros abiertos a
  la vez sobre la misma clave, y triggers que garantizan: no se borra nunca,
  los datos de apertura son inmutables, el cierre se escribe una sola vez,
  toda fecha se normaliza a UTC (`strftime` check).

Propuesta de tablas nuevas (nombres sujetos a ajuste, la forma es lo que
importa):

- **`vehiculos_ruta`**: catálogo liviano, `numero_unidad` (nullable --
  ausente en camiones particulares/apoyo), `placa` (obligatoria, única),
  `activo`. Análogo a `empresas`, no a `contratistas` (no necesita
  PRAIND ni tipo_ingreso).
- **`encargados_ruta`**: catálogo del personal KOF, precargable desde la DB
  que ya tiene el usuario (import similar a
  `contratistas_base_final_limpia_v15.sql`) -- cédula/código de empleado,
  nombre, activo. Nada de gafete propio de este sistema.
- **`salidas_ruta`**: el registro operativo, mismo espíritu que
  `movimientos_visita` pero con sus propios campos: `vehiculo_id` +
  snapshot de `placa`/`numero_unidad`, `encargado_id` + snapshot de
  nombre, `tipo_ruta` (`PRINCIPAL`/`RECARGA`, `CHECK`), `numero_ruta`,
  `numero_documento` (único), `fecha_documento`, `resultado` (`CHECK`:
  `PERMITIDO` / `PERMITIDO_CON_AUTORIZACION`) + `motivo` correlacionado
  (`DOCUMENTO_FECHA_DISTINTA`) cuando aplique, `fecha_hora_salida` +
  `usuario_salida_id`/`nombre`, `fecha_hora_retorno` +
  `usuario_retorno_id`/`nombre` (trío nullable, mismo `CHECK` todo-o-nada
  y cronológico que `registro_ingresos`), `uuid`. Índice único parcial
  sobre `placa` (o `vehiculo_id`) `WHERE fecha_hora_retorno IS NULL` --
  un vehículo no puede tener dos salidas abiertas a la vez.
- Triggers análogos a los de `movimientos_visita`: sin borrado, apertura
  inmutable, cierre una sola vez, fechas UTC.
- Si este módulo sincroniza a la nube igual que ingresos/visitas, sumar
  `salida_ruta` al `CHECK` de `cola_salida.entidad` (mismo patrón que
  `MIGRACION_30` ya usó para meter `movimiento_visita`).

### Dominio y servicios (`src/domain/`, `src/services/`)

- Nuevo enum, mismo molde que `src/domain/resultado_acceso.rs`:
  `ResultadoSalidaRuta { Permitido, PermitidoConAutorizacion }` (no hace
  falta variante "Denegado" real -- con autorización siempre se puede
  continuar; si no hay autorización el flujo ni siquiera llega a intentar
  el registro, queda bloqueado en la UI).
- Nuevo `ruta_service.rs` (mismo patrón que `gafete_service.rs`/
  `cita_service.rs`): `registrar_salida_ruta`, `registrar_retorno_ruta`,
  `listar_rutas_activas`, `buscar_vehiculo`, `buscar_encargado`.
- Repositorio nuevo bajo `src/database/repositories/` para `salidas_ruta`,
  espejo de `movimiento_visita_repository.rs`.

### Capa UniFFI (`mobile/rust-core/src/lib.rs`)

`Nucleo` es una fachada plana de métodos verbo (`registrar_ingreso`,
`registrar_salida`, `listar_ingresos_activos`, ...). Sumar análogos:
`registrar_salida_ruta`, `registrar_retorno_ruta`, `listar_rutas_activas`,
`buscar_encargados_kof`, `generar_reporte_rutas_pdf` (día actual).

### Reporte PDF + WhatsApp

Ya existe un generador de PDF en desktop:
`desktop/src-tauri/src/pdf/{mod,generador,html}.rs`, que arma un HTML y usa
`ICoreWebView2_7::PrintToPdf` de WebView2 (a propósito para no meter un
motor de tipografía nuevo). Mismo enfoque para el reporte de rutas: el
núcleo compartido arma los datos (lista de salidas del día con hora de
salida/regreso) y **cada plataforma renderiza el PDF con su propio motor
nativo**, igual que ya pasa con Historial:

- **Desktop**: reutiliza `pdf/generador.rs` con una plantilla HTML nueva.
- **Mobile**: no hay motor de impresión compartido con desktop -- usa el
  mecanismo nativo de Android (`PrintManager`/WebView) para el mismo HTML.
- **Envío por WhatsApp -- dos caminos posibles, investigado (2026-09-15)**:

  1. **Share-sheet nativo de Android** (`Intent.ACTION_SEND`,
     `type=application/pdf`): el guardia toca "Compartir" → WhatsApp →
     elige el chat/grupo de central → envía. Cero configuración, cero
     costo, cero aprobación de Meta. La única desventaja: siempre lo
     dispara una persona con el teléfono en la mano -- no se puede
     automatizar desde el escritorio ni desde un job sin nadie presente.
  2. **WhatsApp Cloud API (Meta)**: sí es técnicamente posible conectar la
     app directamente. Desde octubre de 2025 es la única vía soportada
     (la versión "on-premises" autoalojada quedó descontinuada) --
     Meta aloja todo, no hay servidor propio que mantener para el
     mensajero en sí. Pero implica:
     - Un Business Portfolio de Meta verificado + un número de WhatsApp
       Business dedicado (no puede ser el mismo número que ya use alguien
       a diario en su celular).
     - Un mensaje **proactivo** como este (nadie en central escribió
       primero) sólo puede salir como **plantilla pre-aprobada por
       Meta** (categoría "utility" probablemente, no "marketing") --
       hay que someterla a revisión antes de poder usarla, y el envío del
       PDF va como adjunto tipo `document` dentro de esa plantilla.
     - Costo por mensaje entregado (no por intento): del orden de
       centavos de dólar por envío según país/categoría -- irrelevante en
       volumen para un aviso diario, pero es un costo recurrente nuevo
       que hoy no existe.
     - Necesita un backend que llame la Graph API de Meta con un token
       -- el proyecto ya tiene `supabase/functions/` (Edge Functions), que
       es el lugar natural para alojar esa integración sin exponer el
       token en el cliente.
  - **Recomendación**: para este caso de uso (un aviso diario a una sola
    central) el share-sheet nativo alcanza y no tiene fricción de
    aprobación ni costo -- la Cloud API sólo se justifica si de verdad se
    quiere que salga solo, sin que nadie lo toque. **A decidir con el
    usuario**, no es una decisión puramente técnica.

### OCR (mobile/android)

Activar el perfil ya documentado en `docs/arquitectura/muestras-ocr-aisladas.md`
("Carnet KOF rojo"): nombre desde el frente (dos líneas), código de
empleado de 7 dígitos desde el reverso cuando esté disponible -- **no**
usar ese código interno como si fuera cédula. Sumar un nuevo modo a
`ModoEscaneoDocumento` (mismo patrón que ya separa
`DOCUMENTO_CONTRATISTA`/`GAFETE_CONTRATISTA`) para: carnet KOF, placa, y
documento de salida de ruta. El layout exacto del documento de salida (y
sus variantes) queda **pendiente de las fotos que trae el usuario** -- no
se define el parser hasta tenerlas.

## UI (después del núcleo)

- **Mobile**: nueva pantalla "Rutas", mismo esqueleto que
  `PantallaActivos.kt` (selector de modo Salida/Retorno en vez de
  Ingreso/Salida/Gafete, lista de rutas activas con `LazyColumn` +
  `HorizontalDivider`, diálogo de confirmación al cerrar). Encargo
  operativo completo (salida + retorno); administración del catálogo de
  vehículos/encargados probablemente vive sólo en desktop (a confirmar).
- **Desktop**: nueva pantalla "Rutas" con el mismo esqueleto que
  `Contratistas.tsx`/`Empresas.tsx` (`Tabla` + modal `Formulario` + cliente
  en `desktop/src/api/` + comando Tauri + servicio + repositorio),
  paridad operativa completa (catálogo, registrar salida/retorno como
  respaldo si falla el celular, historial, disparar el reporte PDF).

## Fuentes externas consultadas (buenas prácticas)

- Los sistemas de control de patio/muelle ("yard/gate management systems")
  documentan check-in/check-out estructurado, con captura automatizada
  (ANPR/QR/kiosco) **complementada** por entrada manual, nunca exclusiva
  -- confirma el requisito de OCR opcional en todo.
  ([DataDocks](https://datadocks.com/posts/yard-management-process-flow),
  [c3 Solutions](https://www.c3solutions.com/blog-c3/gate-management-system/))
- Manifiestos de despacho de flota: identificación de vehículo/conductor
  por separado del viaje, fuente única de verdad por trayecto.
  ([Back4app DOT driver log](https://www.back4app.com/templates/dot-driver-log),
  [Jotform manifest](https://www.jotform.com/form-templates/driver-load-manifest-form))
- Segregación de responsabilidades ("performer vs. verifier") y rastro de
  auditoría con id/evento/timestamp/usuario/motivo -- confirma el diseño
  de operador-salida distinto de operador-retorno, ya alineado con cómo
  este proyecto audita todo lo demás.
  ([ISO 27001 Annex A 5.3](https://iso27001.com/iso-27001-annex-a-5-3-segregation-of-duties/),
  [SG Systems audit trail](https://sgsystemsglobal.com/glossary/audit-trail-gxp/))
- WhatsApp Cloud API: única vía soportada por Meta desde oct-2025 (se
  descontinuó la versión on-premises), mensajes proactivos requieren
  plantilla pre-aprobada, cobro por mensaje entregado según categoría/país.
  ([Meta for Developers -- pricing](https://developers.facebook.com/documentation/business-messaging/whatsapp/pricing),
  [Chatarmin -- setup 2026](https://chatarmin.com/en/blog/whatsapp-cloudapi),
  [Unipile -- guía 2026](https://www.unipile.com/whatsapp-api-a-complete-guide-to-integration/))

## Pendiente antes o durante la ejecución

1. Fotos de los documentos de salida (y sus variantes) → definir el
   layout real para el parser de OCR.
2. Confirmar si "H" y "recarga" son el mismo tipo de ruta o dos distintos.
3. Elegir entre share-sheet nativo o WhatsApp Cloud API para el envío del
   reporte (ver sección de arriba) -- share-sheet es la recomendación por
   defecto salvo que se quiera envío 100% automático.
4. Confirmar el alcance exacto mobile vs. desktop para la administración
   del catálogo (vehículos/encargados): ¿sólo desktop, o también mobile?
5. Importar la DB de KOF que ya tiene el usuario como semilla de
   `encargados_ruta`. **Insumo entregado (2026-09-15):**
   `empleados_costa_rica.sql` (raíz del repo) -- 1438 empleados
   (`numero_empleado` + `nombre`, sociedad TICA, PO países agosto 2026;
   `cedula` queda `NULL`, no venía en la fuente). Import en sí sigue
   pendiente para cuando se ataque el núcleo (orden ya decidido: mobile
   primero). Este archivo ya sirvió para corregir un dato del OCR: reveló
   que `numero_empleado` NO es fijo en 7 dígitos (varía 5-7 -- ver
   `LectorCarnetKof.kt`), cosa que la única muestra de carnet vista
   (`5040017`) no dejaba ver.

## Verificación (una vez implementado)

- `cargo test` en el crate raíz (tests de schema/servicios nuevos, mismo
  estilo que los tests ya existentes de `registro_ingresos`/
  `movimientos_visita`).
- `cargo test` en `mobile/rust-core` para los métodos nuevos de `Nucleo`.
- Build real de Android vía el mismo mecanismo ya usado en esta sesión
  (`workflow_dispatch` sobre `release.yml`) para confirmar que compila de
  punta a punta antes de dar por buena la UI.
- Prueba manual del ciclo completo (salida con OCR y con entrada manual,
  bloqueo transitorio con y sin correo, retorno, generación del PDF) en
  desktop y en un APK de prueba.

## Desktop -- diseño (2026-09-15, confirmado por el usuario)

El usuario pidió parar mobile ("no vamos a unir aún la lógica del núcleo
con mobile porque aún no definimos al 100% el núcleo, vamos primero con
la parte desktop") y aplicar lo investigado en la sección de referencias
de arriba. Explorado el esqueleto real (`Contratistas.tsx`/`Empresas.tsx`
→ `Tabla`/`Modal`/`Formulario` → `api/*.ts` → comando Tauri → `mensaje_*`
→ `AppCore`/servicio) vía agente, propuesto el diseño, **confirmado**.

Dos pantallas, no una:

1. **Catálogo (Vehículos + Encargados)** -- CRUD estándar, calcado 1:1 de
   `Contratistas.tsx`/`Empresas.tsx` (tabla + modal + formulario). Sin
   filtro/paginación del lado del servidor -- mismo criterio que
   `listar_empresas` ("lista completa, filtra del lado del cliente"), los
   catálogos son chicos (KOF: 1438 filas, cabe perfecto en AG Grid
   client-side, ya es el patrón que usa Contratistas con un volumen
   similar).
2. **Rutas (operación)** -- acá se aplica lo investigado, con una
   diferencia clave respecto al mobile: el checklist guiado de 3 pasos
   tiene sentido en mobile porque hay que secuenciar *cámaras* (KYC:
   guiar el escaneo). Desktop es el respaldo cuando el celular falla --
   no hay cámara que secuenciar, es carga manual directa, así que ahí
   aplica la otra mitad de la misma investigación KYC: menos pasos gana
   sobre más precisión con más fricción (70% abandona flujos que se
   sienten complejos) -- **un solo formulario compacto**, no 3 pasos sin
   razón para tenerlos.
   - **Confirmar retorno**: a diferencia de los toggles reversibles de
     Contratistas/Empresas (sin diálogo de confirmación), cerrar una
     ruta es de un solo sentido -- la base ya lo bloquea con trigger. Modal
     de confirmación, mismo patrón que `DialogoConfirmarRetornoRuta` en
     mobile. Doble clic en la fila abre "confirmar retorno", no "editar"
     (una salida no se edita una vez creada -- coincide con que la base la
     hace inmutable).
   - Primer corte: **sólo "Rutas activas"** (lo único que el núcleo
     expone hoy vía `RutaService::listar_activas`) -- el historial queda
     para después.

**Aclaración del usuario sobre el futuro historial (2026-09-15):** cuando
se construya, NO va a ser una lista plana de eventos individuales
(salida/retorno uno por uno, como el historial de Contratistas/Ingresos)
-- va a ser **por ciclo/día completo**: se pide un día (ej. "todas las de
ayer") y trae TODAS las rutas de ese día juntas como una unidad, no una
grilla con filtro de rango que se arma fila por fila. Coincide con el
"Reporte 'salida de rutas'" PDF ya planeado (una tabla con todas las
rutas del día). Tenerlo en cuenta cuando se diseñe esa pantalla --
probablemente un selector de fecha (no de rango) que trae el día
completo de una.

**Implementación en curso** (backend Rust: `AppCore` + comandos Tauri +
`mensaje_ruta`; frontend: pantallas + API client + registro en
`App.tsx`) -- ver commits siguientes para el detalle final.

## Desktop -- implementación completa (2026-09-15)

Las dos pantallas quedaron construidas de punta a punta:

- **Núcleo** (`src/application/rutas.rs`, nuevo): fachada `AppCore` sobre
  `RutaService` -- catálogo (`crear_vehiculo_ruta`/`actualizar_vehiculo_ruta`/
  `crear_encargado_ruta`/`actualizar_encargado_ruta`, sólo actor activo) y
  operación (`registrar_salida_ruta`/`registrar_retorno_ruta`/
  `listar_rutas_activas`, transacción `Immediate` + reloj validado tomando
  el máximo entre ingresos/visitas/rutas -- mismo armazón que `citas.rs`).
  `RutaServiceError` ganó `OperadorNoAutorizado` (antes faltaba, a
  diferencia de `CitaServiceError`/`RegistroIngresoServiceError`) tras
  confirmar con el usuario que no hay restricción de rol en esta app,
  desktop ni mobile -- ese chequeo es sólo "¿la sesión sigue activa?", no
  autorización por rol. `mensaje_ruta`/`mensaje_vehiculo_ruta`/
  `mensaje_encargado_ruta` agregados a `src/mensajes.rs`.
- **Tauri** (`desktop/src-tauri/src/{dto,comandos}/rutas.rs`, nuevos): DTOs
  de frontera (`DatosVehiculoRutaEntrada`/`DatosEncargadoRutaEntrada`/
  `SolicitudSalidaRutaEntrada`) que ocultan `usuario_salida_id`/
  `fecha_hora_salida` -- el núcleo los pisa siempre con el actor/reloj de
  la transacción, nunca confía en lo que mande el webview (mismo criterio
  que `fecha_hora_entrada` en `registrar_entrada_visita`). 10 comandos
  registrados en `lib.rs`.
- **Frontend**: `api/rutas.ts` (barrel), `CatalogoRutas.tsx` (toggle
  Vehículos/Encargados, calcado de `Empresas.tsx` + el toggle de
  `Visitas.tsx`), `FormularioVehiculoRuta.tsx`/`FormularioEncargadoRuta.tsx`
  (calcados de `FormularioEmpresa.tsx`), `Rutas.tsx` + `SalidaRutaModal.tsx`
  (un solo formulario compacto, con el checkbox de autorización apareciendo
  sólo si `fecha_documento !== hoy`, espejo de `verificar_fecha_documento`
  del dominio). Registradas en `App.tsx` (secciones "Rutas" y "Catálogo
  KOF").

**Corrección respecto al diseño de arriba: sin modal de confirmación en el
retorno.** Al implementar contra el esqueleto real (no contra lo asumido
del lado mobile) se confirmó que Contratistas/Empresas/Visitas no tienen
NINGÚN patrón de "confirmar antes de cerrar un registro activo" -- el botón
"Salida" de `Visitas.tsx` (fila de "Movimientos activos") es un botón
directo de un clic, sin diálogo intermedio, aunque cerrar un movimiento
también sea irreversible del lado de la base. Registrar el retorno de una
ruta sigue ese mismo patrón real: botón "Retorno" directo por fila, sin
modal (`Rutas.tsx`). El resguardo real contra un click accidental sigue
siendo el mismo de siempre: la base bloquea un segundo retorno sobre la
misma salida (`DatabaseError::SalidaRutaNoActiva`), así que un doble click
no hace daño. `DialogoConfirmarRetornoRuta` queda como un patrón exclusivo
de mobile (ahí sí tiene sentido: la confirmación llega después de escanear
tres documentos, es el cierre de un flujo guiado largo, no un click suelto
sobre una fila de grilla).

Verificado: `cargo test-plano` (259 tests), clippy limpio (núcleo y
`desktop/src-tauri`, ambos motores por defecto y `sqlite-plano`), `tsc`,
`eslint` y `vitest run` (197 tests) sin regresiones, `npm run build`
completo.

Pendiente (sin empezar, fuera de esta ronda): historial por ciclo/día
completo (ver aclaración de arriba), `tipo_ruta`/H2-H4, reporte PDF,
decisión de entrega por WhatsApp, wiring mobile↔núcleo (UniFFI).

## Bug encontrado al probar en vivo: catálogo KOF pull faltante (2026-09-15)

Al correr la app de escritorio en modo dev (`sqlite-plano`, primera prueba
visual real) el usuario reportó que "Catálogo KOF" → Encargados aparecía
vacío, a pesar del import real de 1438 filas a Supabase (ver arriba). Causa
raíz: `vehiculos_ruta`/`encargados_ruta` sólo tenían **push** (local →
nube, `nube::sincronizacion::enviar_vehiculo_ruta`/`enviar_encargado_ruta`)
-- nunca el **pull** que trae de vuelta lo que otro dispositivo (o, como
en este caso, un `execute_sql` directo contra Supabase) ya puso ahí. Un
dispositivo que nunca creó esas filas él mismo las veía siempre vacías,
mismo bug de fondo que ya se había resuelto para contratistas/empresas/
gafetes/citas/visitas -- rutas se quedó sin su propio pull cuando se armó
el push (commit `f58a832`).

Corregido con el mismo patrón que `recibir_catalogo_del_sitio`
(contratistas/empresas/usuarios) y `recibir_citas_del_sitio`: marca de
agua incremental propia (`MIGRACION_37`, columna
`catalogo_rutas_actualizado_hasta` en `sincronizacion_estado`), sin
`sitio_id=eq...` en el `GET` (ambas tablas son globales, ver más arriba),
`ON CONFLICT(placa)`/`ON CONFLICT(codigo_empleado)` para fusionar con una
fila local existente sin duplicar (mismo criterio que `guardar_empresas`).
Nueva función `nube::recibir_catalogo_rutas_del_sitio`, llamada desde los
3 mismos puntos que ya llaman `recibir_catalogo_del_sitio`
(`AppCore::sincronizar_con_nube`/`configurar_dispositivo_inicial`/
`refrescar_catalogo_sin_sesion`) y desde
`desktop/src-tauri/src/comandos/nube.rs::intentar_sincronizacion`.
`ResumenSincronizacion` (núcleo y su espejo en `comandos/nube.rs`) ganó
`vehiculos_ruta_recibidos`/`encargados_ruta_recibidos`. El crate `mobile`
no se tocó (sigue pausado, ver arriba) -- su propio struct
`ResumenSincronizacion` es independiente, no se ve afectado.

3 tests nuevos en `nube::sincronizacion` (camino feliz, fusión sin
duplicar por placa, marca incremental en el segundo sync). Verificado:
`cargo test-plano` (267 tests en `--lib`, suite completo sin fallos),
clippy limpio (núcleo y `desktop/src-tauri`), `tsc` sin errores.

Corregir con "Sincronizar" en la pantalla, o reiniciando la app (dispara
sync automático a los 10s) trae ahora las 1438 filas KOF a cualquier
dispositivo que configure la nube.

## Catálogo de números de ruta (2026-09-15)

Al ver el modal, el usuario notó un hueco real: `numero_ruta` era texto
libre ("CRR079"), sin ningún límite -- se podía escribir cualquier cosa,
incluido un número de ruta que no existe. Pedido explícito: "hay que
hacer un catálogo de rutas igual que como con los gafetes... al final
importa el número más que eso [el prefijo]".

Aclarado con el usuario antes de modelar:
- **Formato**: entero simple (79), sin prefijo "CRR" ni ceros a la
  izquierda.
- **Alta**: por rango (como gafetes, `crear_uno`/`crear_rango`), pero con
  huecos -- "hay unas que se saltan o no aplican" -- y con flexibilidad
  para deshabilitar una ruta puntual (dar de baja, puro catálogo, sin
  "perdido" como gafetes).
- **Alcance**: por sitio (como gafetes, no global como vehículos/
  encargados) -- "a cada lugar se le asignan" -- pero el número es
  **único en toda la operación**, no por sitio: "en cartago no saquen
  una ruta de brisas". Reasignar una ruta a otro sitio es un caso raro
  ("no es lo normal") que queda como operación administrativa manual
  (`UPDATE sitio_id` directo en Supabase), sin UI propia todavía.
- **Restricción**: a diferencia de vehículo/encargado (consultivos, nunca
  bloquean), el número de ruta SÍ es bloqueante -- debe existir y estar
  activo en el catálogo, mismo criterio que `contratista_id` en
  `RegistroIngresoService`.

Implementado en todas las capas:

- **Núcleo**: `MIGRACION_38` -- tabla `rutas` (id/numero único/activo/uuid,
  sin `sitio_id` local, mismo criterio que `gafetes`) + recreación completa
  de `salidas_ruta` (`numero_ruta` pasa de `TEXT` a `INTEGER`, gana
  `ruta_id INTEGER NOT NULL REFERENCES rutas(id)`). Sin datos que
  preservar (0 filas reales, la tabla nació esta misma sesión).
  `RutaCatalogoService` (nuevo, `src/services/ruta_service.rs`) --
  `crear_uno`/`crear_rango`/`dar_de_baja` (bloqueado si hay una salida
  activa con esa ruta)/`reactivar`, mismo molde que `GafeteService`.
  `RutaService::registrar_salida` ahora resuelve y valida el número contra
  el catálogo antes de crear la salida (`RutaServiceError::RutaNoEncontrada`/
  `RutaInactiva`, nuevos). `AppCore` gana `listar_rutas`/`crear_ruta`/
  `crear_rutas_rango`/`dar_de_baja_ruta`/`reactivar_ruta`.
- **Supabase**: tabla `rutas` nueva (RLS por sitio como `gafetes` --
  select/insert/update acotados a `sitio_id` propio -- pero
  `UNIQUE(numero)` SIN `sitio_id`, para que dos sitios nunca compartan
  número). `salidas_ruta.numero_ruta` migrado de `text` a `bigint`. Push
  (`enviar_ruta`, `on_conflict=numero`) y pull (sumado al
  `recibir_catalogo_del_sitio` existente, con `sitio_id=eq...` como
  gafetes, comparte la marca general -- sin la complejidad de "portador
  pendiente" que sí tiene gafetes).
- **Escritorio**: tercera pestaña "Números de ruta" en la pantalla de
  catálogo (`CatalogoRutas.tsx`), con `FormularioRuta.tsx` (alta
  individual/por rango, calcado de `FormularioGafete.tsx`) y el toggle
  "Activo" de la grilla llamando `dar_de_baja`/`reactivar` (no un
  `actualizar` genérico, porque tiene reglas de negocio propias).
  `SalidaRutaModal.tsx`: el campo de número de ruta pasó de texto libre a
  un `<select>` poblado sólo con rutas activas -- a diferencia de
  vehículo/encargado (autocompletar libre, el catálogo es consultivo),
  acá tiene que ser imposible escribir un número inválido desde la UI.

Verificado: núcleo (286 tests en `--lib`, suite completo, clippy limpio
en núcleo y `desktop/src-tauri`), advisories de seguridad de Supabase
limpios tras la migración (se encontró y corrigió un `search_path`
mutable en la función de trigger nueva), `tsc`/`eslint`/`vitest`
(197 tests)/`npm run build` sin regresiones en el frontend.

Pendiente: el catálogo de rutas arranca vacío en cualquier base nueva
(local y Supabase) -- hay que cargar los números reales de Brisas antes
de poder probar el flujo de salida completo en la app.
