# Respuesta al contrato de integración — backend listo para publicar

Fecha: 2026-09-09. Responde a `contrato-web-visitas.md` y a los bloqueos P0/P1
de `reporte-seguridad-web-2026-09-09.md` marcados como responsabilidad del
backend. Todo lo de acá ya está aplicado y probado en `xidaepyaljzkpbsxrqsm`
(no son instrucciones pendientes, son el estado actual de la base).

## P0 · Recursión de RLS — resuelto

Reproduje el `42P17` exacto del contrato antes de tocar nada. Causa real: la
política SELECT de `citas` hace `EXISTS` sobre `cita_sitios`, y las políticas
de `cita_sitios` hacen `EXISTS` sobre `citas` — cada una está sujeta a RLS de
la otra, así que evaluarlas se realimenta indefinidamente. `anfitriones` sólo
lo exponía porque su política une `citas` con `cita_sitios`.

Arreglo: dos funciones `SECURITY DEFINER` (`private.sitios_de_cita`,
`private.anfitrion_de_cita`) que leen el otro lado del ciclo sin pasar por su
RLS — mismo patrón que `es_admin_global()`, ya usado en este proyecto. Viven
en el esquema `private` (no `public`) a propósito: la primera versión las
dejó en `public` y el linter de seguridad las marcó como invocables directo
por `/rest/v1/rpc/...` incluso por `anon` — eso hubiera sido una fuga real
(cualquiera podía pedir "a qué sitios aplica la cita X" para cualquier uuid,
saltándose la política que se supone lo protege). Corregido antes de seguir.

Verificado con un rol `authenticated` real (no `postgres`, que no aplica
RLS): las 4 tablas se leen sin error, tanto con JWT de anfitrión (`email`)
como de dispositivo (`sitio_id`). `get_advisors(type='security')` ya no
marca nada nuevo.

## P1 · `crear_cita_anfitrion` — implementada exactamente como se propuso

```sql
public.crear_cita_anfitrion(
  p_id uuid, p_fecha_desde date, p_fecha_hasta date,
  p_motivo text, p_sitios uuid[], p_visitantes jsonb
) returns uuid
```

`SECURITY INVOKER`, no `DEFINER`: corre con el RLS del propio anfitrión, los
`INSERT` internos siguen pasando por las políticas ya existentes — no es una
puerta trasera, sólo agrupa 3 escrituras ya permitidas en una transacción
atómica real (cualquier `raise exception` revierte todo lo insertado hasta
ese punto).

Probado explícitamente (transacciones de prueba, revertidas, sin tocar datos
reales):
- Creación normal → devuelve `p_id`.
- Reintento idéntico (mismo `p_id`, mismo contenido) → devuelve el mismo id,
  cero filas duplicadas.
- Mismo `p_id`, contenido distinto → rechazado (`23505`).
- Sitio inexistente → rechazado, cero filas en `citas`/`cita_sitios`/`cita_visitantes`.
- Falla a mitad del array de visitantes (segundo visitante inválido, después
  de ya haber insertado cabecera + sitios + el primer visitante) → rollback
  completo, cero filas — la atomicidad es real, no sólo en el caso feliz.
- Correo sin fila en `anfitriones` (o `activo = false`) → rechazado (`42501`).
- Reactivar una cita cancelada por `UPDATE` directo (no por la RPC) → rechazado
  por un trigger nuevo (`citas_bloquea_reactivacion`), sea por la RPC o por
  REST directo.

Validación aplicada (server-side, cubre también el camino de escritura
directa REST vía `CHECK` en las tablas, no sólo dentro de la RPC):
fechas reales, inicio ≥ hoy en `America/Costa_Rica`, fin ≥ inicio, 1–50
visitantes, 1–100 sitios existentes y sin duplicados, límites de texto
(nombre 150, cédula 30, empresa 150, placa 20, motivo 1000), caracteres de
control rechazados, cédula duplicada dentro del mismo grupo rechazada.
Cuota simple: máx. 20 citas por anfitrión por hora (ajustable sin tocar la
web si hace falta más).

