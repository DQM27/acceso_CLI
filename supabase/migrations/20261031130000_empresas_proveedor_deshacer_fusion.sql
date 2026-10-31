-- Deshacer una fusión de empresas de proveedor desde el panel.
--
-- Primer uso real (2026-10-30): el administrador fusionó al revés ("CAYCHA
-- DEL NORTE", la correcta, quedó fusionada en "CAYCHA"). Fusionar no tenía
-- vuelta atrás desde el panel.
--
-- Para devolverle a la empresa sus propios ingresos, la fusión ahora anota en
-- cada ingreso que mueve de qué empresa venía (`empresa_original_id`); el
-- trigger que pasa a la vigente los ingresos que llegan con una fusionada
-- también. `panel_deshacer_fusion_empresa_proveedor` la vuelve a activar, la
-- deja por revisar y le devuelve esos ingresos. Las fusiones anteriores a esta
-- migración no tienen la anotación: al deshacerlas, sus ingresos se quedan en
-- la que quedó (para corregir una fusión al revés alcanza: después se fusiona
-- en el otro sentido y todos pasan a la correcta).
--
-- Las apps no cambian: reciben la empresa sin marca de fusión con el catálogo
-- y la vuelven a ofrecer.

alter table public.ingresos_proveedor
  add column empresa_original_id uuid references public.empresas_proveedor (id);

comment on column public.ingresos_proveedor.empresa_original_id is
  'Con qué empresa se registró el ingreso, si después pasó a otra por una fusión. Nulo: la de empresa_id.';

create index ingresos_proveedor_empresa_original_idx on public.ingresos_proveedor (empresa_original_id)
  where empresa_original_id is not null;

-- El ingreso que llega con una empresa fusionada pasa a la vigente y anota con
-- cuál llegó. Si es un equipo atrasado que vuelve a subir uno que ya estaba
-- en la vigente, no cambia nada.
create or replace function private.ingreso_proveedor_a_empresa_vigente()
returns trigger
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_pedida uuid := new.empresa_id;
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
  if v_destino is distinct from v_pedida then
    new.empresa_id := v_destino;
    new.empresa_nombre := (select e.nombre from public.empresas_proveedor e where e.id = v_destino);
    if tg_op = 'INSERT' or v_destino is distinct from old.empresa_id then
      new.empresa_original_id := coalesce(new.empresa_original_id, v_pedida);
    end if;
  end if;
  return new;
end;
$$;

-- Los datos de entrada siguen inmutables. La empresa (y su anotación) cambia
-- sólo por una fusión (la vieja se fusionó en la nueva) o al deshacerla
-- (vuelve a la empresa con que se registró).
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
    ) or (new.empresa_id = old.empresa_original_id and new.empresa_original_id is null);
  end if;
  if new.sitio_id is distinct from old.sitio_id
     or new.dispositivo_entrada_id is distinct from old.dispositivo_entrada_id
     or new.cedula is distinct from old.cedula
     or new.nombre is distinct from old.nombre
     or (not v_por_fusion and (
           new.empresa_id is distinct from old.empresa_id
           or new.empresa_nombre is distinct from old.empresa_nombre
           or new.empresa_original_id is distinct from old.empresa_original_id))
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

-- Fusionar: igual que antes, anotando de qué empresa venía cada ingreso.
create or replace function public.panel_fusionar_empresa_proveedor(p_origen uuid, p_destino uuid)
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
     set empresa_id = p_destino, empresa_nombre = v_destino.nombre,
         empresa_original_id = coalesce(empresa_original_id, p_origen)
   where empresa_id = p_origen;
  update public.empresas_proveedor
     set revisada_en = coalesce(revisada_en, pg_catalog.now()),
         revisada_por = coalesce(revisada_por, auth.email())
   where id = p_destino;
end;
$$;

-- Deshacer: la empresa vuelve a estar vigente (activa y por revisar) y
-- recupera los ingresos que se registraron con ella.
create function public.panel_deshacer_fusion_empresa_proveedor(p_id uuid)
returns void
language plpgsql
security definer
set search_path to ''
as $$
declare
  v_empresa public.empresas_proveedor%rowtype;
begin
  if not private.es_admin_global() then
    raise exception 'Sólo un administrador del panel puede deshacer fusiones' using errcode = '42501';
  end if;
  select * into v_empresa from public.empresas_proveedor where id = p_id for update;
  if v_empresa.id is null then
    raise exception 'La empresa no existe' using errcode = 'P0002';
  end if;
  if v_empresa.fusionada_en is null then
    raise exception 'Esa empresa no está fusionada' using errcode = '22023';
  end if;

  -- Primero sin la marca: si no, el trigger de la empresa vigente devolvería
  -- los ingresos a la que quedó.
  update public.empresas_proveedor
     set fusionada_en = null, activa = true, revisada_en = null, revisada_por = null
   where id = p_id;
  update public.ingresos_proveedor
     set empresa_id = p_id, empresa_nombre = v_empresa.nombre, empresa_original_id = null
   where empresa_original_id = p_id;
end;
$$;

grant execute on function public.panel_deshacer_fusion_empresa_proveedor(uuid) to authenticated, service_role;
