# Panel administrativo web — plan (borrador, sin código todavía)

> Documento de continuidad para retomar esta conversación en otra sesión.
> Nace de reemplazar `admin-panel/panel-dispositivos.html` (clave única
> compartida, sin auditoría por persona) por un dashboard real para
> administradores, con historial multi-sitio, reportes y gestión de
> contratistas/operadores por unidad operativa. Nada de esto está
> decidido en firme todavía — es la base para decidir, sesión por sesión,
> igual que `plan-persistencia-nube.md`.

## Estado real de los datos hoy (2026-09-04): se puede arrancar de cero

**Ningún sitio está operando en producción todavía** — cero datos reales
en juego. El usuario confirmó explícitamente: **libre para borrar todo y
volver a sembrar (seed) de nuevo** cuando se implemente el modelo global
de contratistas/empresas/usuarios (ver más abajo). Esto quita de encima
la preocupación de reconciliar filas duplicadas entre sitios que ya
hubieran operado por separado — no hace falta diseñar una migración de
datos existentes, se puede resolver con un reseed limpio.

## Por qué existe este documento

Sesión de consulta (sin tocar código) donde se discutió: hosting
(Cloudflare vs AWS), el nivel de seguridad real del panel actual, y el
alcance de una versión nueva mucho más grande. Se decidió documentar el
alcance y las preguntas abiertas antes de escribir una sola línea.

## Alcance pedido (tal cual lo planteó el usuario)

1. Alta y baja de dispositivos (lo que ya hace `panel-dispositivos.html`
   hoy, vía las Edge Functions `admin-provision-device`/
   `admin-revoke-device`/`admin-list-devices`).
2. **Historial en vivo de las distintas unidades operativas** (multi-sitio).
3. Dar de baja a un **contratista** desde la web, por unidad operativa.
4. Dar de baja a un **operador** (usuario de la app) desde la web, por
   unidad operativa.
5. **Reportes** con "join" de información — métricas/ingresos globales
   agregando entre unidades operativas.
6. Seguridad real (no la clave única compartida de hoy).

## Auditoría del panel actual (`admin-panel/panel-dispositivos.html`)

Hecha en esta misma sesión, sin cambios de código todavía. Lo bueno: la
compuerta de login no es decorativa (valida contra la Edge Function de
verdad, server-side), escapa HTML correctamente en todo lo dinámico
(`escapeHtml`), y usa un header custom en vez de cookie (evita CSRF
clásico sin proponérselo). Lo que hay que resolver al construir la
versión nueva:

- **Clave única compartida entre todos los admins**, sin expiración, sin
  saber quién hizo qué. Reemplazar por identidad real (ver "Auth" abajo).
- **Vive en `localStorage` sin vencimiento** — riesgo bajo hoy (la página
  no carga scripts de terceros) pero sin capa de defensa si algún día se
  agrega una librería externa vulnerable.
- **El nivel de riesgo depende de dónde se hostee**: hoy corre como
  archivo local (según el README de `admin-panel/`); si se publica en una
  URL pública, cualquiera en internet llega a la compuerta de login y
  puede intentar adivinar la clave contra la Edge Function — ahí importa
  si esa función tiene límite de intentos fallidos (no verificado en esta
  sesión, vive del lado de Supabase).

## Decisión de auth (propuesta, a confirmar)

Reemplazar la clave compartida por **Supabase Auth** (mismo backend, sin
sumar otro proveedor):

- **Login principal: "Iniciar sesión con Google" (OAuth)**. Cada admin
  entra con su cuenta real — nombre propio, no más código compartido.
  Como beneficio lateral, hereda gratis el "Google prompt" (2FA de la
  propia cuenta de Google de esa persona) sin que Brisas tenga que
  construir nada de eso.
- **Segunda capa: Email OTP (código por correo), no TOTP.** Se descartó
  TOTP (Google Authenticator/Authy) a propósito — el usuario prefiere no
  depender de instalar/abrir una app de terceros aparte; revisar el
  correo es una acción que ya hace, sin fricción nueva. Supabase Auth
  soporta Email como tipo de factor MFA nativo, mismo nivel que TOTP.
