-- Ingreso "por correo": una persona con visita autorizada por correo
-- (generalmente entrevistas de RH) que entra sin estar en ningún catálogo.
-- Es el comodín mientras se termina el módulo de Visitas: mismo mecanismo
-- que un ingreso de proveedor (cédula, nombre, placa si viene en vehículo,
-- gafete obligatorio), pero con gafete de VISITA y un motivo libre ("a quién
-- visita") en lugar de la empresa.
--
-- Tabla propia, no una columna de tipo en `ingresos_proveedor`: las apps ya
-- instaladas leen esa tabla y una fila sin empresa podía romperles la
-- sincronización. Ésta la ignoran.
--
-- Mismas garantías que proveedores:
-- * la entrada es inmutable y la salida se registra una sola vez;
-- * cada equipo sólo escribe en su unidad (los visores no escriben);
-- * un gafete no puede estar en uso dos veces en la misma unidad, ni la
--   misma cédula adentro dos veces en ninguna unidad (índices únicos: el
--   segundo `INSERT` recibe 409 y la cola de salida lo marca fallido);
-- * funciones `security definer` para saber si la persona está adentro en
--   otra unidad sin abrir la lectura de las filas de otras unidades.

create table public.ingresos_correo (
  id uuid primary key,
  sitio_id uuid not null references public.sitios (id),
  dispositivo_entrada_id uuid not null references public.dispositivos (id),
  cedula text not null,
  nombre text not null,
  motivo text not null check (pg_catalog.btrim(motivo) <> ''),
  placa text,
  gafete_numero bigint not null,
  hora_entrada timestamptz not null,
  usuario_entrada_nombre text not null,
  hora_salida timestamptz,
  dispositivo_salida_id uuid references public.dispositivos (id),
  usuario_salida_nombre text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint ingresos_correo_salida_coherente check (
    (hora_salida is null and dispositivo_salida_id is null and usuario_salida_nombre is null)
    or (hora_salida is not null and usuario_salida_nombre is not null)
  ),
  constraint ingresos_correo_salida_posterior check (hora_salida is null or hora_salida >= hora_entrada)
);

comment on table public.ingresos_correo is
  'Ingresos autorizados por correo (comodín previo al módulo de Visitas). Gafete de visita obligatorio.';

-- Índices: los mismos que `ingresos_proveedor`.
create index ingresos_correo_dispositivo_entrada_id_idx
  on public.ingresos_correo (dispositivo_entrada_id);
create index ingresos_correo_dispositivo_salida_id_idx
  on public.ingresos_correo (dispositivo_salida_id)
  where dispositivo_salida_id is not null;
create index ingresos_correo_sitio_hora_entrada_idx
  on public.ingresos_correo (sitio_id, hora_entrada desc);
create index ingresos_correo_hora_entrada_idx
  on public.ingresos_correo (hora_entrada desc);
create index ingresos_correo_sitio_updated_at_idx
  on public.ingresos_correo (sitio_id, updated_at);
create unique index ingresos_correo_gafete_activo_sitio_idx
  on public.ingresos_correo (sitio_id, gafete_numero)
  where hora_salida is null;
create unique index ingresos_correo_cedula_activa_idx
  on public.ingresos_correo (cedula)
  where hora_salida is null;

-- Triggers: entrada inmutable, salida única, `updated_at` y aviso de cambio
-- a los equipos de la unidad (el mismo `emitir_cambio_nube_sitio` de las
-- demás tablas de movimientos).
create function private.ingresos_correo_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if new.id is distinct from old.id
     or new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.cedula is distinct from old.cedula
     or new.nombre is distinct from old.nombre
     or new.motivo is distinct from old.motivo
     or new.placa is distinct from old.placa
     or new.gafete_numero is distinct from old.gafete_numero
     or new.hora_entrada is distinct from old.hora_entrada
     or new.usuario_entrada_nombre is distinct from old.usuario_entrada_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de entrada de un ingreso por correo son inmutables';
  end if;
  if old.hora_salida is not null and new.hora_salida is distinct from old.hora_salida then
    raise exception 'La salida de un ingreso por correo solo puede registrarse una vez';
  end if;
  new.updated_at := pg_catalog.now();
  return new;
end;
$$;

create trigger ingresos_correo_entrada_inmutable
  before update on public.ingresos_correo
  for each row execute function private.ingresos_correo_bloquear_cambios_de_entrada();

create trigger ingresos_correo_emitir_cambio_nube
  after insert or update or delete on public.ingresos_correo
  for each row execute function private.emitir_cambio_nube_sitio();

-- RLS: igual que `ingresos_proveedor`.
alter table public.ingresos_correo enable row level security;

create policy "leer ingresos_correo del propio sitio o admin"
  on public.ingresos_correo for select to authenticated
  using (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    or (select private.es_admin_global())
  );

create policy "crear ingresos_correo del propio sitio"
  on public.ingresos_correo for insert to authenticated
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "actualizar ingresos_correo del propio sitio"
  on public.ingresos_correo for update to authenticated
  using (sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid)
  with check (
    sitio_id = ((select auth.jwt()) ->> 'sitio_id')::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "solo dispositivos vigentes"
  on public.ingresos_correo as restrictive for all to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));

