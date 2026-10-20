-- Ejecutar en una sola sesión. Todo se revierte.
-- Cubre 20261020130000_permisos_api_explicitos: `anon` no tiene nada sobre las
-- tablas, vistas, secuencias ni funciones de `public` (salvo la telemetría de
-- staging); `public` no ejecuta ninguna función; `authenticated` conserva lo
-- que usan las apps, el panel y Realtime; lo cerrado a propósito sigue
-- cerrado; y lo nuevo que cree una migración no recibe permisos solo.
begin;

-- 1. anon: nada sobre tablas, vistas ni secuencias.
do $$
declare
  v record;
begin
  for v in
    select c.oid, c.relname, c.relkind
      from pg_class c
      join pg_namespace n on n.oid = c.relnamespace
      left join pg_depend d on d.objid = c.oid and d.deptype = 'e'
     where n.nspname = 'public' and d.objid is null
       and c.relkind in ('r', 'v', 'm', 'f', 'p', 'S')
       and c.relname not in ('telemetria_diagnostico')
  loop
    if v.relkind = 'S' then
      if has_sequence_privilege('anon', v.oid, 'usage') or has_sequence_privilege('anon', v.oid, 'select')
         or has_sequence_privilege('anon', v.oid, 'update') then
        raise exception '1: anon tiene permisos sobre la secuencia %', v.relname;
      end if;
    elsif has_table_privilege('anon', v.oid, 'select') or has_table_privilege('anon', v.oid, 'insert')
       or has_table_privilege('anon', v.oid, 'update') or has_table_privilege('anon', v.oid, 'delete')
       or has_table_privilege('anon', v.oid, 'truncate') or has_table_privilege('anon', v.oid, 'references')
       or has_table_privilege('anon', v.oid, 'trigger') then
      raise exception '1: anon tiene permisos sobre %', v.relname;
    end if;
  end loop;
end $$;

-- 2. anon no ejecuta ninguna función, y `public` (que heredan todos) tampoco.
do $$
declare
  v record;
begin
  for v in
    select p.oid, p.oid::regprocedure as firma, p.proowner, p.proacl
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      left join pg_depend d on d.objid = p.oid and d.deptype = 'e'
     where n.nspname = 'public' and d.objid is null
       and p.proname not in ('reglas_telemetria_vigentes')
  loop
    if has_function_privilege('anon', v.oid, 'execute') then
      raise exception '2: anon ejecuta %', v.firma;
    end if;
    if aclcontains(coalesce(v.proacl, acldefault('f', v.proowner)),
                   makeaclitem(0, v.proowner, 'EXECUTE', false)) then
      raise exception '2: public ejecuta %', v.firma;
    end if;
  end loop;
end $$;

-- 3. authenticated conserva lo que usan las apps y Realtime: leer y escribir
--    las tablas de operación, y leer todas las publicadas en Realtime.
do $$
declare
  v record;
begin
  for v in select tablename from pg_publication_tables
            where pubname = 'supabase_realtime' and schemaname = 'public'
  loop
    if not has_table_privilege('authenticated', 'public.' || v.tablename, 'select') then
      raise exception '3: authenticated ya no lee % (publicada en Realtime)', v.tablename;
    end if;
  end loop;
  if not has_table_privilege('authenticated', 'public.ingresos', 'insert')
     or not has_table_privilege('authenticated', 'public.ingresos', 'update')
     or not has_table_privilege('authenticated', 'public.contratistas', 'insert')
     or not has_table_privilege('authenticated', 'public.gafetes', 'update') then
    raise exception '3: authenticated perdió escritura en las tablas de operación';
  end if;
end $$;

-- 4. Las RPC de las apps y del panel siguen abiertas para authenticated y para
--    el rol de servicio; lo cerrado a propósito sigue cerrado.
do $$
declare
  v_rpc text;
begin
  foreach v_rpc in array array[
    'public.sesion_usuario_en_unidad(text, uuid, bigint)',
    'public.cerrar_sesion_usuario_en_unidad(text)',
    'public.hora_servidor_ms()',
    'public.ingreso_activo_de_contratista(text)',
    'public.persona_adentro_por_otra_via(text, text)',
    'public.contratistas_activos_en_otras_unidades(text[])',
    'public.registrar_token_push(text)',
    'public.panel_buscar_movimientos(text, timestamptz, timestamptz, uuid[])'
  ] loop
    if not has_function_privilege('authenticated', v_rpc, 'execute') then
      raise exception '4: authenticated ya no ejecuta %', v_rpc;
    end if;
    if not has_function_privilege('service_role', v_rpc, 'execute') then
      raise exception '4: service_role ya no ejecuta %', v_rpc;
    end if;
  end loop;
  if has_function_privilege('authenticated', 'public.sync_access_policy()', 'execute')
     or has_function_privilege('authenticated',
          'public.canjear_codigo_vinculacion(text, jsonb, text, jsonb, text)', 'execute') then
    raise exception '4: se reabrió a authenticated una función cerrada a propósito';
  end if;
end $$;

-- 5. Lo nuevo no recibe permisos solo (default privileges revocados).
create table public.prueba_permisos_por_defecto (id int primary key);
create sequence public.prueba_permisos_por_defecto_seq;
create function public.prueba_permisos_por_defecto_fn() returns int
  language sql as $$ select 1 $$;
do $$
declare
  v_rol text;
begin
  foreach v_rol in array array['anon', 'authenticated', 'service_role'] loop
    if has_table_privilege(v_rol, 'public.prueba_permisos_por_defecto', 'select')
       or has_table_privilege(v_rol, 'public.prueba_permisos_por_defecto', 'insert') then
      raise exception '5: una tabla nueva le dio permisos a % sola', v_rol;
    end if;
    if has_sequence_privilege(v_rol, 'public.prueba_permisos_por_defecto_seq', 'usage') then
      raise exception '5: una secuencia nueva le dio permisos a % sola', v_rol;
    end if;
    if has_function_privilege(v_rol, 'public.prueba_permisos_por_defecto_fn()', 'execute') then
      raise exception '5: una función nueva le dio EXECUTE a % sola', v_rol;
    end if;
  end loop;
end $$;

-- 6. En la práctica: anon recibe "permission denied" en la API, no una lista vacía.
set local role anon;
do $$
begin
  perform 1 from public.ingresos limit 1;
  raise exception '6: anon pudo consultar ingresos';
exception when insufficient_privilege then null;
end $$;
do $$
begin
  perform public.hora_servidor_ms();
  raise exception '6: anon pudo ejecutar hora_servidor_ms';
exception when insufficient_privilege then null;
end $$;
reset role;

rollback;
