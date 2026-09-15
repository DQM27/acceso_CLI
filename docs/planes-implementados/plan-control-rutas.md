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
  (`DOCUMENTO_FECHA_ANTERIOR`) cuando aplique, `fecha_hora_salida` +
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
   `encargados_ruta`.

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
