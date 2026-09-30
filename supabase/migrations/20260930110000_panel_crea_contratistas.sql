-- El panel puede dar de alta un contratista (y, si hace falta, su empresa).
--
-- Para qué: negar el acceso a alguien que nunca fue contratista (un proveedor,
-- por ejemplo) se hace dándolo de alta como contratista con el acceso apagado
-- (ver docs/features-futuras/plan-veto-por-persona.md, "Límite conocido"). Hasta
-- ahora sólo un equipo podía crear filas en `contratistas`/`empresas` (RLS de
-- INSERT por sitio del JWT), así que el panel no tenía cómo.
--
-- Por qué funciones y no una política de INSERT directa: las reglas del núcleo
-- (cédula en forma única, sólo números de 9 a 13 dígitos, nombre en mayúsculas,
-- PRAIND según el tipo) tienen que aplicarse igual desde acá; en una función
-- quedan en un solo lugar y con mensajes en español. Son gemelas de
-- `domain::cedula`, `domain::contratista` y `ContratistaService::armar`.
--
-- Origen de la fila: un equipo fija `dispositivo_origen_id` al crearla; el panel
-- no tiene equipo, así que esa columna pasa a admitir null (como ya la admite
-- `usuarios`). Los equipos no la leen al recibir el catálogo. `sitio_id` sí
-- queda: es la unidad por cuyo canal en vivo se avisa el cambio; las demás
-- unidades lo reciben en su siguiente sincronización.

alter table public.contratistas alter column dispositivo_origen_id drop not null;
alter table public.empresas alter column dispositivo_origen_id drop not null;

-- Forma única de una cédula: sin espacios, guiones ni puntos, en mayúsculas y
-- sin el cero inicial del formato del TSE (10 dígitos que empiezan en 0).
-- `null` = rechazada (vacía, con símbolos o de más de 20 caracteres). Casos
-- compartidos con el núcleo en tests/vectores_cedula.tsv.
create or replace function public.normalizar_cedula(p_texto text)
returns text
language sql
immutable
parallel safe
set search_path = ''
as $$
  select case
    when limpia = '' then null
    when limpia !~ '^[A-Z0-9]+$' then null
    when length(limpia) > 20 then null
    when limpia ~ '^0[0-9]{9}$' then substr(limpia, 2)
    else limpia
  end
  from (
    select upper(regexp_replace(coalesce(p_texto, ''), '[[:space:]\-.]', '', 'g')) as limpia
  ) as t
$$;

comment on function public.normalizar_cedula(text) is
  'Forma única de una cédula (gemela de domain::cedula::Cedula::normalizar); null si se rechaza.';

-- Crea una empresa (de contratistas) o devuelve la que ya existe con ese nombre
-- (sin importar tildes ni mayúsculas): así crearla dos veces no duplica nada.
create or replace function public.panel_crear_empresa(p_nombre text, p_sitio_id uuid default null)
returns public.empresas
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_nombre text;
  v_sitio uuid;
  v_empresa public.empresas;
begin
  if not private.es_admin_global() then
    raise exception 'No tiene permiso para crear empresas.' using errcode = '42501';
  end if;

  v_nombre := upper(btrim(regexp_replace(coalesce(p_nombre, ''), '[[:space:]]+', ' ', 'g')));
  if v_nombre = '' then
    raise exception 'El nombre de la empresa es obligatorio';
  end if;

  select * into v_empresa
  from public.empresas
  where public.plegar_texto(nombre) = public.plegar_texto(v_nombre)
  limit 1;
  if found then
    return v_empresa;
  end if;

  v_sitio := coalesce(p_sitio_id, (select id from public.sitios order by created_at limit 1));
  if v_sitio is null then
    raise exception 'No hay ninguna unidad operativa configurada todavía.';
  end if;

  insert into public.empresas (id, sitio_id, dispositivo_origen_id, nombre, activa)
  values (gen_random_uuid(), v_sitio, null, v_nombre, true)
  returning * into v_empresa;
  return v_empresa;
end;
$$;

