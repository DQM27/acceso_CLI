# Arquitectura de Supabase — `control-acceso-nube`

Documento de referencia completo: qué hay, por qué está así, y cómo funciona
de verdad por dentro. Escrito para que cualquiera (dev nuevo, vos en seis
meses, o una IA sin contexto previo) pueda entender el sistema completo sin
tener que reconstruirlo leyendo 65 migraciones una por una.

No reemplaza a `docs/decisiones-tecnicas.md` (el "por qué se decidió así, en
orden cronológico, con el error que lo motivó") ni a `docs/recuperacion-supabase.md`
(el runbook paso a paso para reconstruir el proyecto desde cero). Este
documento es el mapa: la foto completa de cómo encajan todas las piezas HOY.

---

## 1. Qué es esto y para qué existe

`control-acceso-nube` es la base de datos en la nube (Postgres, vía Supabase)
que le da a un sistema de control de acceso físico (garitas/recepciones de
varios sitios) tres cosas que SQLite local, por sí solo, no puede dar:

1. **Sincronización entre dispositivos y sitios** — un contratista dado de
   baja en un sitio queda dado de baja en todos; un administrador ve el
   historial de todos los sitios desde un panel web.
2. **Avisos en vivo (Realtime)** — cuando algo cambia en un sitio, los demás
   dispositivos conectados a ese mismo sitio se enteran al instante, sin
   tener que refrescar ni esperar un pulso periódico.
3. **Una identidad única de administración** (`administradores_panel` +
   Supabase Auth) — quién puede entrar al panel web y quién puede operar
   como ROOT/ADMINISTRADOR/OPERADOR en un sitio, en un solo lugar, en vez de
   una contraseña distinta por PC física (el modelo viejo, "kiosco").

Cada dispositivo físico (PC de escritorio, celular, o un "visor" de solo
lectura) sigue teniendo su propia base SQLite local — puede seguir
funcionando sin internet. Supabase es la capa de sincronización, no el
almacén primario del día a día.

---

## 2. El modelo de datos

### 2.1 Control de acceso (el núcleo original)

| Tabla | Qué guarda | Nota clave |
| --- | --- | --- |
| `sitios` | Cada ubicación física (ej. "Brisas") | Poquísimas filas, cambia rarísima vez |
| `dispositivos` | Cada PC/celular/visor autorizado, con su `secret_hash` | El secreto es lo único que un dispositivo necesita para autenticarse — ver sección 4 |
| `contratistas` | Catálogo de personas autorizadas a entrar | **Global**, no por sitio (ver 3.2) |
| `empresas` | Catálogo de empresas contratistas | También global |
| `gafetes` | Inventario físico de gafetes por sitio | Sí es estrictamente por sitio, sin excepción |
| `ingresos` | El historial real: cada entrada/salida | Espejo en la nube de la tabla local `registro_ingresos` |

### 2.2 Identidad y administración

| Tabla | Qué guarda | Nota clave |
| --- | --- | --- |
| `administradores_panel` | Quién puede entrar al panel web administrativo | Alta/baja es manual (dashboard/SQL), a propósito — no hay pantalla para auto-gestionarse |
| `usuarios` | ROOT/ADMINISTRADOR/OPERADOR de cada sitio | `auth_user_id` la enlaza a `auth.users` (Supabase Auth) — la contraseña real vive ahí, nunca en esta tabla |

### 2.3 Control de visitas (más nuevo, 2026-09-09 en adelante)

| Tabla | Qué guarda | Nota clave |
| --- | --- | --- |
| `anfitriones` | Quién puede agendar visitas desde la web pública de visitas | Mismo patrón que `administradores_panel`: el login solo confirma identidad, esta tabla decide autorización |
| `citas` | Una solicitud de visita agendada | `estado` es VIGENTE/CANCELADA; "VENCIDA" se calcula al leer, no se guarda |
| `cita_sitios` | Puente muchos-a-muchos: a qué sitio(s) aplica una cita | Permite una cita "tour" con varios sitios a la vez |
| `cita_visitantes` | Cada persona incluida en una cita | Una cita puede traer varias personas |
| `movimientos_visita` | El cruce real en el punto de acceso (entrada/salida de un visitante) | Espejo en la nube de la tabla local homónima; **no está en la publicación de Realtime** (ver 4.1) |

**Gap conocido:** a diferencia de `contratistas`/`gafetes`, este módulo
todavía no tiene script de repoblado en `supabase/scripts/` — ver
`docs/recuperacion-supabase.md`, sección 6.

---

## 3. Los dos modelos de autorización (y por qué conviven)

Este es el punto que más confunde a quien lee el código por primera vez:
**hay dos sistemas de identidad completamente distintos, superpuestos.**

### 3.1 Identidad de dispositivo (el modelo original, "kiosco")

Cuando un dispositivo se activa, llama a la Edge Function `device-auth` con
su `secret_hash`. Si es válido, esa función firma un JWT propio (ES256, con
`DEVICE_SIGNING_KEY`) con estos claims:

```json
{ "role": "authenticated", "sub": "<id del dispositivo>", "sitio_id": "<uuid del sitio>", "tipo": "pc | mobile | visor" }
```

Este JWT **no pasa por Supabase Auth** — es un token que la app arma y firma
ella misma, y Postgres lo valida igual porque comparte la clave pública de
verificación. Las políticas RLS que dicen "del propio sitio" comparan
`sitio_id` contra este claim. `tipo = 'visor'` está explícitamente excluido
de poder escribir nada (dispositivo de solo lectura).

### 3.2 Identidad de panel/administración (el modelo nuevo, Supabase Auth real)

Un humano que entra al panel web, o un `usuarios` (ROOT/ADMIN/OPERADOR) que
hace login en el escritorio, sí pasa por Supabase Auth real —
`auth.email()`/`auth.jwt()` vienen de una sesión de verdad, con
`administradores_panel` (panel completo) o `usuarios.auth_user_id` (rol por
sitio) decidiendo qué puede hacer esa identidad.

**Cambio reciente e importante:** los usuarios (`usuarios`, ROOT/ADMIN/
OPERADOR) **ya no se crean desde las apps de escritorio/móvil** — se crean
desde el panel web (`admin-create-usuario`, sección 5). Las apps cliente
consumen esa identidad (login), no la originan. Esto es parte de
`docs/plan-autenticacion-supabase-auth.md`.

### 3.3 "Global" no significa "sin control" — el modelo cross-site

`contratistas`, `empresas` y `usuarios` tienen políticas de lectura/
escritura que dicen `using (true)` matizado por "sitio_id no nulo en el JWT
O admin_global" — es decir, **cualquier dispositivo autenticado de
cualquier sitio puede leer y escribir estas tres tablas para TODOS los
sitios**, no solo el suyo. Es intencional (una baja de contratista tiene
que verse en todos los sitios a la vez, ver `docs/decisiones-tecnicas.md`,
entrada "globaliza_contratistas_y_empresas"), pero es un radio de exposición
real y documentado: un dispositivo comprometido de un sitio puede, hoy,
tocar datos de todos los demás sitios, incluido reasignarle el `rol` a un
usuario ajeno. Esto está marcado como hallazgo de seguridad A-01, abierto,
en `docs/auditorias/AUDITORIA_SEGURIDAD_WEB_SUPABASE_2026-09-10.md`, y los
propios tests (`supabase/tests/usuarios_autorizacion.sql`,
`contratistas_autorizacion.sql`) lo verifican a propósito — si algún día se
cierra esa política, esos tests van a fallar, y esa falla es la señal de
que hay que revisar el hallazgo, no un bug del test.

`ingresos`, `dispositivos` y `gafetes`, en cambio, sí están acotados
estrictamente por sitio para un dispositivo normal (solo `admin_global` ve
todo).

### 3.4 Autorización interna: el esquema `private`

Hay funciones `SECURITY DEFINER` (corren con privilegios elevados, saltando
RLS del objeto que consultan) que existen solo para uso INTERNO de otras
políticas RLS — nunca para ser llamadas directo por un cliente:

- `private.es_admin_global()` — ¿el correo de quien llama está en
  `administradores_panel`? (movida a `private` el 2026-09-12, ver 6.4)
- `private.sitios_de_cita(cita_id)` / `private.anfitrion_de_cita(cita_id)`
  — rompen una recursión infinita de RLS entre `citas`/`cita_sitios`

Viven en el esquema `private` **a propósito**: `supabase/config.toml` solo
expone `public`/`graphql_public` en la Data API, así que cualquier función
en `private` es invisible por HTTP (`/rest/v1/rpc/...`) aunque un cliente
autenticado la conozca de nombre — solo las políticas RLS (que corren
dentro de Postgres, no por HTTP) pueden invocarlas.

---

## 4. Realtime — cómo funciona de verdad

Esta es la pieza que más costó dejar funcionando bien, así que va con
detalle. Hay **tres mecanismos distintos**, todos bajo el nombre "Realtime"
pero con comportamiento y configuración independientes:

### 4.1 Postgres Changes (el más simple: "avisame cuando cambie una fila")

Se activa publicando una tabla:

```sql
alter publication supabase_realtime add table nombre_tabla;
```

Publicadas hoy: **`ingresos`, `contratistas`, `empresas`, `usuarios`,
`dispositivos`, `administradores_panel`**. Un cliente suscrito a esta
publicación recibe automáticamente cada INSERT/UPDATE/DELETE de esas tablas
que la RLS le permita ver — Supabase decodifica el WAL (Write-Ahead Log) de
Postgres por detrás; de hecho, es la consulta que más tiempo de CPU consume
de todo el proyecto (ver Query Performance en el dashboard).

**No publicadas:** `gafetes`, `sitios`, `anfitriones`, `citas`,
`cita_sitios`, `cita_visitantes`, `movimientos_visita` — el panel web las
actualiza por *polling* periódico en vez de en vivo.

⚠️ Si algún día se hace `DROP TABLE` + recrear cualquiera de las 6 tablas
publicadas, el `ALTER PUBLICATION` se pierde solo — hay que volver a
ejecutarlo a mano, no sobrevive a un recreate.

### 4.2 Broadcast — el aviso instantáneo "algo cambió en este sitio"

Distinto de Postgres Changes: acá, un **trigger** en cada tabla llama
explícitamente a `realtime.send(...)` para publicar un mensaje en un canal
con nombre `sitio:<uuid-del-sitio>`:

```sql
create trigger <tabla>_emitir_cambio_nube
  after insert or delete or update on public.<tabla>
  for each row execute function private.emitir_cambio_nube_sitio();
```

Existe hoy en: `contratistas`, `empresas`, `gafetes`, `ingresos`,
`usuarios`, `movimientos_visita`. El payload incluye quién hizo el cambio
(`auth.jwt()->>'sub'`, el dispositivo que escribió, no el que creó la fila
originalmente — un bug ya corregido, ver `docs/decisiones-tecnicas.md`,
entrada 2026-09-06) para que el propio emisor pueda descartar su eco y no
reprocesar su propio cambio.

Para que un cliente pueda *escuchar* este canal, hace falta una política de
autorización sobre `realtime.messages` (una tabla especial de Supabase, NO
una tabla de tu esquema):

```sql
create policy "dispositivos reciben broadcast de su sitio"
  on realtime.messages for select to authenticated
  using (
    extension = 'broadcast'
    and (select realtime.topic()) = ('sitio:' || (select auth.jwt())->>'sitio_id')
  );
```

**El infierno histórico** (2026-09-05, `corregir_autorizacion_realtime`): la
primera versión de esta política exigía además `private = true` en la fila,
pero Supabase evalúa esa columna con un valor por defecto (`false`) que
nunca calzaba — el resultado era que la política bloqueaba **todo**, no
solo lo que debía bloquear. El síntoma no daba ninguna pista obvia (no es
un error de token, ni de clave, ni de conexión — el canal se conectaba
bien, simplemente nunca recibía nada). La lección: cuando Realtime "se
conecta pero no llega nada", sospechar primero de la política de
autorización sobre `realtime.messages`, no de credenciales.

### 4.3 Presence — "quién está conectado ahora mismo"

Tercer mecanismo, para saber qué dispositivos están activos en vivo (no
cambios de datos, sino presencia de clientes). Necesita **dos** políticas
separadas sobre `realtime.messages` con `extension = 'presence'`: una de
`SELECT` (para poder escuchar quién está presente) y otra de `INSERT` (para
poder anunciar la propia presencia con `track()`) — faltar la segunda da el
error `UnableToHandlePresence: :unauthorized`, visto en producción antes de
agregarla (`autoriza_publicar_presencia_por_sitio`, 2026-09-08). Un
dispositivo solo ve presencia de su propio sitio; `admin_global` ve la de
cualquiera (para el panel de presencia en tiempo real del panel web).

### 4.4 Invariantes a NO romper nunca sin revisar esta sección

- Las 6 tablas en `supabase_realtime` publication (listadas en 4.1).
- Los 6 triggers `*_emitir_cambio_nube` (listados en 4.2) y la función
  `private.emitir_cambio_nube_sitio()` que todos comparten.
- Las 3 políticas de `realtime.messages` (broadcast recibir, presencia leer,
  presencia publicar).
- Usar `DELETE`, no `TRUNCATE`, si algún día hay que vaciar una tabla con
  triggers de aviso en vivo — `TRUNCATE` no dispara triggers `FOR EACH ROW`,
  así que un cliente conectado no se enteraría de que sus datos
  desaparecieron.

---

## 5. Edge Functions

Todas (salvo `device-auth`) exigen una sesión de Supabase Auth válida cuyo
correo esté en `administradores_panel` (`correoAdminAutorizado`, reemplazo
de un viejo `x-admin-key` compartido).

| Función | Qué hace | Nota |
| --- | --- | --- |
| `device-auth` | Autentica un dispositivo por `secret_hash`, firma su JWT | Único endpoint público sin auth de admin |
| `admin-provision-device` | Da de alta un dispositivo nuevo, genera su secreto | El secreto se devuelve una sola vez, en texto plano |
| `admin-create-site` | Da de alta un sitio | |
| `admin-list-devices` | Lista dispositivos | |
| `admin-revoke-device` | Baja permanente de un dispositivo | |
| `admin-suspend-device` | Baja temporal (`suspended_at`) | Distinto de revocar — se puede reactivar |
| `admin-delete-device` | Borra un dispositivo, o lo oculta si tiene historial (FK) | |
| `admin-create-usuario` | Crea un `usuarios` + su cuenta en Supabase Auth | Correo sintético `<cedula>@brisas.local`, contraseña temporal — este es el reemplazo de "crear usuario desde la app" |
| `admin-reset-password-usuario` | Resetea la contraseña de un `usuarios` | Cubre también el backfill de usuarios viejos sin `auth_user_id` |
| `sync-access-policy` | Sincroniza `administradores_panel` → Cloudflare Access | Se dispara sola por trigger; ver Vault abajo |

---

## 6. Vault (secretos cifrados en la base)

Distinto de los secrets de Edge Functions (`supabase secrets set`, viven en
la plataforma) — Vault guarda secretos **dentro de la base de datos**,
accesibles solo desde funciones `SECURITY DEFINER` vía
`vault.decrypted_secrets`.

| Secreto | Quién lo usa | Para qué |
| --- | --- | --- |
| `sync_access_policy_apikey` | `sync_access_policy()` (trigger) | Header `Authorization: Bearer` al invocar la Edge Function |
| `sync_access_policy_webhook_secret` | `sync_access_policy()` (trigger) | Header `x-webhook-secret` — la Edge Function lo exige además del apikey, para que no baste con un `anon key` público para invocarla |

Si `administradores_panel` cambia y estos dos secretos no están cargados,
el trigger lanza una excepción explícita en vez de fallar en silencio.

---

## 6.4 Hallazgos de seguridad ya cerrados (2026-09-12)

Auditoría completa + limpieza de esta fecha, para que quede claro qué ya
está resuelto y no haga falta re-auditarlo:

- **Drift de migraciones**: 65 migraciones locales ahora coinciden 1:1 con
  `supabase_migrations.schema_migrations` de producción (antes había 19 con
  el timestamp de archivo incorrecto y 10 que no existían en git). Ver
  `docs/decisiones-tecnicas.md`.
- **`es_admin_global()` invocable por HTTP**: movida a `private` (no
  expuesta por la Data API). Verificado sin cambio de comportamiento contra
  las 9 baterías de test de `supabase/tests/`.
- **12 políticas RLS de visitas sin optimizar** (`auth_rls_initplan`): ahora
  todas envuelven `auth.email()`/`auth.jwt()` en `(select ...)`.
- **4 tablas con políticas RLS permisivas duplicadas**
  (`multiple_permissive_policies`): fusionadas en `anfitriones`,
  `dispositivos`, `sitios`, `usuarios` — mismo resultado, una sola
  evaluación.
- **3 FKs sin índice + 1 índice duplicado**: corregidos.

**Pendientes, deliberadamente no tocados hoy** (bajo riesgo pero requieren
más cuidado/tiempo del que ameritaba esta pasada):

- `pg_net` instalada en el esquema `public` en vez de uno dedicado — cambio
  de esquema de una extensión con dependencias (`sync_access_policy()` la
  usa) requiere más prueba antes de tocarla.
- "Leaked password protection" desactivado en Auth — no se puede activar
  por SQL/migración, es un toggle de dashboard (Authentication → Policies).
  Ya no aplica la razón original ("no hay contraseñas que proteger") desde
  que existe login por contraseña real vía Supabase Auth — se recomienda
  activarlo.
- A-01 (RLS cross-site en contratistas/empresas/usuarios, sección 3.3) —
  riesgo aceptado y documentado, no un olvido.
- 7 índices "sin uso" — normal en un proyecto con pocos días de tráfico
  real, no acota nada todavía.

---

## 7. Flujo de trabajo: cómo cambiar el esquema de ahora en adelante

1. Escribir el `.sql` en `supabase/migrations/` (o generarlo con
   `supabase migration new <nombre>`).
2. Probar contra un Branch de Supabase (ver `docs/decisiones-tecnicas.md`)
   o directo contra producción si el cambio es de bajo riesgo y está bien
   entendido — correr `supabase/tests/*.sql` antes y después si toca RLS.
3. Correr `mcp__supabase__get_advisors` (o `supabase db advisors` con CLI
   ≥2.81.3) después de cualquier cambio de esquema — detecta huecos de RLS,
   funciones mal expuestas, índices faltantes.
4. Commitear y mergear a `main` — desde 2026-09-12, GitHub Integration
   despliega la migración a producción sola. Ya no hace falta correr
   `supabase db push` a mano en el flujo normal (sigue sirviendo para
   reconstruir un proyecto desde cero, ver `docs/recuperacion-supabase.md`).

## 8. Para profundizar

- **El "por qué" de cada decisión, en orden cronológico** →
  `docs/decisiones-tecnicas.md`
- **Cómo reconstruir el proyecto desde cero** →
  `docs/recuperacion-supabase.md`
- **El contrato de la web de visitas** → `docs/contrato-web-visitas.md`
- **El plan de autenticación con Supabase Auth** →
  `docs/plan-autenticacion-supabase-auth.md`
- **Auditorías de seguridad puntuales** → `docs/auditorias/`
- **Los tests de autorización reales, uno por tabla** → `supabase/tests/`
  (correrlos es la mejor forma de entender qué puede hacer cada rol —
  están escritos como diagnóstico ejecutable, con `begin`/`rollback`, no
  solo como documentación)
