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
- **TOTP (Supabase Auth lo soporta nativo)** como segunda capa exigida
  por la app, independiente de si el admin tiene 2FA en su Google — para
  no depender de que cada persona lo tenga bien configurado del otro lado.
- Nota de terminología aclarada en la conversación: "contraseña + correo
  con un PIN" es *Email OTP* (un segundo factor que si acaso se
  construye); "te manda a abrir Gmail para comprobar que sos vos" es el
  *Google prompt*, una función de la cuenta de Google del usuario, no
  algo que se programa acá — sólo se activa usando "Sign in with Google".

## Modelo de roles (propuesta, a confirmar)

**Importante: los admins de este panel web son un espacio de actores
distinto de Root/Administrador/Operador** — esos roles viven dentro de
cada sitio (base SQLite local de cada dispositivo, ver
`src/domain/autorizacion.rs`) y siguen existiendo igual. El panel web
necesita su propia tabla de roles, por ejemplo:

- `admin_global` — ve y actúa sobre todas las unidades operativas.
- `admin_regional` — ve y actúa sólo sobre las unidades operativas
  asignadas a esa persona.

**A decidir**: nombres definitivos de los roles, y si hace falta algo más
granular (ej. un rol de solo-lectura para reportes, sin permiso de dar de
baja a nadie).

Autorización por **RLS en Postgres filtrando por ese rol**, no lógica de
permisos sólo en el frontend — mismo criterio que ya sigue el resto del
proyecto (nunca confiar en el cliente para autorizar).

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
tienen su tabla espejo). "Dar de baja a un operador de determinada
unidad desde la web" necesita:
- Una tabla `usuarios` (o similar) en Supabase, con RLS por sitio.
- Un `recibir_usuarios` (mismo patrón que `recibir_catalogo_del_sitio`
  en `src/nube/sincronizacion.rs`) para que cada dispositivo reciba el
  cambio de vuelta.
- Definir qué campos viajan (¿el hash de password? probablemente no —
  sólo `activo`/rol, como con `tiene_acceso` de contratistas).

**Contratistas sí funciona ya, sin tocar nada**: `recibir_catalogo_del_sitio`
trae el catálogo completo en cada sync y sobreescribe `tiene_acceso`
desde la nube — dar de baja a un contratista desde una fuente externa ya
se propaga sola a todos los dispositivos del sitio (confirmado leyendo
el código, ver `docs/plan-persistencia-nube.md`, sesión 2026-09-04).

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

1. Resolver el hueco de **historial** (opción a vs. b arriba) — bloquea
   diseñar el esquema de reportes.
2. Resolver el mirror de **usuarios/operadores** — bloquea la función de
   dar de baja a un operador desde la web.
3. Definir el modelo de roles del panel web (nombres, granularidad).
4. Migrar auth: Supabase Auth (Google OAuth + TOTP) reemplazando la clave
   compartida — esto sí se puede hacer ya, sin esperar a 1/2, porque el
   panel actual (alta/baja de dispositivos) ya usa Edge Functions que
   pueden migrar a validar sesión de Supabase Auth en vez de `x-admin-key`.
5. Recién ahí: construir el dashboard (React+Vite) sobre el modelo de
   datos y auth ya resueltos.

Sin diseñar el esquema SQL, los RLS puntuales, ni las pantallas todavía
— eso es la siguiente sesión, una vez que 1-3 tengan respuesta del
usuario.
