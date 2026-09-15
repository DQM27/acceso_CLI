-- Control de rutas (docs/planes-implementados/plan-control-rutas.md) --
-- primer corte de sync, espejo de ingresos/movimientos_visita. Catálogos
-- (vehiculos_ruta/encargados_ruta) acotados por sitio como ingresos, NO
-- globales como empresas/contratistas (una flota/personal KOF es propia
-- de un sitio, no compartida entre sitios distintos).

-- =====================================================================
-- vehiculos_ruta
-- =====================================================================
create table public.vehiculos_ruta (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_origen_id uuid not null references public.dispositivos(id),
  numero_unidad text,
  placa text not null,
  activo boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index vehiculos_ruta_placa_key on public.vehiculos_ruta (placa);
create index vehiculos_ruta_sitio_idx on public.vehiculos_ruta (sitio_id);
create index vehiculos_ruta_dispositivo_origen_id_idx on public.vehiculos_ruta (dispositivo_origen_id);

alter table public.vehiculos_ruta enable row level security;

create policy "leer vehiculos_ruta del propio sitio o admin"
  on public.vehiculos_ruta for select to authenticated
  using (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    or private.es_admin_global()
  );

create policy "crear vehiculos_ruta del propio sitio"
  on public.vehiculos_ruta for insert to authenticated
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "actualizar vehiculos_ruta del propio sitio"
  on public.vehiculos_ruta for update to authenticated
  using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create function public.vehiculos_ruta_actualizar_updated_at()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  new.updated_at = now();
  return new;
end;
$function$;

create trigger vehiculos_ruta_set_updated_at
  before update on public.vehiculos_ruta
  for each row execute function public.vehiculos_ruta_actualizar_updated_at();

create trigger vehiculos_ruta_emitir_cambio_nube
  after insert or delete or update on public.vehiculos_ruta
  for each row execute function private.emitir_cambio_nube_sitio();

-- =====================================================================
-- encargados_ruta
-- =====================================================================
create table public.encargados_ruta (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_origen_id uuid not null references public.dispositivos(id),
  codigo_empleado text not null,
  nombre text not null,
  cedula text,
  activo boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index encargados_ruta_codigo_empleado_key on public.encargados_ruta (codigo_empleado);
create index encargados_ruta_sitio_idx on public.encargados_ruta (sitio_id);
create index encargados_ruta_dispositivo_origen_id_idx on public.encargados_ruta (dispositivo_origen_id);

alter table public.encargados_ruta enable row level security;

create policy "leer encargados_ruta del propio sitio o admin"
  on public.encargados_ruta for select to authenticated
  using (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    or private.es_admin_global()
  );

create policy "crear encargados_ruta del propio sitio"
  on public.encargados_ruta for insert to authenticated
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "actualizar encargados_ruta del propio sitio"
  on public.encargados_ruta for update to authenticated
  using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create function public.encargados_ruta_actualizar_updated_at()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  new.updated_at = now();
  return new;
end;
$function$;

create trigger encargados_ruta_set_updated_at
  before update on public.encargados_ruta
  for each row execute function public.encargados_ruta_actualizar_updated_at();

create trigger encargados_ruta_emitir_cambio_nube
  after insert or delete or update on public.encargados_ruta
  for each row execute function private.emitir_cambio_nube_sitio();

-- =====================================================================
-- salidas_ruta -- espejo de ingresos (apertura/cierre), pero con
-- "retorno" en vez de "salida" para no chocar con la terminología local
-- (fecha_hora_salida/fecha_hora_retorno, ver MIGRACION_36 del núcleo
-- Rust). vehiculo_id/encargado_id OPCIONALES a propósito (pedido
-- explícito del usuario): el snapshot de texto es la fuente real, el
-- link al catálogo es sólo un plus si hay match.
-- =====================================================================
create table public.salidas_ruta (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_salida_id uuid not null references public.dispositivos(id),
  vehiculo_id uuid references public.vehiculos_ruta(id),
  vehiculo_placa text not null,
  vehiculo_numero_unidad text,
  encargado_id uuid references public.encargados_ruta(id),
  encargado_nombre text not null,
  numero_ruta text not null,
  sub_numero bigint not null,
  numero_documento text not null,
  fecha_documento date not null,
  resultado text not null check (resultado in ('PERMITIDO', 'PERMITIDO_CON_AUTORIZACION')),
  motivo_resultado text check (
    motivo_resultado is null or motivo_resultado in ('DOCUMENTO_FECHA_DISTINTA')
  ),
  hora_salida timestamptz not null,
  usuario_salida_nombre text not null,
  hora_retorno timestamptz,
  dispositivo_retorno_id uuid references public.dispositivos(id),
  usuario_retorno_nombre text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (
    (resultado = 'PERMITIDO' and motivo_resultado is null)
    or (resultado = 'PERMITIDO_CON_AUTORIZACION' and motivo_resultado = 'DOCUMENTO_FECHA_DISTINTA')
  ),
  check (
    (hora_retorno is null and dispositivo_retorno_id is null and usuario_retorno_nombre is null)
    or (hora_retorno is not null and usuario_retorno_nombre is not null)
  ),
  check (hora_retorno is null or hora_retorno >= hora_salida)
);

create unique index salidas_ruta_sitio_documento_key on public.salidas_ruta (sitio_id, numero_documento);
create index salidas_ruta_sitio_idx on public.salidas_ruta (sitio_id);
create index salidas_ruta_vehiculo_id_idx on public.salidas_ruta (vehiculo_id) where vehiculo_id is not null;
create index salidas_ruta_encargado_id_idx on public.salidas_ruta (encargado_id) where encargado_id is not null;
create index salidas_ruta_dispositivo_salida_id_idx on public.salidas_ruta (dispositivo_salida_id);
create index salidas_ruta_dispositivo_retorno_id_idx on public.salidas_ruta (dispositivo_retorno_id) where dispositivo_retorno_id is not null;
create index salidas_ruta_activas_idx on public.salidas_ruta (sitio_id) where hora_retorno is null;
create unique index salidas_ruta_placa_activa_key on public.salidas_ruta (sitio_id, vehiculo_placa) where hora_retorno is null;

alter table public.salidas_ruta enable row level security;

create policy "leer salidas_ruta del propio sitio o admin"
  on public.salidas_ruta for select to authenticated
  using (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    or private.es_admin_global()
  );

create policy "crear salidas_ruta del propio sitio"
  on public.salidas_ruta for insert to authenticated
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "actualizar salidas_ruta del propio sitio"
  on public.salidas_ruta for update to authenticated
  using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create function public.salidas_ruta_actualizar_updated_at()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  new.updated_at = now();
  return new;
end;
$function$;

create trigger salidas_ruta_set_updated_at
  before update on public.salidas_ruta
  for each row execute function public.salidas_ruta_actualizar_updated_at();

create function public.salidas_ruta_bloquear_cambios_de_salida()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_salida_id is distinct from old.dispositivo_salida_id
     or new.vehiculo_id is distinct from old.vehiculo_id
     or new.vehiculo_placa is distinct from old.vehiculo_placa
     or new.vehiculo_numero_unidad is distinct from old.vehiculo_numero_unidad
     or new.encargado_id is distinct from old.encargado_id
     or new.encargado_nombre is distinct from old.encargado_nombre
     or new.numero_ruta is distinct from old.numero_ruta
     or new.sub_numero is distinct from old.sub_numero
     or new.numero_documento is distinct from old.numero_documento
     or new.fecha_documento is distinct from old.fecha_documento
     or new.resultado is distinct from old.resultado
     or new.motivo_resultado is distinct from old.motivo_resultado
     or new.hora_salida is distinct from old.hora_salida
     or new.usuario_salida_nombre is distinct from old.usuario_salida_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de salida de una ruta son inmutables';
  end if;
  return new;
end;
$function$;

create trigger salidas_ruta_apertura_inmutable
  before update on public.salidas_ruta
  for each row execute function public.salidas_ruta_bloquear_cambios_de_salida();

create function public.salidas_ruta_bloquear_doble_retorno()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  if old.hora_retorno is not null and new.hora_retorno is distinct from old.hora_retorno then
    raise exception 'El retorno de una ruta solo puede registrarse una vez';
  end if;
  return new;
end;
$function$;

create trigger salidas_ruta_retorno_unico
  before update on public.salidas_ruta
  for each row execute function public.salidas_ruta_bloquear_doble_retorno();

create trigger salidas_ruta_emitir_cambio_nube
  after insert or delete or update on public.salidas_ruta
  for each row execute function private.emitir_cambio_nube_sitio();

-- Suma las 3 tablas nuevas al CHECK de `cola_salida.entidad` local
-- (espejo de MIGRACION_36 del crate Rust) -- no aplica del lado
-- Supabase, no existe una tabla `cola_salida` acá.