-- Crea un contratista. `p_con_acceso = false` lo deja con el acceso denegado
-- (el bloqueo). Devuelve la fila creada.
create or replace function public.panel_crear_contratista(
  p_cedula text,
  p_nombre text,
  p_empresa_id uuid,
  p_tipo_ingreso text,
  p_fecha_vencimiento_praind date default null,
  p_con_acceso boolean default true,
  p_sitio_id uuid default null
)
returns public.contratistas
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_cedula text;
  v_nombre text;
  v_empresa public.empresas;
  v_sitio uuid;
  v_hoy date := (now() at time zone 'America/Costa_Rica')::date;
  v_fila public.contratistas;
begin
  if not private.es_admin_global() then
    raise exception 'No tiene permiso para crear contratistas.' using errcode = '42501';
  end if;

  -- Cédula: obligatoria, en forma única y sólo números de 9 a 13 dígitos
  -- (contratistas y proveedores no admiten pasaporte).
  if btrim(coalesce(p_cedula, '')) = '' then
    raise exception 'La cédula es obligatoria';
  end if;
  v_cedula := public.normalizar_cedula(p_cedula);
  if v_cedula is null or v_cedula !~ '^[0-9]{9,13}$' then
    raise exception 'La cédula debe tener sólo números, entre 9 y 13 dígitos';
  end if;

  -- Nombre: sólo letras (con tildes), espacios, apóstrofo y guion; en mayúsculas.
  v_nombre := btrim(regexp_replace(coalesce(p_nombre, ''), '[[:space:]]+', ' ', 'g'));
  if v_nombre = '' then
    raise exception 'El nombre es obligatorio';
  end if;
  if v_nombre !~ '^[[:alpha:] ''-]+$' then
    raise exception 'El nombre no puede tener números ni símbolos';
  end if;
  v_nombre := upper(v_nombre);

  -- Empresa: tiene que existir. Los equipos descartan un contratista sin una
  -- empresa que puedan resolver, y el bloqueo no les llegaría.
  select * into v_empresa from public.empresas where id = p_empresa_id;
  if not found then
    raise exception 'Empresa no encontrada';
  end if;

  if p_tipo_ingreso is null or p_tipo_ingreso not in ('PRAIND', 'IN_HOUSE', 'POR_CORREO', 'SWAT') then
    raise exception 'El tipo de ingreso no es válido';
  end if;

  -- PRAIND: obligatorio y vigente para PRAIND e IN HOUSE. A quien se crea con
  -- el acceso denegado no se le pide: no va a entrar de todos modos.
  if p_con_acceso and p_tipo_ingreso in ('PRAIND', 'IN_HOUSE') then
    if p_fecha_vencimiento_praind is null then
      raise exception 'La fecha de PRAIND es obligatoria';
    end if;
    if p_fecha_vencimiento_praind < v_hoy then
      raise exception 'El PRAIND está vencido';
    end if;
  end if;

  -- Cédula repetida: se compara en forma única, así que 1-1234-0567 y
  -- 112340567 cuentan como la misma persona (también contra filas viejas).
  if exists (
    select 1 from public.contratistas where public.normalizar_cedula(identificacion) = v_cedula
  ) then
    raise exception 'La cédula del contratista ya existe';
  end if;

  v_sitio := coalesce(p_sitio_id, (select id from public.sitios order by created_at limit 1));
  if v_sitio is null then
    raise exception 'No hay ninguna unidad operativa configurada todavía.';
  end if;

  -- `id` no tiene valor por defecto: cada equipo genera el suyo, y el panel también.
  insert into public.contratistas (
    id, sitio_id, dispositivo_origen_id, nombre, identificacion, activo,
    empresa_id, empresa_nombre, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta
  ) values (
    gen_random_uuid(), v_sitio, null, v_nombre, v_cedula, coalesce(p_con_acceso, true),
    v_empresa.id, v_empresa.nombre, p_tipo_ingreso, p_fecha_vencimiento_praind, false
  )
  returning * into v_fila;
  return v_fila;
end;
$$;

-- Sólo usuarios autenticados las llaman (y adentro se exige ser administrador
-- del panel); ni anónimos ni el rol público.
revoke all on function public.panel_crear_empresa(text, uuid) from public, anon;
revoke all on function public.panel_crear_contratista(text, text, uuid, text, date, boolean, uuid) from public, anon;
grant execute on function public.panel_crear_empresa(text, uuid) to authenticated;
grant execute on function public.panel_crear_contratista(text, text, uuid, text, date, boolean, uuid) to authenticated;