revoke all on table public.ingresos_correo from anon;
revoke truncate, references, trigger, delete on table public.ingresos_correo from authenticated;

-- Ingreso por correo abierto con esta cédula, en cualquier unidad (a lo
-- sumo uno, por el índice). Para la verificación antes de registrar.
create function public.ingreso_correo_activo(p_cedula text)
returns table (sitio_id uuid, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if (private.dispositivo_que_llama()).id is null and not (select private.es_admin_global()) then
    raise exception 'Sólo un equipo registrado puede consultar ingresos activos' using errcode = '42501';
  end if;

  return query
    select i.sitio_id, s.nombre
      from public.ingresos_correo i
      join public.sitios s on s.id = i.sitio_id
     where i.cedula = pg_catalog.btrim(p_cedula)
       and i.hora_salida is null
     limit 1;
end;
$$;

revoke all on function public.ingreso_correo_activo(text) from public, anon;
grant execute on function public.ingreso_correo_activo(text) to authenticated;

-- De una lista de cédulas, las que están adentro por correo en OTRA unidad
-- que la del equipo que llama. Para el aviso posterior a sincronizar.
create function public.correos_activos_en_otras_unidades(p_cedulas text[])
returns table (cedula text, sitio_nombre text)
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  v_equipo public.dispositivos;
begin
  v_equipo := private.dispositivo_que_llama();
  if v_equipo.id is null then
    raise exception 'Sólo un equipo registrado puede consultar ingresos activos' using errcode = '42501';
  end if;
  if pg_catalog.cardinality(p_cedulas) > 1000 then
    raise exception 'Demasiadas cédulas' using errcode = '22023';
  end if;

  return query
    select i.cedula, s.nombre
      from public.ingresos_correo i
      join public.sitios s on s.id = i.sitio_id
     where i.cedula = any (p_cedulas)
       and i.sitio_id <> v_equipo.sitio_id
       and i.hora_salida is null;
end;
$$;

revoke all on function public.correos_activos_en_otras_unidades(text[]) from public, anon;
grant execute on function public.correos_activos_en_otras_unidades(text[]) to authenticated;

-- Panel: "quién está adentro ahora" suma los ingresos por correo (el
-- motivo va en la columna de empresa, que es lo que identifica la visita).
create or replace view public.panel_adentro_ahora
with (security_invoker = true) as
select
  'CONTRATISTA'::text as tipo,
  i.id,
  i.sitio_id,
  s.nombre as sitio_nombre,
  i.contratista_cedula as identificacion,
  i.contratista_nombre as nombre,
  i.empresa_nombre,
  i.gafete_numero,
  nullif(btrim(i.placa), '') as placa,
  i.hora_entrada,
  i.usuario_entrada_nombre
from public.ingresos i
left join public.sitios s on s.id = i.sitio_id
where i.hora_salida is null
union all
select
  'PROVEEDOR'::text,
  p.id,
  p.sitio_id,
  s.nombre,
  p.cedula,
  p.nombre,
  p.empresa_nombre,
  p.gafete_numero,
  nullif(btrim(p.placa), ''),
  p.hora_entrada,
  p.usuario_entrada_nombre
from public.ingresos_proveedor p
left join public.sitios s on s.id = p.sitio_id
where p.hora_salida is null
union all
select
  'PROVISIONAL_KOF'::text,
  k.id,
  k.sitio_id,
  s.nombre,
  k.encargado_codigo_empleado,
  k.encargado_nombre,
  'KOF',
  k.gafete_numero,
  null,
  k.hora_entrega,
  k.usuario_entrega_nombre
from public.prestamos_gafete_provisional k
left join public.sitios s on s.id = k.sitio_id
where k.hora_devolucion is null
union all
select
  'POR_CORREO'::text,
  c.id,
  c.sitio_id,
  s.nombre,
  c.cedula,
  c.nombre,
  c.motivo,
  c.gafete_numero,
  nullif(btrim(c.placa), ''),
  c.hora_entrada,
  c.usuario_entrada_nombre
from public.ingresos_correo c
left join public.sitios s on s.id = c.sitio_id
where c.hora_salida is null;

revoke all on public.panel_adentro_ahora from anon, authenticated;
grant select on public.panel_adentro_ahora to authenticated;
