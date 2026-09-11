# Handoff: app web "Agendar visitas" (subdominio nuevo, para anfitriones/KOF)

Este documento es para otra IA/desarrollador que va a construir esta pieza
en paralelo. Es **trabajo totalmente desacoplado** del núcleo Rust/Tauri
(que se sigue desarrollando por separado) — no toca ese código, no importa
nada de `src/`, `desktop/`, `mobile/`. Solo consume Supabase.

## Qué hay que construir

Una SPA web nueva, en un **subdominio nuevo** (ej. `agendar.megabrisas.com`,
a definir), donde un empleado de la empresa (el "anfitrión", a quien
internamente llamamos KOF) agenda una visita: quién viene, cuándo, por
cuánto tiempo, a qué sitio(s). Cuando el visitante llega físicamente, un
guardia (en la app de escritorio/Tauri, **no en esta web**) escanea su
cédula, el sistema busca la cita por cédula, y si está vigente le asigna un
gafete y lo deja entrar. Esta web **no hace check-in** — solo agenda.

No hay app móvil para esto (agendar es poco frecuente, no necesita cámara
ni modo offline) — solo esta web.

## Precedente a copiar: el panel administrativo ya existente

Ya existe una web hermana en este mismo repo, en [`web/`](../web/), que se
despliega como Worker de Cloudflare (`panel-brisas`, ver
[`web/wrangler.jsonc`](../web/wrangler.jsonc)) y usa exactamente el patrón
de auth que esta app nueva debe replicar:

- **Stack**: React 19 + TypeScript + Vite + Tailwind v4, `@supabase/supabase-js`,
  `react-router-dom`, `zod`. Ver [`web/package.json`](../web/package.json).
- **Auth**: Google OAuth vía Supabase Auth (`supabase.auth.signInWithOAuth({provider: "google"})`).
  El login de Google **solo prueba identidad** — la autorización real la
  decide una tabla propia en Postgres (ver abajo), consultada tras cada
  cambio de sesión. Copiar el patrón completo de
  [`web/src/contexto/AuthContexto.tsx`](../web/src/contexto/AuthContexto.tsx)
  (incluye manejo cuidadoso de: no desloguear por un blip de red, no
  remontar toda la UI en cada refresh de token, logout que también limpia
  la cookie de Cloudflare Access si aplica).
- **Cliente Supabase**: ver [`web/src/lib/supabase.ts`](../web/src/lib/supabase.ts)
  — instancia simple con URL + publishable key (son públicas por diseño,
  la seguridad la da RLS, no esconder la clave).
- **Deploy**: Cloudflare Worker con static assets (no Pages). `wrangler.jsonc`
  con `"assets": {"directory": "./dist", "not_found_handling": "single-page-application"}`.
  Este agente/IA puede pedir que se cree el Worker y subdominio nuevos
  (yo tengo acceso a Cloudflare vía MCP) cuando el código esté listo — no
  hace falta que la otra IA tenga esas credenciales.

## Backend: ya existe, no crear nada nuevo en Postgres sin coordinar

Proyecto Supabase: `xidaepyaljzkpbsxrqsm`.

```
SUPABASE_URL = https://xidaepyaljzkpbsxrqsm.supabase.co
SUPABASE_PUBLISHABLE_KEY = sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU
```

(Ambas son seguras para vivir en el cliente/frontend — no son secretas.)

Las tablas de este dominio ya están creadas y con RLS activo. **No
modificar el esquema** — si hace falta una columna o política nueva,
pedirla en vez de aplicarla directo, porque este mismo proyecto Supabase
también sirve a la app de escritorio/guardias y a los dispositivos
offline, y un cambio mal pensado puede romper esos sync.

### `anfitriones`

Quién puede entrar a esta web. Mismo patrón que `administradores_panel`
(la tabla que autoriza el panel de admin existente): el login de Google
solo confirma identidad, esta tabla decide autorización real. Alta/baja es
manual (SQL directo) por ahora — no hay pantalla de gestión de anfitriones
todavía, y no es parte de este alcance.

| columna     | tipo        | notas                     |
|-------------|-------------|---------------------------|
| `correo`    | text PK     | debe matchear `auth.email()` de Google |
| `nombre`    | text        |                           |
| `creado_en` | timestamptz | default now()             |

RLS: `select` — cada anfitrión lee únicamente su propia fila
(`auth.email() = correo`). Así el frontend, ya logueado con Google, puede
chequear "¿estoy autorizado?" con un `select` simple; si no hay fila,
mostrar "tu cuenta no está autorizada" y desloguear (igual que hace
`AuthContexto.tsx` con `administradores_panel`).

### `citas`

La autorización de la visita, con vigencia. Puede ser para un grupo de
personas (ver `cita_visitantes`) y puede aplicar a más de un sitio (caso
"tour", ver `cita_sitios`).

