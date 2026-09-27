# Sistema de visitas: auditoría y rediseño desde cero (2026-09-27)

Alcance: todo el sistema de visitas: núcleo Rust (`src/domain/cita.rs`,
`src/services/cita_service.rs`, `src/application/citas.rs`, repositorios,
sincronización), escritorio Tauri (`desktop/src/pantallas/Visitas.tsx`,
`VisitaCheckInModal.tsx`), web de anfitriones (`web-visitas/`) y el
esquema en Supabase (`anfitriones`, `citas`, `cita_sitios`,
`cita_visitantes`, `movimientos_visita`, RPC `crear_cita_anfitrion`).

Pedido del dueño: **el diseño actual es un MVP, no se toma como base.**
Se propone un rediseño desde cero, apoyado en cómo funcionan los sistemas
de gestión de visitantes en plantas industriales. Documento pensado para
que un agente lo implemente por fases. Las decisiones del dueño están
cerradas (ver "Decisiones tomadas", 2026-09-27).

## Decisiones tomadas (2026-09-27)

| Tema | Decisión |
|---|---|
| Alcance de una visita | **Una visita = un sitio.** El anfitrión puede elegir varios sitios en un solo paso; se crea una visita por sitio unida por `grupo_id`. |
| Duración | **Rango de fechas** (`fecha_desde`–`fecha_hasta`), que puede ser un solo día, con **ventana horaria diaria** (`hora_desde`–`hora_hasta`) que aplica a cada día del rango. |
| Tolerancia | **±60 min** sobre la ventana, configurable por sitio. |
| Gafete | **Obligatorio** en toda entrada de visita. |
| Retención | Los movimientos se guardan **para siempre**. Sin purga automática. |
| Web de anfitriones | **Rediseño total** de UX/UI (no sólo colores): ver 3.8. |


## Reglas para el agente

- Leer `AGENTS.md` (español; nunca el término prohibido para el puesto de
  control: usar "puesto de control", "portería" o "punto de acceso";
  commit documentado + push por cada cambio exitoso).
- **Nunca** tocar Supabase de producción. Migraciones al repo, prueba en
  staging (`control-acceso-staging`), producción la aplica el dueño.
- Hacer esto **después** de N1-N3 de `auditoria-nucleo-rust-2026-09-27.md`
  (sincronización única): visitas se monta sobre esa función.
- No reutilizar tablas de contratistas para visitas (decisión vigente:
  "no mezclar entidades").

---

## Parte 1: Qué tiene hoy el MVP y por qué no alcanza

### Modelo

- **No existe el visitante como entidad.** Cada `cita_visitantes` copia
  cédula/nombre/empresa/placa. La misma persona que viene 20 veces son 20
  filas sin relación: no hay historial por persona, no se la reconoce al
  volver, no se la puede restringir, no se pueden atender sus derechos
  sobre sus datos (Ley 8968).
- **Estados pobres.** La cita sólo es `VIGENTE`/`CANCELADA`; "vencida" se
  calcula. No existe "llegó", "terminó", "no se presentó", "pendiente de
  aprobación" ni "rechazada". Por persona no hay estado: sólo
  movimientos sueltos.
- **Ventana sólo por días.** `fecha_desde`/`fecha_hasta`; `hora_estimada`
  es texto informativo que ninguna regla usa. Una cita "de 9:00 a 11:00"
  permite entrar a las 23:00.
- **Una cita para varios sitios** (`cita_sitios`). Obliga a RLS con
  `EXISTS` y a que el puesto de control de un sitio cargue citas que
  también son de otros. La vigencia, el anfitrión y el motivo no pueden
  variar por sitio.
- **Anfitriones sin alcance.** `anfitriones` = correo + nombre; puede
  agendar en cualquier sitio. Se dan de alta con SQL a mano (el panel web
  no los gestiona). Sin estado activo/inactivo.

### Operación en el puesto de control

- **Sin visita no hay nada.** Regla binaria (`domain::cita::verificar_cita`):
  permitido o denegado. El caso más común de la vida real, la persona que
  llega **sin cita**, no tiene flujo: el guardia la rechaza o improvisa.
- **Sólo escritorio.** La app móvil no tiene visitas (ni el OCR de cédula,
  que ya existe para contratistas, se usa acá); el núcleo móvil ni siquiera
  baja el historial de visitas (`mobile/rust-core/src/lib.rs`, comentario
  "Sin `recibir_historial_visitas_del_sitio` a propósito").
- **El anfitrión no se entera** de que su visita llegó (la notificación
  quedó "no prioridad").
