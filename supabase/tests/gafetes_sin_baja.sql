-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Migración 20261116140000_gafetes_sin_baja: el espejo sólo admite
-- DISPONIBLE y PERDIDO, un DE_BAJA de una app sin actualizar se guarda como
-- PERDIDO, y un PERDIDO sin responsable se acepta.
begin;

insert into public.sitios (id, nombre) values (gen_random_uuid(), 'Diagnóstico gafetes sin baja');
select set_config('diagnostico.sitio', (select id::text from public.sitios where nombre = 'Diagnóstico gafetes sin baja'), true);
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid, 'pc', 'Diagnóstico PC sin baja', '{"kty":"EC"}', 'diag-huella-sin-baja');
select set_config('diagnostico.dispositivo', (select id::text from public.dispositivos where etiqueta = 'Diagnóstico PC sin baja'), true);

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sub', current_setting('diagnostico.dispositivo'), 'huella', 'diag-huella-sin-baja', 'sitio_id', current_setting('diagnostico.sitio'))::text,
  true);

-- 1. Una app vieja que manda DE_BAJA (alta o cambio) queda guardada como PERDIDO.
do $$
begin
  insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, tipo, estado)
  values (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid, current_setting('diagnostico.dispositivo')::uuid, 998001, 'CONTRATISTA', 'DE_BAJA');
  if (select estado from public.gafetes where numero = 998001) <> 'PERDIDO' then
    raise exception '1: un alta con DE_BAJA no quedó PERDIDO';
  end if;

  insert into public.gafetes (id, sitio_id, dispositivo_origen_id, numero, tipo, estado)
  values (gen_random_uuid(), current_setting('diagnostico.sitio')::uuid, current_setting('diagnostico.dispositivo')::uuid, 998002, 'CONTRATISTA', 'DISPONIBLE');
  update public.gafetes set estado = 'DE_BAJA' where numero = 998002;
  if (select estado from public.gafetes where numero = 998002) <> 'PERDIDO' then
    raise exception '1: un cambio a DE_BAJA no quedó PERDIDO';
  end if;
end $$;

-- 2. PERDIDO sin responsable se acepta; vuelve a DISPONIBLE al recuperarse.
do $$
begin
  update public.gafetes set estado = 'DISPONIBLE' where numero = 998002;
  update public.gafetes set estado = 'PERDIDO', contratista_portador_id = null where numero = 998002;
  if not found then
    raise exception '2: no se pudo dejar un gafete PERDIDO sin responsable';
  end if;
end $$;

-- 3. Cualquier otro estado se rechaza.
do $$
begin
  begin
    update public.gafetes set estado = 'EXTRAVIADO' where numero = 998002;
    raise exception '3: se aceptó un estado desconocido';
  exception
    when check_violation then null;
  end;
end $$;

reset role;

-- 4. No quedó ningún DE_BAJA en la tabla, y la función del disparador no la
--    ejecuta cualquiera.
do $$
begin
  if exists (select 1 from public.gafetes where estado = 'DE_BAJA') then
    raise exception '4: quedan gafetes DE_BAJA';
  end if;
  if has_function_privilege('anon', 'private.gafetes_baja_es_perdido()', 'execute') then
    raise exception '4: anon puede ejecutar la función del disparador';
  end if;
end $$;

select '4 comprobaciones de gafetes sin baja correctas' as resultado;
rollback;
