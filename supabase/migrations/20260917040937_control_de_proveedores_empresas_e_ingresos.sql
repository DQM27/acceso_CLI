-- Control de proveedores (docs/features-futuras/plan-control-proveedores.md)
-- -- primer corte de sync, mismo espejo que rutas/gafetes provisionales
-- KOF. `empresas_proveedor` acotada por sitio, mismo criterio que
-- vehiculos_ruta/encargados_ruta al crearse (una empresa proveedora bien
-- puede repetirse entre sitios, pero cada dispositivo sólo administra la
-- de su propio sitio; no se anticipa acá la corrección "global" que
-- encargados_ruta recibió después). `ingresos_proveedor` es puro
-- entrada/salida como `movimientos_visita` -- NO entrega/devolución como
-- `prestamos_gafete_provisional`, ese vocabulario es exclusivo de KOF.
-- Sin índice único de `gafete_numero` ni de `cedula` a propósito -- igual
-- que `movimientos_visita`/`prestamos_gafete_provisional`, la unicidad
-- real es sólo local (índices parciales en SQLite) y se "simula" contra la
-- nube con una consulta en vivo
-- (`gafete_de_proveedor_ocupado_en_otro_dispositivo`).

-- =====================================================================
-- empresas_proveedor
-- =====================================================================
create table public.empresas_proveedor (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_origen_id uuid not null references public.dispositivos(id),
  nombre text not null,
  activa boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index empresas_proveedor_nombre_key on public.empresas_proveedor (nombre);
create index empresas_proveedor_sitio_idx on public.empresas_proveedor (sitio_id);
create index empresas_proveedor_dispositivo_origen_id_idx on public.empresas_proveedor (dispositivo_origen_id);

alter table public.empresas_proveedor enable row level security;

create policy "leer empresas_proveedor del propio sitio o admin"
  on public.empresas_proveedor for select to authenticated
  using (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    or private.es_admin_global()
  );

create policy "crear empresas_proveedor del propio sitio"
  on public.empresas_proveedor for insert to authenticated
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "actualizar empresas_proveedor del propio sitio"
  on public.empresas_proveedor for update to authenticated
  using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create function public.empresas_proveedor_actualizar_updated_at()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  new.updated_at = now();
  return new;
end;
$function$;

create trigger empresas_proveedor_set_updated_at
  before update on public.empresas_proveedor
  for each row execute function public.empresas_proveedor_actualizar_updated_at();

create trigger empresas_proveedor_emitir_cambio_nube
  after insert or delete or update on public.empresas_proveedor
  for each row execute function private.emitir_cambio_nube_sitio();

-- =====================================================================
-- ingresos_proveedor -- espejo de movimientos_visita (apertura/cierre),
-- vocabulario entrada/salida. Sin catálogo de personas: cedula/nombre son
-- snapshot puro (pedido explícito del usuario, el colaborador de un
-- proveedor nunca se repite). gafete_numero siempre obligatorio (a
-- diferencia de ingresos/movimientos_visita, acá no es condicional).
-- =====================================================================
create table public.ingresos_proveedor (
  id uuid primary key,
  sitio_id uuid not null references public.sitios(id),
  dispositivo_entrada_id uuid not null references public.dispositivos(id),
  cedula text not null,
  nombre text not null,
  empresa_id uuid references public.empresas_proveedor(id),
  empresa_nombre text not null,
  placa text,
  gafete_numero bigint not null,
  hora_entrada timestamptz not null,
  usuario_entrada_nombre text not null,
  hora_salida timestamptz,
  dispositivo_salida_id uuid references public.dispositivos(id),
  usuario_salida_nombre text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (
    (hora_salida is null and dispositivo_salida_id is null and usuario_salida_nombre is null)
    or (hora_salida is not null and usuario_salida_nombre is not null)
  ),
  check (hora_salida is null or hora_salida >= hora_entrada)
);

create index ingresos_proveedor_sitio_idx on public.ingresos_proveedor (sitio_id);
create index ingresos_proveedor_empresa_id_idx on public.ingresos_proveedor (empresa_id) where empresa_id is not null;
create index ingresos_proveedor_dispositivo_entrada_id_idx on public.ingresos_proveedor (dispositivo_entrada_id);
create index ingresos_proveedor_dispositivo_salida_id_idx on public.ingresos_proveedor (dispositivo_salida_id) where dispositivo_salida_id is not null;
create index ingresos_proveedor_activos_idx on public.ingresos_proveedor (sitio_id) where hora_salida is null;

alter table public.ingresos_proveedor enable row level security;

create policy "leer ingresos_proveedor del propio sitio o admin"
  on public.ingresos_proveedor for select to authenticated
  using (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    or private.es_admin_global()
  );

create policy "crear ingresos_proveedor del propio sitio"
  on public.ingresos_proveedor for insert to authenticated
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create policy "actualizar ingresos_proveedor del propio sitio"
  on public.ingresos_proveedor for update to authenticated
  using (sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid)
  with check (
    sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
    and ((select auth.jwt()) ->> 'tipo') <> 'visor'
  );

create function public.ingresos_proveedor_actualizar_updated_at()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  new.updated_at = now();
  return new;
end;
$function$;

create trigger ingresos_proveedor_set_updated_at
  before update on public.ingresos_proveedor
  for each row execute function public.ingresos_proveedor_actualizar_updated_at();

create function public.ingresos_proveedor_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.cedula is distinct from old.cedula
     or new.nombre is distinct from old.nombre
     or new.empresa_id is distinct from old.empresa_id
     or new.empresa_nombre is distinct from old.empresa_nombre
     or new.placa is distinct from old.placa
     or new.gafete_numero is distinct from old.gafete_numero
     or new.hora_entrada is distinct from old.hora_entrada
     or new.usuario_entrada_nombre is distinct from old.usuario_entrada_nombre
     or new.created_at is distinct from old.created_at
  then
    raise exception 'Los datos de entrada de un ingreso de proveedor son inmutables';
  end if;
  return new;
end;
$function$;

create trigger ingresos_proveedor_entrada_inmutable
  before update on public.ingresos_proveedor
  for each row execute function public.ingresos_proveedor_bloquear_cambios_de_entrada();

create function public.ingresos_proveedor_bloquear_doble_salida()
returns trigger
language plpgsql
set search_path to 'public'
as $function$
begin
  if old.hora_salida is not null and new.hora_salida is distinct from old.hora_salida then
    raise exception 'La salida de un ingreso de proveedor solo puede registrarse una vez';
  end if;
  return new;
end;
$function$;

create trigger ingresos_proveedor_salida_unica
  before update on public.ingresos_proveedor
  for each row execute function public.ingresos_proveedor_bloquear_doble_salida();

create trigger ingresos_proveedor_emitir_cambio_nube
  after insert or delete or update on public.ingresos_proveedor
  for each row execute function private.emitir_cambio_nube_sitio();

-- Suma las 2 tablas nuevas al CHECK de `cola_salida.entidad` local
-- (espejo de MIGRACION_41 del crate Rust) -- no aplica del lado
-- Supabase, no existe una tabla `cola_salida` acá.