- **Sin requisitos**: ni aceptación de reglamento/consentimiento de datos,
  ni inducción de seguridad, ni EPP.
- **Sin control de permanencia**: nadie avisa si una visita sigue adentro
  pasada su hora.
- **Sin lista de restricción** de personas no autorizadas.
- **Sin vista de emergencia** consolidada ("¿quién está adentro?").

### Web de anfitriones

- Buen trabajo de detalle (validación zod compartida, RPC atómica con id
  idempotente, accesibilidad), pero el flujo es sólo "crear" y
  "cancelar". No se puede **editar** una cita, **reutilizar** visitantes
  anteriores, ver **si llegaron**, ni recibir avisos.
- Sistema de diseño propio (Bootstrap + Sass) separado del resto
  (`design/brisas.json`): dos sistemas de diseño que mantener.
- Voseo en textos ("Ingresá", "Seleccioná", "Intentá").

### Panel administrativo

- No tiene nada de visitas: ni anfitriones, ni citas, ni historial de
  visitas, ni reportes.

---

## Parte 2: Qué hace un sistema de visitas industrial

Resumen de prácticas habituales en plantas (fuentes al final):

1. **Pre-registro** por el anfitrión: propósito, fecha y **ventana
   horaria**, documentos requeridos.
2. **Clasificación** del visitante (cliente, auditor, proveedor
   ocasional, entrevista, autoridad...) para aplicar requisitos por tipo.
3. **Llegada**: identificación contra la visita y contra una **lista de
   restricción**; si no hay visita, **walk-in con aprobación del
   anfitrión**.
4. **Requisitos**: aceptación de reglamento / confidencialidad, inducción
   de seguridad vigente, EPP; registrados con fecha y versión.
5. **Gafete** con color por tipo y ventana de validez; **escolta** cuando
   el riesgo lo pide.
6. **Aviso al anfitrión** al llegar.
7. **Salida** con devolución de gafete; **alerta de permanencia** si se
   excede la ventana.
8. **Emergencia**: lista en tiempo real de quién está adentro para
   evacuación y pase de lista.
9. **Registro de consentimiento** (en Costa Rica, Ley 8968:
   consentimiento informado y expreso, derecho de acceso y
   rectificación) y política de retención.

---

## Parte 3: Rediseño

### 3.1 Modelo de datos

```
visitantes            (persona; una fila por persona real)
  id uuid, tipo_documento (CEDULA|DIMEX|PASAPORTE|OTRO), numero_documento,
  nombre, empresa, telefono?, correo?, creado_en, actualizado_en
  UNIQUE (tipo_documento, numero_documento)

restricciones_visitante   (lista de restricción)
  visitante_id, nivel (BLOQUEADO|REQUIERE_AUTORIZACION), motivo,
  desde, hasta?, creado_por (admin), creado_en

anfitriones
  id uuid, correo UNIQUE, nombre, activo, creado_en
anfitrion_sitios          (dónde puede recibir visitas)
  anfitrion_id, sitio_id

visitas                   (la autorización; UNA por sitio)
  id uuid, sitio_id, anfitrion_id, tipo_visita, motivo,
  fecha_desde, fecha_hasta,              -- rango (puede ser un día)
  hora_desde, hora_hasta,                -- ventana diaria real
  requiere_escolta bool, grupo_id?,      -- agrupa un "tour" multi-sitio
  origen (PRE_REGISTRO|WALK_IN), estado, creado_por, creado_en, actualizado_en

visita_invitados          (persona dentro de una visita)
  id uuid, visita_id, visitante_id, placa_vehiculo?, estado,
  aprobado_por?, aprobado_en?, motivo_rechazo?

movimientos_visita        (cada cruce por el puesto de control)
  id uuid, invitado_id, sitio_id, entrada_en, salida_en?,
  gafete_numero?, gafete_devuelto?, usuario_entrada_id, usuario_salida_id?,
  dispositivo_entrada_id, dispositivo_salida_id?

aceptaciones              (requisitos cumplidos, con versión)
  visitante_id, requisito (CONSENTIMIENTO_DATOS|REGLAMENTO|INDUCCION),
  version, aceptado_en, sitio_id, vence_en?

requisitos_sitio          (qué exige cada sitio y por cuánto tiempo vale)
  sitio_id, tipo_visita?, requisito, version_vigente, validez_dias
```

Claves del diseño:

- **Visitante como entidad**: historial por persona, reconocimiento al
  volver (el anfitrión elige de "mis visitantes frecuentes"), restricción,
  derechos de datos.