- Nota de terminología aclarada en la conversación: "te manda a abrir
  Gmail para comprobar que sos vos" es el **Google prompt**, una función
  de la cuenta de Google del usuario, no algo que se programa acá — sólo
  se activa usando "Sign in with Google". Distinto del Email OTP de
  arriba (que si acaso se genera, no algo que ya traiga la cuenta de
  Google de la persona).

## Modelo de roles (decidido: sin roles)

**Importante: los admins de este panel web son un espacio de actores
distinto de Root/Administrador/Operador** — esos roles viven dentro de
cada sitio (base SQLite local de cada dispositivo, ver
`src/domain/autorizacion.rs`) y siguen existiendo igual.

Se había propuesto un segundo rol `admin_regional` (ve y actúa sólo sobre
las unidades operativas asignadas a esa persona), pero se eliminó
(migración `elimina_admin_regional`, 2026-09-05) sin haberse construido
nunca: `administradores_panel` nunca guardó qué sitios administraba cada
quien, así que el rol no tenía alcance real -- todo admin veía y tocaba
todo igual que `admin_global`. Decisión explícita del usuario: si algún
día hace falta acotar a alguien a unidades operativas específicas, se
diseña ese scoping desde cero (con su propia tabla de sitios asignados),
no reviviendo `admin_regional` como estaba.

`administradores_panel` es hoy una lista simple: `correo` + `creado_en`,
sin columna de rol. Estar en la tabla ES tener acceso completo al panel.

Autorización por **RLS en Postgres** (`es_admin_global()`, que ahora sólo
chequea que el correo esté en `administradores_panel`), no lógica de
permisos sólo en el frontend — mismo criterio que ya sigue el resto del
proyecto (nunca confiar en el cliente para autorizar).

## Modelo de datos: qué es global y qué es por sitio (decidido)

Terminología aclarada primero: **"sitio" = "unidad operativa" = "grupo"**
son el mismo concepto (Brisas, Cartago, Belén) — ya existe en el esquema
(`sitios`, `dispositivos.sitio_id`), no hace falta nada nuevo para esto.

Decisión de fondo, explícita: **contratistas, empresas y usuarios son
globales — ingresos (historial) son por sitio.**

- Si a un contratista se le niega el acceso en un sitio, queda negado en
  **todos** los sitios — es la misma persona, la misma decisión, no una
  copia local independiente por unidad operativa. Mismo criterio para
  dar de baja a un operador.
- Motivación real, no hipotética: **un operador puede tener que cubrir
  turno en otro sitio** — con cuentas globales no hace falta crearlo de
  nuevo en cada unidad donde vaya a trabajar.
- Los ingresos (el historial de entradas/salidas) siguen siendo
  estrictamente por sitio — un movimiento pasó en un lugar físico
  concreto, eso no se globaliza. Sin cambios sobre lo que ya existe.

**Esto contradice el esquema actual de Supabase, y hay que cambiarlo**:
hoy `contratistas`/`empresas` tienen columna `sitio_id` y
`recibir_catalogo_del_sitio` filtra explícitamente por el sitio del
dispositivo que sincroniza. Con el modelo global:
- Se saca el filtro por `sitio_id` de esa consulta y de las políticas
  RLS — cualquier dispositivo, de cualquier sitio, recibe el catálogo
  completo (no solo "el suyo").
- La futura tabla `usuarios` (ver huecos más abajo) se diseña así desde
  el arranque: global, sin `sitio_id` como filtro de lectura.
- `sitio_id` puede seguir existiendo como dato informativo, pero deja de
  ser el criterio que decide quién ve o recibe qué.
- **No hace falta diseñar una migración de datos existentes** — ver
  "Estado real de los datos hoy" arriba: ningún sitio opera todavía, se
  puede reseedear limpio en vez de reconciliar filas duplicadas.

