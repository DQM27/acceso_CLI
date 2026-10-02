-- Contratistas y empresas son un catálogo GLOBAL: una persona entra por
-- cualquier unidad operativa y negarle el acceso vale en todas a la vez. Aun
-- así cada fila llevaba la unidad donde se creó (`sitio_id`), y eso sólo
-- servía para dos cosas, las dos en contra:
--
-- 1. El aviso en vivo (`cambio_nube`) salía sólo por el canal de ESA unidad.
--    Si se negaba el acceso a un contratista, las demás unidades no se
--    enteraban al instante: esperaban a su próxima sincronización.
-- 2. La política de alta obligaba al equipo a poner su propia unidad, y el
--    panel (que no tiene unidad) tenía que inventar una.
--
-- Desde ahora:
-- - `contratistas` y `empresas` ya no tienen `sitio_id`.
-- - El aviso en vivo de contratistas y empresas llega a TODAS las unidades.
-- - Cualquier equipo vinculado puede dar de alta, sin atarlo a su unidad.
--
-- `sitio_id` SIGUE en las tablas que sí son de una unidad: `ingresos`,
-- `ingresos_proveedor`, `prestamos_gafete_provisional`, `empresas_proveedor`
-- y `gafetes`.
--
-- Las apps dejaron de mandar la columna en la misma entrega (`cola.rs`,
-- `construir_cuerpo_contratista`/`construir_cuerpo_empresa`) y nunca la
-- leyeron (`catalogo.rs`). Una app anterior a esta entrega fallaría al subir
-- un alta; no hay ninguna en uso porque todos los equipos se reinstalan con
-- el registro de dispositivos por código.

-- Alta desde cualquier equipo -----------------------------------------------
-- Va antes de borrar la columna: las políticas viejas dependen de `sitio_id`.

drop policy if exists "crear contratistas del propio sitio" on public.contratistas;
create policy "crear contratistas (global)" on public.contratistas
  for insert to authenticated
  with check (((select auth.jwt()) ->> 'sitio_id') is not null);

drop policy if exists "crear empresas del propio sitio" on public.empresas;
create policy "crear empresas (global)" on public.empresas
  for insert to authenticated
  with check (((select auth.jwt()) ->> 'sitio_id') is not null);

alter table public.contratistas drop column sitio_id;
alter table public.empresas drop column sitio_id;

-- Aviso en vivo a todas las unidades ----------------------------------------

-- Mismo mensaje que `private.emitir_cambio_nube_sitio`, repetido por el canal
-- de cada unidad. Son pocas unidades y los cambios de catálogo son
-- esporádicos, así que el costo es mínimo.
create or replace function private.emitir_cambio_nube_global()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_fila jsonb;
  v_dispositivo_id text;
  v_sitio uuid;
begin
  if tg_op = 'DELETE' then
    v_fila := pg_catalog.to_jsonb(old);
  else
    v_fila := pg_catalog.to_jsonb(new);
  end if;
  v_dispositivo_id := (select auth.jwt() ->> 'sub');

  for v_sitio in select id from public.sitios loop
    perform realtime.send(
      pg_catalog.jsonb_build_object(
        'schema', tg_table_schema,
        'table', tg_table_name,
        'operation', tg_op,
        'sitio_id', v_sitio,
        'dispositivo_id', v_dispositivo_id,
        'changed_at', pg_catalog.now(),
        'id', v_fila ->> 'id',
        'registro', null
      ),
      'cambio_nube',
      'sitio:' || v_sitio::text,
      true
    );
  end loop;

  return null;
end;
$$;

revoke all on function private.emitir_cambio_nube_global() from public;

drop trigger if exists contratistas_emitir_cambio_nube on public.contratistas;
create trigger contratistas_emitir_cambio_nube
  after insert or update or delete on public.contratistas
  for each row execute function private.emitir_cambio_nube_global();

drop trigger if exists empresas_emitir_cambio_nube on public.empresas;
create trigger empresas_emitir_cambio_nube
  after insert or update or delete on public.empresas
  for each row execute function private.emitir_cambio_nube_global();

-- El panel crea sin unidad --------------------------------------------------

drop function if exists public.panel_crear_empresa(text, uuid);
drop function if exists public.panel_crear_contratista(text, text, uuid, text, date, boolean, uuid);

-- Crea una empresa (de contratistas) o devuelve la que ya existe con ese nombre
-- (sin importar tildes ni mayúsculas): así crearla dos veces no duplica nada.
create function public.panel_crear_empresa(p_nombre text)
returns public.empresas
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_nombre text;
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

  insert into public.empresas (id, dispositivo_origen_id, nombre, activa)
  values (gen_random_uuid(), null, v_nombre, true)
  returning * into v_empresa;
  return v_empresa;
end;
$$;

-- Crea un contratista. `p_con_acceso = false` lo deja con el acceso denegado
-- (el bloqueo). Devuelve la fila creada. Reglas gemelas de `domain::cedula`,
-- `domain::contratista` y `ContratistaService::armar`.
create function public.panel_crear_contratista(
  p_cedula text,
  p_nombre text,
  p_empresa_id uuid,
  p_tipo_ingreso text,
  p_fecha_vencimiento_praind date default null,
  p_con_acceso boolean default true
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

  -- Cédula repetida en cualquier formato (1-1234-0567 = 112340567). Usa el
  -- índice `contratistas_cedula_normalizada_key`, que además frena dos altas
  -- simultáneas.
  if exists (
    select 1 from public.contratistas where public.normalizar_cedula(identificacion) = v_cedula
  ) then
    raise exception 'La cédula del contratista ya existe';
  end if;

  -- `id` no tiene valor por defecto: cada equipo genera el suyo, y el panel también.
  insert into public.contratistas (
    id, dispositivo_origen_id, nombre, identificacion, activo,
    empresa_id, empresa_nombre, tipo_ingreso, fecha_vencimiento_praind, es_personal_ruta
  ) values (
    gen_random_uuid(), null, v_nombre, v_cedula, coalesce(p_con_acceso, true),
    v_empresa.id, v_empresa.nombre, p_tipo_ingreso, p_fecha_vencimiento_praind, false
  )
  returning * into v_fila;
  return v_fila;
end;
$$;

-- Sólo usuarios autenticados las llaman (y adentro se exige ser administrador
-- del panel); ni anónimos ni el rol público.
revoke all on function public.panel_crear_empresa(text) from public, anon;
revoke all on function public.panel_crear_contratista(text, text, uuid, text, date, boolean) from public, anon;
grant execute on function public.panel_crear_empresa(text) to authenticated;
grant execute on function public.panel_crear_contratista(text, text, uuid, text, date, boolean) to authenticated;
