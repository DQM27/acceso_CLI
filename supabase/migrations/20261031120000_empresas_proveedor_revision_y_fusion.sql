-- Empresas de proveedor en orden: revisión y fusión desde el panel.
--
-- Incidente 2026-10-30: en dos meses, el catálogo de proveedores de Brisas
-- juntó 5 duplicados mal escritos en la portería ("FUDDY FACTORY" /
-- "FOODIE FACTORY", "PROCESOS FRESCOS" / "PROCESADOS FRESCOS", ...). La
-- portería sigue pudiendo crear empresas (las apps ahora sugieren las
-- parecidas antes), y el administrador ordena el catálogo desde el panel:
--
-- 1. Revisión: lo que se crea en la portería queda "por revisar"
--    (`revisada_en` nulo) hasta que el administrador lo confirma o lo fusiona.
--    Las que ya existían quedan por revisar a propósito: es la limpieza
--    inicial.
-- 2. Fusión (`panel_fusionar_empresa_proveedor`): la empresa mal escrita no se
--    borra, queda marcada `fusionada_en` la que queda y desactivada. Sus
--    ingresos pasan a la que queda (empresa y nombre de empresa).
--    Los equipos tienen copia del catálogo: con la marca dejan de ofrecerla
--    (llega por `catalogo_del_sitio`), y si un equipo atrasado igual sube un
--    ingreso con la empresa fusionada, `ingresos_proveedor_empresa_fusionada`
--    lo pasa a la que queda. Borrarla habría hecho que la autorreparación de
--    la cola la volviera a crear.
-- 3. Nombre único por unidad: `(sitio_id, nombre)` y `(sitio_id,
--    plegar_texto(nombre))`. Las apps nuevas suben con
--    `on_conflict=sitio_id,nombre`. La unicidad global
--    (`empresas_proveedor_nombre_key`, `idx_empresas_proveedor_nombre_plegado`)
--    se queda HASTA QUE TODOS LOS EQUIPOS TENGAN la versión nueva (móvil
--    1.8.11, escritorio 1.10.5): las anteriores suben con `on_conflict=nombre`
--    y sin ese índice la nube les rechazaría toda alta. Se saca en una
--    migración aparte.
--
-- Datos de entrada inmutables: `ingresos_proveedor_bloquear_cambios_de_entrada`
-- ahora deja cambiar la empresa de un ingreso en un único caso, que la vieja
-- se haya fusionado en la nueva.

alter table public.empresas_proveedor
  add column revisada_en timestamptz,
  add column revisada_por text,
  add column fusionada_en uuid references public.empresas_proveedor (id),
  add constraint empresas_proveedor_no_se_fusiona_consigo check (fusionada_en is distinct from id);

comment on column public.empresas_proveedor.revisada_en is
  'Cuándo el administrador del panel la confirmó o la fusionó. Nulo: por revisar (creada en la portería).';
comment on column public.empresas_proveedor.fusionada_en is
  'La empresa en que el administrador la fusionó (era la misma, mal escrita). Nulo: vigente.';

create index empresas_proveedor_fusionada_en_idx on public.empresas_proveedor (fusionada_en)
  where fusionada_en is not null;

-- 3. Unicidad por unidad (la global se saca después, ver arriba).
create unique index empresas_proveedor_sitio_nombre_key on public.empresas_proveedor (sitio_id, nombre);
create unique index empresas_proveedor_sitio_nombre_plegado_key
  on public.empresas_proveedor (sitio_id, public.plegar_texto(nombre));

-- 2. Un ingreso que llega con una empresa fusionada pasa a la que quedó (al
-- crearse, o si un equipo atrasado lo vuelve a subir). Corre antes que
-- `ingresos_proveedor_entrada_inmutable` (los BEFORE van por orden alfabético
-- del nombre: "empresa_fusionada" < "entrada_inmutable").
create function private.ingreso_proveedor_a_empresa_vigente()
returns trigger
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_destino uuid := new.empresa_id;
  v_siguiente uuid;
  v_vueltas integer := 0;
begin
  if new.empresa_id is null then
    return new;
  end if;
  loop
    select e.fusionada_en into v_siguiente
      from public.empresas_proveedor e
     where e.id = v_destino;
    exit when v_siguiente is null or v_vueltas >= 10;
    v_destino := v_siguiente;
    v_vueltas := v_vueltas + 1;
  end loop;
  if v_destino is distinct from new.empresa_id then
    new.empresa_id := v_destino;
    new.empresa_nombre := (select e.nombre from public.empresas_proveedor e where e.id = v_destino);
  end if;
  return new;