**Root inicial y login offline: sin cambios.** Se evaluó (y se descartó)
que el arranque de un dispositivo nuevo dependiera de la nube para
recibir sus primeras cuentas — se prefirió no sumar esa complejidad.
Cada sitio sigue arrancando con `crear_root_inicial` local, como hoy. Lo
que sí cambia es que, una vez que un operador global existe (creado
localmente o desde el panel web), se sincroniza a **todos** los
dispositivos — y el login sigue siendo 100% local/offline en cualquiera
de ellos una vez que ese dispositivo ya sincronizó ese operador al menos
una vez.

## Conflicto: mismo contratista con ingreso abierto en dos sitios a la vez (decidido)

Caso real que surge **justo por** volver global a los contratistas: antes
era imposible ni plantearlo (cada sitio tenía su propio contratista
aislado). Ahora que es la misma persona en todos lados, puede intentar
(o ya tener) un ingreso abierto en más de un sitio al mismo tiempo —
físicamente no debería pasar.

Resuelto con dos niveles, sin sacrificar que el registro de ingreso siga
funcionando offline (principio ya establecido, no se toca):

1. **Con conexión, en el momento**: antes de confirmar el registro, el
   dispositivo consulta si ese contratista ya tiene un ingreso abierto en
   otro sitio. Si lo tiene, **se bloquea el segundo intento** — primero
   en llegar gana (mismo principio que ya usa `cerrar_ingreso_remoto` para
   cierres). Es una extensión de `ingresos_remotos`, que hoy sólo mira
   otros dispositivos del *mismo* sitio, a mirar *todos* los sitios para
   este chequeo puntual.
2. **Sin conexión, o si la consulta falla**: el ingreso se registra igual
   localmente — no se le puede negar el paso a un guardia offline. El
   conflicto se detecta después, al sincronizar: si aparecen dos ingresos
   abiertos para el mismo contratista en sitios distintos, se dispara una
   **alerta hacia ambos sitios involucrados y hacia el admin** (panel
   web) — ej. "Jenna Ortega tiene entradas abiertas simultáneas en Brisas
   y Cartago". Ahí un humano llama por teléfono a verificar qué pasó (mal
   registrado, o de verdad no se marcó la salida en el primer sitio antes
   de entrar al segundo).

No hay forma honesta de garantizar bloqueo 100% de las veces sin
depender de red en el momento del registro — la mezcla "bloqueo si hay
conexión, alerta si no la hay" es la que preserva el offline-first.

## Verificación en dos pasos al dar de alta un dispositivo (decidido)

Corrige una confusión de una vuelta anterior de esta conversación: **no
es TOTP/Authy, es un código por correo**, y no ocurre al generar el
secreto sino al **activarlo**:

1. Se genera el secreto del dispositivo (sin gate, como hoy).
2. Se pega en la app del dispositivo nuevo.
3. La app, al usar el secreto por primera vez, dispara una petición de
   verificación al servidor.
4. El código llega **al correo del admin**, no al dispositivo.
5. El admin escribe ese código **en la app del dispositivo** (no en el
   panel web).
6. Recién ahí el dispositivo queda activado.

El punto: el secreto solo **no alcanza** para activar nada. Si se filtra
(screenshot, archivo compartido), sin ese código —que sólo llega al
correo del admin— no sirve de nada. Es un gate humano en el momento de
activación (no de creación), un patrón de *step-up authentication*
similar a "confirmar transferencia" de un banco aunque ya estés logueado.

**Implicación de backend real**: la Edge Function `device-auth` (o una
nueva) necesita un estado intermedio — "secreto válido, pendiente de
verificación" — antes de emitir el JWT final, más el envío del correo
con el código. Trabajo concreto a diseñar, no configuración.

## Protección del secreto del dispositivo en reposo (decidido)

Hoy se guarda en texto plano (`src/nube/credenciales.rs`,
`fs::write(directorio.join(FILE_NAME), secreto.trim())`), tanto en
Windows como en Android. Dos capas, cada una cubre algo que la otra no:

