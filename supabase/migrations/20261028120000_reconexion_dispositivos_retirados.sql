-- Reconexión de un equipo retirado (ver
-- docs/features-futuras/plan-reconexion-equipo-retirado.md, decisiones del
-- dueño del 2026-10-24).
--
-- Hasta ahora `revoked_at` era definitivo (migración
-- revocacion_efectiva_dispositivos): para volver a usar un equipo retirado
-- había que borrar su base y darlo de alta como nuevo. Desde acá, un
-- administrador puede autorizar en el panel la reconexión de ESE equipo con
-- un código propio:
--
-- 1. `codigos_vinculacion.proposito`: 'alta' (lo de siempre) o 'reconexion'.
--    Un código de reconexión lleva la unidad y la etiqueta que va a tener el
--    equipo (`sitio_destino_id`, `etiqueta_destino`). Un código de un
--    propósito nunca sirve para el otro.
-- 2. `canjear_codigo_vinculacion` (alta) exige `proposito = 'alta'`.
-- 3. `consultar_codigo_reconexion`: dice a qué unidad va el equipo sin gastar
--    el código (el equipo avisa antes si va a perder registros).
-- 4. `canjear_codigo_reconexion`: de una sola vez marca el código usado, le
--    ata al equipo su clave NUEVA, lo reactiva con la unidad y la etiqueta
--    del código y deja el evento con lo que pasó. Sólo si el código es de
--    reconexión, está vigente y sin usar, y el equipo sigue retirado.
--
-- Los tokens viejos del equipo no vuelven a servir: llevan la huella de la
-- clave anterior y `private.dispositivo_vigente()` la compara con la nueva.
-- Los disparadores de expulsión y de cierre de sesiones sólo actúan al
-- retirar (`revoked_at` de nulo a fecha), no al reactivar.

alter table public.codigos_vinculacion
  add column if not exists proposito text not null default 'alta',
  add column if not exists sitio_destino_id uuid references public.sitios (id),
  add column if not exists etiqueta_destino text;

alter table public.codigos_vinculacion
  drop constraint if exists codigos_vinculacion_proposito_check;
alter table public.codigos_vinculacion
  add constraint codigos_vinculacion_proposito_check
  check (proposito in ('alta', 'reconexion'));

-- Un código de reconexión siempre dice a dónde va el equipo; uno de alta no.
alter table public.codigos_vinculacion
  drop constraint if exists codigos_vinculacion_destino_check;
alter table public.codigos_vinculacion
  add constraint codigos_vinculacion_destino_check
  check ((proposito = 'reconexion') = (sitio_destino_id is not null and etiqueta_destino is not null));

alter table public.eventos_seguridad_dispositivos
  drop constraint eventos_seguridad_dispositivos_tipo_check;
alter table public.eventos_seguridad_dispositivos
  add constraint eventos_seguridad_dispositivos_tipo_check check (tipo in (
    'codigo_inexistente',
    'codigo_usado',
    'codigo_vencido',
    'codigo_anulado',
    'firma_invalida',
    'hardware_distinto',
    'sesion_en_otra_unidad',
    'sesion_en_duda',
    'reconexion_autorizada',
    'reconexion_canjeada'
  ));

-- Igual que antes, salvo `c.proposito = 'alta'`.
create or replace function public.canjear_codigo_vinculacion(
  p_codigo_hash text,
  p_clave_publica_jwk jsonb,
  p_clave_huella text,
  p_metadata jsonb,
  p_ip text
)
returns table (dispositivo_id uuid, sitio_id uuid, tipo text)
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_dispositivo_id uuid;
begin
  update public.codigos_vinculacion c
     set usado_en = pg_catalog.now(),
         usado_ip = p_ip
   where c.codigo_hash = p_codigo_hash
     and c.proposito = 'alta'
     and c.usado_en is null
     and c.anulado_en is null
     and c.expira_en > pg_catalog.now()
     and exists (
       select 1 from public.dispositivos d
        where d.id = c.dispositivo_id
          and d.revoked_at is null
          and d.clave_huella is null
     )
  returning c.dispositivo_id into v_dispositivo_id;

  if v_dispositivo_id is null then
    return;
  end if;

  return query
  update public.dispositivos d
     set clave_publica_jwk = p_clave_publica_jwk,
         clave_huella = p_clave_huella,
         vinculado_en = pg_catalog.now(),
         last_seen_at = pg_catalog.now(),
         last_ip = p_ip,
         identificador_hardware = p_metadata ->> 'identificador_hardware',
         nombre_dispositivo = p_metadata ->> 'nombre_dispositivo',
         plataforma = p_metadata ->> 'plataforma',
         version_build = p_metadata ->> 'version_build',
         app_version = p_metadata ->> 'app_version'
   where d.id = v_dispositivo_id
     and d.revoked_at is null
     and d.clave_huella is null
  returning d.id, d.sitio_id, d.tipo;