| columna            | tipo | notas |
|--------------------|------|-------|
| `id`                | uuid PK | |
| `anfitrion_correo`  | text FK → `anfitriones.correo` | |
| `motivo`            | text nullable | |
| `fecha_desde`       | date | |
| `fecha_hasta`       | date | `fecha_hasta >= fecha_desde` (constraint) |
| `estado`            | text | `'VIGENTE'` o `'CANCELADA'` (default `'VIGENTE'`). No hay `'VENCIDA'` persistida — se calcula comparando `fecha_hasta` contra hoy en el momento de la consulta. |
| `created_at` / `updated_at` | timestamptz | |

RLS:
- `select`: el propio anfitrión (`anfitrion_correo = auth.email()`), o un
  dispositivo del sitio (vía `cita_sitios`), o admin global.
- `insert`: el anfitrión solo puede crear citas con su propio correo.
- `update`: el anfitrión solo puede editar sus propias citas (para
  cancelar — **no hay `delete`**, una cita se cancela, nunca se borra,
  mismo criterio que "los movimientos de acceso no se eliminan").

### `cita_sitios` (puente muchos-a-muchos)

A qué sitio(s) aplica la cita. Una fila por sitio incluido.

| columna    | tipo | notas |
|------------|------|-------|
| `cita_id`  | uuid FK → `citas.id` on delete cascade | |
| `sitio_id` | uuid FK → `sitios.id` | PK compuesta (cita_id, sitio_id) |

RLS: el anfitrión puede insertar/leer/borrar filas de sus propias citas.

### `cita_visitantes`

Una fila por persona del grupo que agenda la cita.

| columna          | tipo | notas |
|------------------|------|-------|
| `id`              | uuid PK | |
| `cita_id`         | uuid FK → `citas.id` on delete cascade | |
| `cedula`          | text | **clave de búsqueda** que usará el guardia al escanear |
| `nombre`          | text | |
| `empresa`         | text nullable | |
| `placa_vehiculo`  | text nullable | |
| `created_at`      | timestamptz | |

RLS: insert/update/delete restringidos a citas propias del anfitrión;
select cubre también dispositivos del sitio y admin global.

### `sitios` (ya existía, solo lectura para esta app)

| columna | tipo | notas |
|---------|------|-------|
| `id` | uuid PK | |
| `nombre` | text | |
| `direccion` | text nullable | |

RLS: acabo de agregar una política `"anfitrion lee sitios"` — cualquier
correo presente en `anfitriones` puede leer la tabla completa (necesario
para el selector de "a qué sitio(s) invito"). Antes de esto, solo
`admin_global` podía leerla.

## Flujo funcional esperado (mínimo viable)

1. Login con Google → si el correo no está en `anfitriones`, mostrar
   mensaje de no autorizado y desloguear (igual que el panel existente).
2. Pantalla "Nueva cita": elegir sitio(s) (multi-select sobre `sitios`),
   rango de fechas (`fecha_desde`/`fecha_hasta`), motivo opcional, y una
   lista de visitantes (nombre + cédula obligatorios, empresa/placa
   opcionales — puede ser 1 o varios). Al guardar: insert en `citas`,
   luego inserts en `cita_sitios` y `cita_visitantes` con el `cita_id`
   devuelto.
3. Pantalla "Mis citas": listar las citas del anfitrión (`select` con
   embeds a `cita_visitantes` y `cita_sitios`/`sitios`), mostrando estado
   real (VIGENTE / CANCELADA / vencida-calculada comparando `fecha_hasta`
   contra hoy). Permitir cancelar (`update estado = 'CANCELADA'`) — nunca
   borrar.
4. (Opcional, no prioritario) Editar visitantes de una cita ya creada
   antes de que empiece.

**Explícitamente fuera de alcance para esta app:**
- Cualquier flujo de check-in/check-out físico (eso vive en el desktop
  Tauri, lado guardia).
- Notificación al anfitrión cuando su visitante llega (deferred, "más
  estético que otra cosa" según el dueño del producto).
- Gestión de gafetes, colores por tipo de actor, proveedores.
- Pantalla de alta/baja de anfitriones (por ahora manual vía SQL).

## Convenciones del repo a respetar

- Comentarios y nombres de variables en español (como el resto del repo).
- Sin la palabra "garita" en ningún lado (preferencia explícita del
  dueño del producto — usar "punto de acceso" si hace falta el concepto).
- Comentarios solo quando explican un *por qué* no obvio, no qué hace el
  código.
- No mezclar esta entidad (`citas`/visitas) con `contratistas` en el
  modelo — son conceptos separados a propósito, aunque compartan
  infraestructura de Supabase.