- **Una visita = un sitio** (decidido). Un "tour" por varias unidades
  se crea en un solo paso pero genera una visita por sitio unida por
  `grupo_id`. Ventajas: cada sitio tiene su propia ventana y estado, el
  puesto de control sólo ve lo suyo, RLS por igualdad simple (desaparece
  el patrón `EXISTS` sobre `cita_sitios`). Cambia la decisión anterior
  de citas multi-sitio; la experiencia del anfitrión no cambia.
- **Ventana horaria real** con tolerancia configurable por sitio (p. ej.
  60 min antes / 60 min después; decidido ±60 min por defecto).
- **Rango de fechas + ventana diaria** (decidido): una visita del 10 al
  12 de 8:00 a 17:00 permite entrar esos tres días dentro de esa franja
  (±tolerancia). Un solo día = `fecha_desde = fecha_hasta`.

### 3.2 Estados

Por invitado (lo que ve el puesto de control):

```
                ┌──────────── CANCELADA (anfitrión/admin)
PROGRAMADO ─────┤
   │            └──────────── NO_SE_PRESENTO (terminó el rango sin ninguna entrada)
   │ llega
   ▼
EN_SITIO ◄────► FUERA (salió; puede volver otro momento del rango)
   │ termina el rango (o el guardia marca salida definitiva)
   ▼
FINALIZADO

WALK_IN:  SOLICITADO ──► APROBADO ──► EN_SITIO ...
                    └──► RECHAZADO
```

Por visita: derivado de sus invitados (programada, en curso, finalizada,
cancelada). No se persiste lo que se puede calcular.

### 3.3 Reglas (núcleo Rust, funciones puras)

`verificar_llegada(visitante, visitas_del_dia, restricciones,
aceptaciones, requisitos, ahora, tolerancia) -> ResultadoLlegada`:

| Resultado | Qué ve el guardia |
|---|---|
| `Permitido { visita, requisitos_pendientes }` | Ficha de la persona + anfitrión + ventana; si faltan requisitos, se resuelven ahí mismo antes de registrar. |
| `SinVisita` | Botón **"Solicitar aprobación al anfitrión"** (walk-in). |
| `FueraDeVentana { visita, minutos }` | Llegó antes/después: permitir con motivo o pedir aprobación. |
| `Restringido { nivel, motivo_visible }` | `BLOQUEADO`: no pasa. `REQUIERE_AUTORIZACION`: pedir a un admin. |
| `YaEnSitio` | Ofrecer registrar salida. |
| `Cancelada` / `Rechazada` | Motivo claro. |

Reglas explícitas, con tests, versión de reglas persistida en el
movimiento (como contratistas).

### 3.4 Flujos

**Anfitrión (web):**
1. "Nueva visita": ¿cuándo y dónde? (fecha(s), sitio(s) permitidos al
   anfitrión, ventana) → ¿quiénes? (buscar en **mis visitantes**, o
   agregar nuevo) → revisar y confirmar.
2. "Mis visitas": hoy / próximas / anteriores, con estado por persona en
   vivo ("Juan Pérez llegó 9:12, gafete 7").
3. Editar ventana o invitados mientras no haya llegado nadie; cancelar.
4. Bandeja de **solicitudes walk-in**: aprobar / rechazar con un toque
   (también desde el enlace del correo).

**Puesto de control (escritorio y móvil):**
1. Escanear cédula (OCR existente) o digitar → `verificar_llegada`.
2. Ficha: foto opcional, datos, anfitrión, ventana, requisitos.
3. Requisitos pendientes: consentimiento de datos + reglamento
   (texto corto, "El visitante acepta" registrado con versión).
4. Gafete rojo **obligatorio** (decidido), escolta si
   aplica → registrar entrada → aviso automático al anfitrión.
5. Salida: escanear gafete o cédula → confirmar devolución de gafete.
6. Pantalla **"Adentro ahora"**: visitas con tiempo en sitio y alerta de
   las que exceden la ventana.

**Walk-in sin conexión**: el guardia registra "aprobado por teléfono por
<anfitrión>" con motivo obligatorio; queda marcado para revisión en el
panel.

**Administración (panel web):**
- Anfitriones: alta, sitios permitidos, activar/desactivar (fin del SQL
  a mano).
- Lista de restricción.
- Requisitos por sitio (textos y versiones).
- Historial y reportes de visitas; revisión de walk-ins sin conexión.
- Datos del visitante: consultar y corregir.

**Emergencia (todas las apps):** "Adentro ahora" consolidado por sitio
(contratistas + proveedores + visitas + rutas), disponible **sin
conexión** en el puesto de control e imprimible.

