-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `20261031120000_empresas_proveedor_revision_y_fusion`:
-- - sólo un administrador del panel confirma o fusiona;
-- - la fusión marca la empresa, la desactiva y pasa sus ingresos (empresa y
--   nombre) a la que queda, sin chocar con los datos de entrada inmutables;
-- - un ingreso que llega después con la empresa fusionada (equipo atrasado)
--   se guarda con la que quedó, también si se vuelve a subir;
-- - cambiar la empresa de un ingreso por cualquier otra sigue prohibido;
-- - fusiones inválidas (consigo misma, otra unidad, ya fusionada);
-- - el catálogo de los equipos trae la marca y la vista del panel, los conteos.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a301', 'Unidad fusión A'),
       ('aaaaaaaa-0000-0000-0000-00000000a302', 'Unidad fusión B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c301', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'mobile', 'Diagnóstico fusión', '{"kty":"EC"}', 'diag-huella-fusion');

insert into public.empresas_proveedor (id, sitio_id, dispositivo_origen_id, nombre)
values ('eeeeeeee-0000-0000-0000-00000000e301', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'cccccccc-0000-0000-0000-00000000c301', 'DIAG FOODIE FACTORY'),
       ('eeeeeeee-0000-0000-0000-00000000e302', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'cccccccc-0000-0000-0000-00000000c301', 'DIAG FUDDY FACTORY'),
       ('eeeeeeee-0000-0000-0000-00000000e303', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'cccccccc-0000-0000-0000-00000000c301', 'DIAG OTRA'),
       ('eeeeeeee-0000-0000-0000-00000000e304', 'aaaaaaaa-0000-0000-0000-00000000a302',
        'cccccccc-0000-0000-0000-00000000c301', 'DIAG DE OTRA UNIDAD');

insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_id,
                                       empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre,
                                       hora_salida, usuario_salida_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b301', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'cccccccc-0000-0000-0000-00000000c301', '900000301', 'PROVEEDOR UNO',
        'eeeeeeee-0000-0000-0000-00000000e302', 'DIAG FUDDY FACTORY', 301,
        '2026-10-20 15:00+00', 'OPERADOR', '2026-10-20 16:00+00', 'OPERADOR');

-- Una empresa nueva queda por revisar.
do $$
begin
  if (select revisada_en from public.empresas_proveedor
       where id = 'eeeeeeee-0000-0000-0000-00000000e302') is not null then
    raise exception 'Una empresa recién creada no debería estar revisada';
  end if;
end $$;

-- Mismo nombre (sin mayúsculas ni tildes) en la misma unidad: rechazado.
do $$
begin
  insert into public.empresas_proveedor (id, sitio_id, dispositivo_origen_id, nombre)
  values (gen_random_uuid(), 'aaaaaaaa-0000-0000-0000-00000000a301',
          'cccccccc-0000-0000-0000-00000000c301', 'diag otrá');
  raise exception 'Aceptó dos empresas con el mismo nombre en la misma unidad';
exception when unique_violation then null;
end $$;

set local role authenticated;

-- Quien no es administrador del panel no confirma ni fusiona.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text, true);
do $$
begin
  begin
    perform public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e302',
                                                    'eeeeeeee-0000-0000-0000-00000000e301');
    raise exception 'Un usuario que no es administrador pudo fusionar';
  exception when insufficient_privilege then null;
  end;
  begin
    perform public.panel_confirmar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e303');
    raise exception 'Un usuario que no es administrador pudo confirmar';
  exception when insufficient_privilege then null;
  end;
end $$;

select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);

do $$
begin
  -- Fusiones inválidas.
  begin
    perform public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e301',
                                                    'eeeeeeee-0000-0000-0000-00000000e301');
    raise exception 'Fusionó una empresa consigo misma';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e304',
                                                    'eeeeeeee-0000-0000-0000-00000000e301');
    raise exception 'Fusionó empresas de unidades distintas';
  exception when invalid_parameter_value then null;
  end;

  -- La fusión: FUDDY era FOODIE.
  perform public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e302',
                                                  'eeeeeeee-0000-0000-0000-00000000e301');
end $$;

reset role;

do $$
declare
  v_empresa record;
  v_ingreso record;