end;
$$;

create trigger ingresos_proveedor_empresa_fusionada
  before insert or update of empresa_id on public.ingresos_proveedor
  for each row execute function private.ingreso_proveedor_a_empresa_vigente();

-- Los datos de entrada siguen inmutables, salvo la empresa cuando la vieja se
-- fusionó en la nueva. Resto, tal como estaba.
create or replace function public.ingresos_proveedor_bloquear_cambios_de_entrada()
returns trigger
language plpgsql
set search_path to 'public'
as $$
declare
  v_por_fusion boolean := false;
begin
  if new.empresa_id is distinct from old.empresa_id then
    v_por_fusion := exists (
      select 1 from public.empresas_proveedor e
       where e.id = old.empresa_id and e.fusionada_en = new.empresa_id
    );
  end if;
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.cedula is distinct from old.cedula
     or new.nombre is distinct from old.nombre
     or (not v_por_fusion and (
           new.empresa_id is distinct from old.empresa_id
           or new.empresa_nombre is distinct from old.empresa_nombre))
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
$$;

-- Los equipos reciben la marca de fusión con su catálogo. Resto, tal como
-- estaba (`20261020140000_catalogo_del_sitio`).
create or replace function public.catalogo_del_sitio(
  p_desde timestamptz default null,
  p_desde_gafetes timestamptz default null,
  p_limite integer default 500
)
returns jsonb
language sql
stable
set search_path to ''
as $$
  with parametros as (
    select
      least(greatest(coalesce(p_limite, 500), 1), 1000) as limite,
      ((select auth.jwt()) ->> 'sitio_id')::uuid as sitio_id
  )
  select pg_catalog.jsonb_build_object(
    'empresas', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select e.id, e.nombre, e.activa, e.updated_at
          from public.empresas e
         where p_desde is null or e.updated_at > p_desde
         order by e.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'contratistas', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select c.id, c.nombre, c.identificacion, c.empresa_id, c.empresa_nombre, c.activo,
               c.tipo_ingreso, c.fecha_vencimiento_praind, c.es_personal_ruta, c.updated_at
          from public.contratistas c
         where p_desde is null or c.updated_at > p_desde
         order by c.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'usuarios', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select u.id, u.cedula, u.nombre, u.rol, u.activo, u.updated_at
          from public.usuarios u
         where p_desde is null or u.updated_at > p_desde
         order by u.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'gafetes', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select g.id, g.numero, g.tipo, g.estado, g.contratista_portador_id,
               g.contratista_portador_nombre, g.visita_portador_id, g.updated_at
          from public.gafetes g
         where g.sitio_id = (select sitio_id from parametros)
           and (p_desde_gafetes is null or g.updated_at > p_desde_gafetes)
         order by g.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'rutas', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select r.id, r.numero, r.activo, r.updated_at
          from public.rutas r
         where r.sitio_id = (select sitio_id from parametros)
           and (p_desde is null or r.updated_at > p_desde)
         order by r.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb),
    'empresas_proveedor', coalesce((
      select pg_catalog.jsonb_agg(f order by f.id) from (
        select ep.id, ep.nombre, ep.activa, ep.updated_at, ep.fusionada_en
          from public.empresas_proveedor ep
         where ep.sitio_id = (select sitio_id from parametros)
           and (p_desde is null or ep.updated_at > p_desde)
         order by ep.id
         limit (select limite from parametros)
      ) f), '[]'::jsonb)
  );
$$;

-- El catálogo de proveedores para el panel: por revisar, confirmadas y
-- fusionadas, con cuántos ingresos tiene cada una y desde qué equipo se creó.
create view public.panel_empresas_proveedor
with (security_invoker = true) as
select
  e.id,
  e.sitio_id,
  s.nombre as sitio_nombre,
  e.nombre,
  e.activa,
  e.created_at,
  e.revisada_en,
  e.revisada_por,
  e.fusionada_en,
  f.nombre as fusionada_en_nombre,
  d.etiqueta as creada_por_equipo,
  (select count(*) from public.ingresos_proveedor i where i.empresa_id = e.id) as ingresos,
  (select max(i.hora_entrada) from public.ingresos_proveedor i where i.empresa_id = e.id) as ultimo_ingreso