### 3.5 Notificaciones

- Llegada y solicitud walk-in: correo (Edge Function) + tiempo real en la
  web de anfitriones. WhatsApp/SMS más adelante.
- Permanencia excedida: al puesto de control y al anfitrión.

### 3.6 Datos personales

Se guardan sólo documento, nombre, empresa (y placa si aplica): datos
personales comunes, **no** datos sensibles en el sentido de la Ley 8968.
Decisión del dueño: los movimientos se conservan **para siempre**, sin
purga.

Aun así la Ley 8968 pide informar y obtener consentimiento al recoger
datos personales, así que se mantiene un requisito mínimo:

- Aviso corto de uso de datos que el guardia muestra/lee la **primera
  vez** que se registra a una persona; queda registrado en `aceptaciones`
  (versión del texto + fecha). No se repite en visitas siguientes salvo
  que cambie la versión.
- El panel permite consultar y corregir datos de un visitante.
- El dispositivo sólo guarda los visitantes con visita en su sitio en
  el rango vigente/próximo más la lista de restricción (mínimo necesario).

### 3.7 Arquitectura

- **Núcleo Rust**: módulo nuevo `visitas` (dominio puro con estados y
  reglas + servicio + repositorios), reemplaza `cita*`. Se sincroniza con
  la función única de N1 (`AlcanceSincronizacion` gana `visitas` y
  `restricciones`).
- **Offline**: el puesto de control recibe visitas de su sitio para hoy y
  los próximos días, visitantes referenciados, restricciones y requisitos.
  Walk-in necesita red (salvo el fallback telefónico).
- **Supabase**: tablas nuevas con RLS por igualdad
  (`sitio_id = sitio del JWT` para dispositivos, `anfitrion_id = mío`
  para anfitriones, `es_admin_global()` para el panel). Escrituras de
  anfitriones por RPC (atómicas e idempotentes, como hoy). Vista
  `adentro_ahora` con `security_invoker`.
- **Móvil**: nueva pestaña Visitas reutilizando el OCR de cédula.
- **Escritorio**: pantallas rehechas (ficha, adentro ahora, walk-in).
- **Web de anfitriones**: rehecha desde cero (ver 3.8). Usa los tokens
  compartidos (`design/brisas.json`) con acento rojo propio y deja
  Bootstrap + Sass. Tono: *usted* en todos los textos.

### 3.8 Web de anfitriones: rediseño de UX/UI

Usuario: persona de oficina, no técnica, que agenda pocas veces al mes,
a menudo desde el celular. Tiene que poder agendar **sin instrucciones**
y en **menos de un minuto**.

**Qué se elimina del actual:**
- Wizard de varios pasos con barra de progreso, selector de fechas con
  calendario propio y reglas de CSP alrededor, filtros de estado con
  cuatro opciones técnicas (`TODAS`/`VIGENTE`/`CANCELADA`/`VENCIDA`),
  paginación de 12 en 12.
- Textos técnicos y voseo.

**Estructura (tres pantallas y nada más):**

1. **Inicio = "Mis visitas"**
   - Arriba, un solo botón primario grande: **Agendar visita**.
   - Dos bloques: **Hoy** (tarjetas con estado en vivo por persona:
     "Esperando", "Llegó 9:12 · gafete 7", "Salió 11:40") y
     **Próximas**. Las anteriores, en un enlace "Ver historial".
   - Si hay **solicitudes de ingreso sin cita** pendientes, aparecen
     primero, en un aviso destacado con botones **Aprobar** / **Rechazar**.
   - Estado vacío: "Todavía no tiene visitas agendadas" + botón.

2. **Agendar visita** (una sola página, sin wizard, con secciones que se
   completan de arriba hacia abajo):
   - **¿Quién viene?** Buscador "Nombre o documento" que sugiere
     **visitantes anteriores** del anfitrión (un toque los agrega) o
     "Agregar persona nueva" (documento, nombre, empresa; placa sólo si
     marca "viene en vehículo"). Lista de personas como chips
     removibles. Pegar varias filas desde Excel crea varias personas.
   - **¿Cuándo?** Campo de fecha nativo "Desde" y "Hasta" (por defecto el
     mismo día) + atajos **Hoy** / **Mañana**; horario "De" / "A" con
     valores por defecto del sitio (p. ej. 8:00–17:00).
   - **¿Dónde?** Si el anfitrión tiene un solo sitio, no se pregunta. Si
     tiene varios, casillas grandes con el nombre de cada sitio.
   - **Motivo** (opcional, una línea).
   - Resumen en lenguaje natural sobre el botón: "3 personas · Brisas ·
     jueves 10 de octubre, 8:00 a 17:00" → **Agendar**.
   - Validación en línea, en español claro, sin bloquear hasta que
     intente enviar.
   - Al confirmar: pantalla de éxito con **"Agendar otra"** y **"Volver a
     mis visitas"**.