begin
  select * into v_empresa from public.empresas_proveedor where id = 'eeeeeeee-0000-0000-0000-00000000e302';
  if v_empresa.fusionada_en is distinct from 'eeeeeeee-0000-0000-0000-00000000e301'::uuid
     or v_empresa.activa or v_empresa.revisada_en is null
     or v_empresa.revisada_por is distinct from current_setting('diagnostico.correo_admin') then
    raise exception 'La empresa fusionada quedó mal: %', row_to_json(v_empresa);
  end if;
  if (select revisada_en from public.empresas_proveedor
       where id = 'eeeeeeee-0000-0000-0000-00000000e301') is null then
    raise exception 'La empresa que queda debería quedar revisada';
  end if;

  select * into v_ingreso from public.ingresos_proveedor where id = 'bbbbbbbb-0000-0000-0000-00000000b301';
  if v_ingreso.empresa_id is distinct from 'eeeeeeee-0000-0000-0000-00000000e301'::uuid
     or v_ingreso.empresa_nombre is distinct from 'DIAG FOODIE FACTORY' then
    raise exception 'El ingreso no pasó a la empresa que queda: %', row_to_json(v_ingreso);
  end if;

  -- Ya fusionada: no se vuelve a fusionar, ni se fusiona otra en ella.
  begin
    perform public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e303',
                                                    'eeeeeeee-0000-0000-0000-00000000e302');
    raise exception 'Fusionó en una empresa ya fusionada';
  exception when insufficient_privilege or invalid_parameter_value then null;
  end;
end $$;

-- Un equipo atrasado registra un ingreso con la empresa fusionada: queda con
-- la que quedó.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_id,
                                       empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b302', 'aaaaaaaa-0000-0000-0000-00000000a301',
        'cccccccc-0000-0000-0000-00000000c301', '900000302', 'PROVEEDOR DOS',
        'eeeeeeee-0000-0000-0000-00000000e302', 'DIAG FUDDY FACTORY', 302,
        '2026-10-31 15:00+00', 'OPERADOR');

do $$
begin
  if (select empresa_id from public.ingresos_proveedor where id = 'bbbbbbbb-0000-0000-0000-00000000b302')
     is distinct from 'eeeeeeee-0000-0000-0000-00000000e301'::uuid then
    raise exception 'Un ingreso nuevo con la empresa fusionada no pasó a la que queda';
  end if;

  -- El mismo equipo lo vuelve a subir (upsert con la empresa vieja): sin
  -- error y sigue con la que quedó.
  update public.ingresos_proveedor
     set empresa_id = 'eeeeeeee-0000-0000-0000-00000000e302', empresa_nombre = 'DIAG FUDDY FACTORY'
   where id = 'bbbbbbbb-0000-0000-0000-00000000b302';
  if (select empresa_nombre from public.ingresos_proveedor where id = 'bbbbbbbb-0000-0000-0000-00000000b302')
     is distinct from 'DIAG FOODIE FACTORY' then
    raise exception 'Volver a subir el ingreso con la empresa fusionada la cambió';
  end if;

  -- Cambiarle la empresa por cualquier otra sigue prohibido.
  begin
    update public.ingresos_proveedor set empresa_id = 'eeeeeeee-0000-0000-0000-00000000e303'
     where id = 'bbbbbbbb-0000-0000-0000-00000000b302';
    raise exception 'Dejó cambiar la empresa de un ingreso sin fusión';
  exception when raise_exception then
    if sqlerrm not like 'Los datos de entrada%' then
      raise;
    end if;
  end;
end $$;

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);

do $$
declare
  v_fila record;
begin
  perform public.panel_confirmar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e303');

  select * into v_fila from public.panel_empresas_proveedor where id = 'eeeeeeee-0000-0000-0000-00000000e301';
  if v_fila.ingresos <> 2 or v_fila.sitio_nombre <> 'Unidad fusión A' then
    raise exception 'La vista del panel no cuenta bien: %', row_to_json(v_fila);
  end if;
  select * into v_fila from public.panel_empresas_proveedor where id = 'eeeeeeee-0000-0000-0000-00000000e302';
  if v_fila.fusionada_en_nombre is distinct from 'DIAG FOODIE FACTORY' or v_fila.ingresos <> 0 then
    raise exception 'La vista del panel no muestra la fusión: %', row_to_json(v_fila);
  end if;
  if (select revisada_en from public.panel_empresas_proveedor
       where id = 'eeeeeeee-0000-0000-0000-00000000e303') is null then
    raise exception 'Confirmar no la marcó revisada';
  end if;
end $$;

-- El catálogo de un equipo de la unidad trae la marca de fusión.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'sub', 'cccccccc-0000-0000-0000-00000000c301',
                    'sitio_id', 'aaaaaaaa-0000-0000-0000-00000000a301', 'tipo', 'mobile')::text, true);
do $$
declare
  v_fila jsonb;
begin
  select f into v_fila
    from jsonb_array_elements(public.catalogo_del_sitio() -> 'empresas_proveedor') f
   where f ->> 'id' = 'eeeeeeee-0000-0000-0000-00000000e302';
  if v_fila ->> 'fusionada_en' is distinct from 'eeeeeeee-0000-0000-0000-00000000e301' then
    raise exception 'El catálogo del equipo no trae la fusión: %', v_fila;
  end if;
end $$;

rollback;