- **Capa 1 — cifrar el archivo con un ID único de la máquina.** Se
  deriva una clave del identificador del dispositivo y se cifra el
  secreto antes de escribirlo a disco; si el archivo se copia a otra
  máquina, descifra mal (falla el `match`, no da un secreto usable). Se
  evaluó reusar el `generate_hardware_id()` de otro proyecto del usuario
  (`rust-brisas-app`), que usa la dirección MAC — **descartado**: la MAC
  se puede cambiar por software y es inconsistente entre interfaces de
  red. Se usa en su lugar: **Machine GUID de Windows** (registro,
  vía crate `machine-uid`) y **`Settings.Secure.ANDROID_ID`** en Android
  (se pasa desde Kotlin al núcleo, mismo patrón que ya usa `directorio`
  hoy porque Android no tiene `%LOCALAPPDATA%`).
- **Capa 2 — el receptor también recuerda el fingerprint del dispositivo**,
  guardado en la verificación por correo del alta (sección anterior), y
  lo exige en cada sincronización futura además del secreto. Esta es la
  que de verdad no depende de la máquina del atacante.

**Alcance explícito de esto — qué NO promete**: ninguna de las dos capas
protege contra alguien con acceso total/administrador a la máquina
original (extraer el secreto ya descifrado en memoria, debuggear el
binario) — ese es un límite aceptado, no un hueco a tapar; ningún
esquema que viva solo en la máquina sobrevive a eso. Lo que sí cubren:
Capa 1 evita que **copiar el archivo** (backup mal subido, pendrive,
correo por error) sirva en otro lado; Capa 2 evita que un secreto
extraído **incluso en el caso extremo** sirva para autenticarse desde
una máquina distinta a la original, porque el fingerprint no calza del
lado del servidor.

## Los dos huecos reales en el modelo de datos (a decidir antes de construir)

Descubiertos al mirar qué existe hoy contra lo que el alcance pide:

### 1. Historial no se espeja a la nube — "historial en vivo" no existe todavía

Hoy la nube sólo guarda un caché liviano de ingresos **abiertos** (para
el cierre cruzado entre dispositivos del mismo sitio,
`ingresos_remotos`/`recibir_ingresos_abiertos`) — el historial completo
(movimientos ya cerrados) es deliberadamente local-only en cada sitio,
por diseño, para no cargar una tabla que crece sin límite
(`docs/pendientes.md`, sección Historial).

**A decidir**: para "historial en vivo multi-sitio" y "reportes con
ingresos globales", alguna de:
- (a) Espejar el historial completo (o los movimientos ya cerrados) a
  una tabla en Supabase — cambia el modelo de privacidad/tamaño de datos
  en la nube, hay que pensar retención.