3. **Detalle de visita** (al tocar una tarjeta):
   - Personas con su estado y horas de entrada/salida.
   - Acciones: **Editar** (fechas, horario, personas; sólo mientras nadie
     haya entrado), **Cancelar visita** (confirmación con consecuencia
     clara), **Duplicar** (reagendar lo mismo otro día).

**Avisos:** correo al anfitrión cuando llega su visita y cuando hay una
solicitud sin cita (con botones Aprobar/Rechazar que funcionan desde el
correo con un enlace firmado de un solo uso). En la web, actualización en
vivo sin recargar.

**Principios visuales:**
- Mobile-first; en escritorio, columna central de ~720 px.
- Tipografía y espaciado de `design/brisas.json`; acento rojo sólo en la
  acción principal y en alertas, no decorativo.
- Estados siempre con texto + ícono, nunca sólo color.
- Controles nativos accesibles (fecha, hora, casillas) en vez de
  componentes propios; foco visible; objetivos táctiles ≥ 44 px.
- Nada de tablas: tarjetas.

**Criterios de aceptación:** prueba con 3 personas que no conocen el
sistema: agendan una visita de 2 personas para mañana en < 60 s sin
ayuda; entienden el estado de su visita de hoy sin explicación.

### 3.9 Transición desde el MVP

Visitas **no está en uso en producción**: no hay datos reales que
migrar. El modelo nuevo se crea limpio. Las tablas viejas (`citas`,
`cita_sitios`, `cita_visitantes`, `movimientos_visita`) y la RPC
`crear_cita_anfitrion` se mantienen intactas hasta V3 sólo porque el
núcleo las consulta en cada sincronización; en V3 se retira ese código y
se eliminan con una migración.

## Fases

| Fase | Contenido | Terminado cuando |
|---|---|---|
| **V1 Núcleo** | Dominio `visitas` (estados, `verificar_llegada`), servicio, repos, esquema local, tests. | 100 % de reglas con tests; sin UI todavía. |
| **V2 Nube** | Migraciones Supabase (tablas nuevas) + RLS + RPC (staging). | Cuenta ajena ve cero filas; tablas viejas intactas. |
| **V3 Puesto de control** | Escritorio y móvil: llegada, requisitos, gafete, salida, adentro ahora, walk-in. Retirar el código de citas del núcleo y borrar las tablas viejas. | Flujo completo en ambos, también sin conexión. |
| **V4 Anfitriones** | Web nueva según 3.8: mis visitas en vivo, agendar en una página, detalle con editar/cancelar/duplicar, solicitudes sin cita, avisos por correo. | Criterios de aceptación de 3.8. |
| **V5 Panel** | Anfitriones, restricciones, requisitos, historial/reportes, datos personales. | Nada de visitas requiere SQL a mano. |
| **V6 Emergencia** | "Adentro ahora" consolidado de todos los tipos, imprimible y sin conexión. | Pase de lista posible con la red caída. |

## Fuentes

- [Industrial Visitor Access Management Practices | friendlyway](https://www.friendlyway.com/visitor-access-industrial-best-practices/)
- [Compliant visitor management for manufacturing | Envoy](https://envoy.com/workplace-compliance-security-safety/compliant-visitor-management-for-manufacturing)
- [Manufacturing Visitor Management Systems | Avigilon](https://www.avigilon.com/blog/manufacturing-visitor-management-systems)
- [Manufacturing Visitor Management System: A 2026 Guide | GateSentry](https://gatesentry.com/blog/manufacturing-visitor-management-system-guide/)
- [Guard app: alertas y evacuación | Lobbytrack](https://www.lobbytrack.com/visitor-management/guard-app-features)
- [Tracking employees & visitors during an evacuation | FacilityOS](https://www.facilityos.com/blog/tracking-employees-visitors-during-emergency)
- [Ley 8968 | RIPD](https://www.redipd.org/en/legislation/ley-8968)
- [Reglamento a la Ley 8968 | SCIJ](https://pgrweb.go.cr/scij/Busqueda/Normativa/Normas/nrm_texto_completo.aspx?nValor1=1&nValor2=74352)
