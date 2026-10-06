-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre la creación desde el panel: normalizar_cedula, panel_crear_empresa y,
-- para contratistas, lo que sigue siendo de la base desde 20261004170000 (el
-- alta la hace la Edge Function admin-crear-contratista).
begin;

-- 1. normalizar_cedula da lo mismo que el núcleo (tests/vectores_cedula.tsv).
do $$
declare
  caso record;
begin
  for caso in
    select * from (values
      ('112340567', '112340567'),
      ('1-1234-0567', '112340567'),
      ('01-1234-0567', '112340567'),
      ('0112340567', '112340567'),
      (' 1 1234 0567 ', '112340567'),
      ('1.1234.0567', '112340567'),
      ('155812345678', '155812345678'),
      ('1558-1234-5678', '155812345678'),
      ('15581234567', '15581234567'),
      ('a1234567', 'A1234567'),
      ('AB 123-456', 'AB123456'),
      ('0A12345678', '0A12345678'),
      ('01234567890', '01234567890'),
      ('', null),
      ('  - . ', null),
      ('1234/5678', null),
      ('12345ñ', null),
      ('111111111111111111111', null)
    ) as v(entrada, esperada)
  loop
    if public.normalizar_cedula(caso.entrada) is distinct from caso.esperada then
      raise exception 'normalizar_cedula(%) dio %, se esperaba %',
        caso.entrada, public.normalizar_cedula(caso.entrada), caso.esperada;
    end if;
  end loop;
end $$;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a002', 'Unidad de prueba contratistas');
insert into public.empresas (id, dispositivo_origen_id, nombre, activa)
values ('eeeeeeee-0000-0000-0000-00000000e001', null, 'EMPRESA DE PRUEBA', true);

-- 2. Quien no es administrador del panel no puede crear empresas.
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  begin
    perform public.panel_crear_empresa('Empresa nueva');
    raise exception 'Un usuario sin permiso pudo crear una empresa';
  exception when insufficient_privilege then null;
  end;
end $$;

-- 3. Contratistas: desde 20261004170000 el alta del panel la hace la Edge
--    Function `admin-crear-contratista` (reglas del núcleo, vía WebAssembly;
--    sus tests están en supabase/functions/admin-crear-contratista/). Acá se
--    prueba lo que sigue siendo de la base.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
declare
  empresa public.empresas;
begin
  -- 3a. La función SQL con las reglas duplicadas ya no existe.
  if to_regprocedure('public.panel_crear_contratista(text, text, uuid, text, date, boolean)') is not null then
    raise exception 'panel_crear_contratista debía estar retirada';
  end if;

  -- 3b. Ni un administrador del panel puede insertar directo (sin pasar por
  --     la Edge Function y sus reglas): no tiene política de INSERT.
  begin
    insert into public.contratistas (id, dispositivo_origen_id, nombre, identificacion, activo,
                                     empresa_id, empresa_nombre, tipo_ingreso)
    values (gen_random_uuid(), null, 'ANA', '112340567', true,
            'eeeeeeee-0000-0000-0000-00000000e001', 'EMPRESA DE PRUEBA', 'POR_CORREO');
    raise exception 'Un administrador pudo insertar un contratista sin la Edge Function';
  exception when insufficient_privilege then null;
  end;

  -- 3c. Empresa: en mayúsculas y sin duplicar aunque cambien tildes o mayúsculas.
  empresa := public.panel_crear_empresa('  nueva   compañía ');
  if empresa.nombre <> 'NUEVA COMPAÑÍA' then raise exception 'nombre de empresa: %', empresa.nombre; end if;
  if (public.panel_crear_empresa('NUEVA COMPANIA')).id <> empresa.id then
    raise exception 'la misma empresa se duplicó';
  end if;
end $$;

-- 4. Lo que hace la Edge Function con la cuenta de servicio (que no pasa por
--    RLS): la cédula repetida, escrita de otra forma, la frena el índice
--    único (23505, que la función responde como 409), y el alta avisa por el
--    canal de TODAS las unidades (el catálogo es global).
reset role;
insert into public.contratistas (id, dispositivo_origen_id, nombre, identificacion, activo,
                                 empresa_id, empresa_nombre, tipo_ingreso)
values (gen_random_uuid(), null, 'JOSÉ ÑANDÚ', '112340567', false,
        'eeeeeeee-0000-0000-0000-00000000e001', 'EMPRESA DE PRUEBA', 'PRAIND');
do $$
begin
  begin
    insert into public.contratistas (id, dispositivo_origen_id, nombre, identificacion, activo,
                                     empresa_id, empresa_nombre, tipo_ingreso)
    values (gen_random_uuid(), null, 'OTRO', '01-1234-0567', true,
            'eeeeeeee-0000-0000-0000-00000000e001', 'EMPRESA DE PRUEBA', 'SWAT');
    raise exception 'Se creó dos veces la misma cédula';
  exception when unique_violation then null;
  end;

  if (
    select count(distinct topic) from realtime.messages
    where event = 'cambio_nube'
      and payload ->> 'table' = 'contratistas'
      and payload ->> 'id' = (select id::text from public.contratistas where identificacion = '112340567')
  ) <> (select count(*) from public.sitios) then
    raise exception 'El alta de un contratista no avisó a todas las unidades';
  end if;
end $$;

-- 5. Un usuario anónimo no puede ni crear empresas.
set local role anon;
do $$
begin
  begin
    perform public.panel_crear_empresa('Empresa anónima');
    raise exception 'anon pudo llamar panel_crear_empresa';
  exception when insufficient_privilege then null;
  end;
end $$;

rollback;