- (b) Algo más liviano: agregados/contadores por sitio y período (ej. "38
  ingresos hoy en Brisas"), sin guardar cada movimiento individual en la
  nube — cubre "reportes globales" sin el volumen de (a), pero no un
  historial fila-por-fila en vivo.

### 2. Usuarios/operadores tampoco se espejan a la nube

Hoy son 100% locales a la base SQLite de cada sitio — nunca viajan a
Supabase (a diferencia de contratistas, empresas y gafetes, que sí
tienen su tabla espejo). Con la decisión de operadores globales (ver
arriba), esto ya no es sólo "para poder dar de baja a un operador desde
la web" — es también **cómo un operador llega a poder loguearse en un
sitio nuevo que no es el suyo**. Hace falta:
- Una tabla `usuarios` (o similar) en Supabase, **sin `sitio_id` como
  filtro de lectura** (global, no por sitio — ver "Modelo de datos"
  arriba, a diferencia de lo que se pensaba antes en esta misma sesión).
- Un `recibir_usuarios` (mismo patrón que `recibir_catalogo_del_sitio`
  en `src/nube/sincronizacion.rs`, pero sin el `WHERE sitio_id = ...`)
  para que cada dispositivo reciba el catálogo completo de operadores.
- **Decidido: el hash de password NO viaja a la nube.** Se evaluó
  mandarlo (más simple) contra un enrolamiento único por dispositivo (más
  seguro, un paso extra sólo la primera vez) — se optó por el segundo.
  La nube distribuye quién existe y su rol/estado activo, no la
  contraseña. La primera vez que un operador global entra a un
  dispositivo nuevo, ese dispositivo pide una confirmación única en
  línea (código al correo del operador o del admin) para fijar la
  contraseña ahí — de ahí en adelante ese dispositivo la valida local y
  offline con su propio hash, generado en el momento (Argon2, mismo
  código que ya existe en `src/services/password.rs` — confirmado que el
  hash es autocontenido/portable por formato PHC, no dependiente de la
  instalación, así que no hay problema técnico de portabilidad; la
  decisión de no mandarlo es sólo por reducir el radio de un breach de
  la nube). Motivo real de descartar mandarlo: un solo breach de
  Supabase expondría los hashes de todos los operadores de todos los
  sitios en un solo lugar, en vez de "si rompen un sitio, comprometen
  sólo ese sitio" como hoy.

**Contratistas — la mecánica base ya funciona, falta sacarle el filtro
por sitio**: `recibir_catalogo_del_sitio` ya trae el catálogo completo en
cada sync y sobreescribe `tiene_acceso` desde la nube — dar de baja a un
contratista desde una fuente externa ya se propaga sola (confirmado
leyendo el código, ver `docs/plan-persistencia-nube.md`, sesión
2026-09-04). Hoy sólo llega a los dispositivos **del mismo sitio**
(`?sitio_id=eq...`); con el modelo global (ver arriba) hay que sacar ese
filtro para que llegue a todos los sitios, no sólo al de origen.

## Arquitectura propuesta (a confirmar)

- **React + Vite**, mismo stack que `desktop/` — reusar patrones
  (tablas, formularios, capa `api/*.ts` que es la única que llama al
  backend) en vez de JS a mano como el panel actual.
- Hosting: retomar la conversación de Cloudflare Pages vs. AWS de esta
  sesión una vez que el alcance de datos (arriba) esté resuelto — no
  bloquea empezar el diseño de datos/auth.
- Si se hostea en una URL pública (no ya como archivo local): considerar
  una capa de gate adicional antes del login de la app (ej. Cloudflare
  Access) — discutido en esta sesión, gratis hasta 50 usuarios, cierra el
  acceso a nivel de red antes de que nadie llegue siquiera a la pantalla
  de login de Supabase Auth.

## Orden sugerido para retomar

1. Resolver el hueco de **historial** (opción a vs. b) — bloquea diseñar
   el esquema de reportes. Sigue siendo por sitio, sólo falta decidir
   cuánto se agrega/espeja para reportes globales.
2. Construir el mirror **global** de `usuarios` (`recibir_usuarios`, sin
   filtro por sitio) — confirmar antes si el hash de password viaja o no.
   Sacar el filtro `sitio_id` del lado de `recibir_catalogo_del_sitio`
   (contratistas/empresas) para que también sean globales.
3. Diseñar el chequeo cruzado de ingreso abierto en más de un sitio
   (bloqueo online / alerta offline) — depende de 2 (necesita saber "está
   abierto en otro sitio" más allá del propio).
4. Definir el modelo de roles del panel web (nombres, granularidad).
5. Migrar auth: Supabase Auth (Google OAuth + Email OTP) reemplazando la
   clave compartida — se puede hacer ya, sin esperar a 1-3, porque el
   panel actual (alta/baja de dispositivos) ya usa Edge Functions que
   pueden migrar a validar sesión de Supabase Auth en vez de `x-admin-key`.
   La verificación en dos pasos al activar un dispositivo (secreto +
   código por correo) es parte de este mismo trabajo.
6. Recién ahí: construir el dashboard (React+Vite) sobre el modelo de
   datos y auth ya resueltos.

Sin diseñar el esquema SQL, los RLS puntuales, ni las pantallas todavía
— eso es la siguiente sesión, una vez que 1, 2 y 4 tengan respuesta
final del usuario (2 tiene una pregunta abierta: el hash de password).