end;
$$;

revoke all on function public.canjear_codigo_vinculacion(text, jsonb, text, jsonb, text)
  from public, anon, authenticated;
grant execute on function public.canjear_codigo_vinculacion(text, jsonb, text, jsonb, text)
  to service_role;

-- A dónde va el equipo con este código, sin gastarlo. Vacío si el código no
-- es de reconexión, venció, se usó o se anuló, o si el equipo ya no está
-- retirado.
create or replace function public.consultar_codigo_reconexion(p_codigo_hash text)
returns table (
  dispositivo_id uuid,
  sitio_actual_id uuid,
  sitio_destino_id uuid,
  sitio_destino_nombre text,
  etiqueta_destino text
)
language sql
stable
security definer
set search_path = ''
as $$
  select c.dispositivo_id, d.sitio_id, c.sitio_destino_id, s.nombre, c.etiqueta_destino
    from public.codigos_vinculacion c
    join public.dispositivos d on d.id = c.dispositivo_id
    join public.sitios s on s.id = c.sitio_destino_id
   where c.codigo_hash = p_codigo_hash
     and c.proposito = 'reconexion'
     and c.usado_en is null
     and c.anulado_en is null
     and c.expira_en > pg_catalog.now()
     and d.revoked_at is not null;
$$;

revoke all on function public.consultar_codigo_reconexion(text) from public, anon, authenticated;
grant execute on function public.consultar_codigo_reconexion(text) to service_role;

-- Canje de un código de reconexión. `p_detalle` lo arma la Edge Function
-- (registros que el equipo descartó y quién confirmó) y queda en el evento,
-- junto con la unidad anterior y la nueva. Si la clave nueva ya pertenece a
-- otro dispositivo (índice único de `clave_huella`), todo se revierte,
-- incluido el uso del código.
create or replace function public.canjear_codigo_reconexion(
  p_codigo_hash text,
  p_clave_publica_jwk jsonb,
  p_clave_huella text,
  p_metadata jsonb,
  p_ip text,
  p_detalle jsonb
)
returns table (dispositivo_id uuid, sitio_id uuid, tipo text, cambio_de_unidad boolean)
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_dispositivo_id uuid;
  v_sitio_destino uuid;
  v_etiqueta text;
  v_sitio_anterior uuid;
begin
  update public.codigos_vinculacion c
     set usado_en = pg_catalog.now(),
         usado_ip = p_ip
   where c.codigo_hash = p_codigo_hash
     and c.proposito = 'reconexion'
     and c.usado_en is null
     and c.anulado_en is null
     and c.expira_en > pg_catalog.now()
     and exists (
       select 1 from public.dispositivos d
        where d.id = c.dispositivo_id
          and d.revoked_at is not null
     )
  returning c.dispositivo_id, c.sitio_destino_id, c.etiqueta_destino
       into v_dispositivo_id, v_sitio_destino, v_etiqueta;

  if v_dispositivo_id is null then
    return;
  end if;

  select d.sitio_id into v_sitio_anterior
    from public.dispositivos d
   where d.id = v_dispositivo_id
     for update;

  update public.dispositivos d
     set revoked_at = null,
         oculto_en_panel = false,
         sitio_id = v_sitio_destino,
         etiqueta = v_etiqueta,
         clave_publica_jwk = p_clave_publica_jwk,
         clave_huella = p_clave_huella,
         vinculado_en = pg_catalog.now(),
         last_seen_at = pg_catalog.now(),
         last_ip = p_ip,
         identificador_hardware = p_metadata ->> 'identificador_hardware',
         nombre_dispositivo = p_metadata ->> 'nombre_dispositivo',
         plataforma = p_metadata ->> 'plataforma',
         version_build = p_metadata ->> 'version_build',
         app_version = p_metadata ->> 'app_version'
   where d.id = v_dispositivo_id
     and d.revoked_at is not null;

  insert into public.eventos_seguridad_dispositivos (dispositivo_id, tipo, ip, detalle)
  values (
    v_dispositivo_id,
    'reconexion_canjeada',
    p_ip,
    coalesce(p_detalle, '{}'::jsonb) || pg_catalog.jsonb_build_object(
      'sitio_anterior', v_sitio_anterior,
      'sitio_nuevo', v_sitio_destino
    )
  );

  return query
  select d.id, d.sitio_id, d.tipo, v_sitio_anterior is distinct from v_sitio_destino
    from public.dispositivos d
   where d.id = v_dispositivo_id;
end;
$$;

revoke all on function public.canjear_codigo_reconexion(text, jsonb, text, jsonb, text, jsonb)
  from public, anon, authenticated;
grant execute on function public.canjear_codigo_reconexion(text, jsonb, text, jsonb, text, jsonb)
  to service_role;