Idempotencia: `citas.contenido_hash` (columna nueva) guarda un hash del
contenido normalizado. Un reintento con el mismo `p_id` se resuelve **antes**
de validar fecha pasada, tal como pedía el contrato (una respuesta perdida
reenviada después de medianoche sobre una cita que sí se guardó a tiempo no
debe fallar).

**Importante — no renormalicen la cédula del lado del cliente más allá de
`trim`.** El contrato proponía "convierte a mayúsculas y elimina espacios y
guiones", pero confirmé contra el núcleo real
(`CitaRepository::buscar_por_cedula`, `src/database/repositories/cita_repository.rs`):
la búsqueda en el check-in del guardia compara por **igualdad exacta de
texto en SQL**, sin `UPPER`/`TRIM` de guiones. La RPC guarda con `btrim`
(sólo espacios) y nada más — es justo lo que hice porque agregar una
normalización de mayúsculas/guiones del lado de la web, sin ese mismo cambio
también en el guardia, hubiera roto silenciosamente el check-in (cédula
agendada como "1-1111-1111", guardada como "11111111" tras la normalización,
y el guardia escaneando/escribiendo con guiones nunca la encuentra). De paso
corregí un bug real que esto mismo destapó: el check-in del guardia tampoco
recortaba espacios antes de esa comparación exacta — ya alineado
(`src/services/cita_service.rs`, commit `b802fe7`).

## Baja de anfitriones

`anfitriones.activo boolean not null default true` (columna nueva). La
política de lectura propia (`cada anfitrion lee su propia fila`) y la RPC ya
lo exigen. No hace falta invalidar el JWT de Supabase Auth: la autorización
se re-lee de la tabla en cada operación, nunca se cachea — desactivar a
alguien lo bloquea de inmediato aunque su sesión siga técnicamente vigente.
El alta/baja sigue siendo manual (SQL directo), como ya era.

## De regalo: dos hallazgos P1 del panel, corregidos

No estaban en el bloqueo de visitas pero eran rápidos y de alto impacto,
los resolví de una vez:

- **Administradores por API**: `web/src/App.tsx` ya había retirado la
  pantalla "para que un compromiso del panel no pueda agregar
  administradores", pero las políticas `INSERT`/`DELETE` de
  `administradores_panel` seguían vivas — cualquier sesión de admin (robada
  o no) podía seguir usándolas por PostgREST directo. Las retiré,
  restaurando la intención ya documentada en la migración original (alta/baja
  por `service_role`, no por rol cliente).

## Lo que sigue abierto (no lo toqué — no es mi capa o necesita otra decisión)

- **Confirmación por correo de `admin-revoke-device`/`admin-delete-device`
  dependiente de la interfaz** (P1): es código de Edge Functions
  (`supabase/functions/`), no esquema/RLS. Requiere definir qué cuenta como
  "elevación de autenticación" server-side antes de tocarlo.
- **Sesión vieja puede restaurar estado tras cambio de cuenta** (P2):
  `web/src/contexto/AuthContexto.tsx`, código React del panel existente.
- **Validación de entrada de funciones administrativas** (P2): Edge
  Functions también.
- `pg_net` en `public`, protección de contraseñas filtradas: configuración
  del proyecto/dashboard, no migraciones.
- Cloudflare Access/WAF para `visitas.megabrisas.com`: configuración de
  cuenta, no algo que se resuelva desde Supabase.

Nada de esto bloquea publicar `visitas.megabrisas.com` — son mejoras al
panel existente o configuración de infraestructura, no al flujo nuevo de
citas. El backend de visitas (RLS + RPC) está listo para que se conecte de
verdad y se repitan las pruebas de dos anfitriones / cuenta sin alta /
dispositivo de otro sitio que pedía el contrato.