from public.empresas_proveedor e
join public.sitios s on s.id = e.sitio_id
left join public.empresas_proveedor f on f.id = e.fusionada_en
left join public.dispositivos d on d.id = e.dispositivo_origen_id;

grant select on public.panel_empresas_proveedor to authenticated, service_role;

-- Confirmar: es una empresa nueva de verdad.
create function public.panel_confirmar_empresa_proveedor(p_id uuid)
returns void
language plpgsql
security definer
set search_path to ''
as $$
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador del panel puede revisar empresas' using errcode = '42501';
  end if;
  update public.empresas_proveedor
     set revisada_en = pg_catalog.now(), revisada_por = auth.email()
   where id = p_id and fusionada_en is null;
  if not found then
    raise exception 'La empresa no existe o ya fue fusionada' using errcode = 'P0002';
  end if;
end;
$$;

-- Fusionar: `p_origen` era la misma empresa que `p_destino`, mal escrita.
create function public.panel_fusionar_empresa_proveedor(p_origen uuid, p_destino uuid)
returns void
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_origen public.empresas_proveedor%rowtype;
  v_destino public.empresas_proveedor%rowtype;
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador del panel puede fusionar empresas' using errcode = '42501';
  end if;
  if p_origen = p_destino then
    raise exception 'Una empresa no se fusiona consigo misma' using errcode = '22023';
  end if;
  -- Siempre en el mismo orden, para que dos fusiones a la vez no se traben.
  perform 1 from public.empresas_proveedor
   where id in (p_origen, p_destino) order by id for update;
  select * into v_origen from public.empresas_proveedor where id = p_origen;
  select * into v_destino from public.empresas_proveedor where id = p_destino;
  if v_origen.id is null or v_destino.id is null then
    raise exception 'La empresa no existe' using errcode = 'P0002';
  end if;
  if v_origen.sitio_id <> v_destino.sitio_id then
    raise exception 'Las dos empresas tienen que ser de la misma unidad' using errcode = '22023';
  end if;
  if v_origen.fusionada_en is not null then
    raise exception 'Esa empresa ya fue fusionada' using errcode = '22023';
  end if;
  if v_destino.fusionada_en is not null then
    raise exception 'No se puede fusionar en una empresa que ya fue fusionada' using errcode = '22023';
  end if;

  -- Primero la marca: con ella el trigger de datos inmutables deja mover los
  -- ingresos, y los equipos dejan de ofrecerla al recibir el catálogo.
  update public.empresas_proveedor
     set fusionada_en = p_destino, activa = false,
         revisada_en = pg_catalog.now(), revisada_por = auth.email()
   where id = p_origen;
  -- Las que ya se habían fusionado en la de origen pasan a la de destino.
  update public.empresas_proveedor set fusionada_en = p_destino where fusionada_en = p_origen;
  update public.ingresos_proveedor
     set empresa_id = p_destino, empresa_nombre = v_destino.nombre
   where empresa_id = p_origen;
  update public.empresas_proveedor
     set revisada_en = coalesce(revisada_en, pg_catalog.now()),
         revisada_por = coalesce(revisada_por, auth.email())
   where id = p_destino;
end;
$$;

grant execute on function public.panel_confirmar_empresa_proveedor(uuid) to authenticated, service_role;
grant execute on function public.panel_fusionar_empresa_proveedor(uuid, uuid) to authenticated, service_role;

-- El asistente del lienzo ve la fusión y la revisión: una empresa fusionada
-- es otro nombre (mal escrito) de la que quedó y no se cuenta aparte. Agregar
-- columnas al final no cambia los permisos de la vista.
create or replace view ia.empresas_proveedor as
  select id, sitio_id, nombre, activa, created_at, updated_at, fusionada_en, revisada_en
    from public.empresas_proveedor;
comment on view ia.empresas_proveedor is
  'Empresas proveedoras, por sitio. Sólo para preguntas por EMPRESAS; "proveedores" a secas son personas (ia.ingresos_proveedor). fusionada_en no nulo: era otro nombre (mal escrito) de la empresa fusionada_en, ya sin ingresos propios; no contarla como empresa distinta. revisada_en nulo: creada en la portería y todavía sin revisar por el administrador.';
