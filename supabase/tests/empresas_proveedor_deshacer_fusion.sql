-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `20261031130000_empresas_proveedor_deshacer_fusion`:
-- - sólo un administrador del panel deshace una fusión;
-- - la fusión anota en cada ingreso movido de qué empresa venía, también en
--   los que llegan después con la empresa fusionada (equipo atrasado);
-- - deshacer la vuelve a activar, la deja por revisar y le devuelve sus
--   ingresos, sin tocar los de la que quedó;
-- - un ingreso registrado antes de la anotación se queda en la que quedó;
-- - después se puede fusionar en el otro sentido (fusión al revés corregida);
-- - deshacer una que no está fusionada falla; mover la empresa de un ingreso
--   a mano sigue prohibido, también tocando la anotación.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a401', 'Unidad deshacer');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c401', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'mobile', 'Diagnóstico deshacer', '{"kty":"EC"}', 'diag-huella-deshacer');

-- e401: la correcta (CAYCHA DEL NORTE); e402: la mal escrita (CAYCHA).
insert into public.empresas_proveedor (id, sitio_id, dispositivo_origen_id, nombre)
values ('eeeeeeee-0000-0000-0000-00000000e401', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'cccccccc-0000-0000-0000-00000000c401', 'DIAG CAYCHA DEL NORTE'),
       ('eeeeeeee-0000-0000-0000-00000000e402', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'cccccccc-0000-0000-0000-00000000c401', 'DIAG CAYCHA');

-- b401 con la correcta, b402 con la mal escrita; los dos cerrados.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_id,
                                       empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre,
                                       hora_salida, usuario_salida_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b401', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'cccccccc-0000-0000-0000-00000000c401', '900000401', 'PROVEEDOR UNO',
        'eeeeeeee-0000-0000-0000-00000000e401', 'DIAG CAYCHA DEL NORTE', 401,
        '2026-10-20 15:00+00', 'OPERADOR', '2026-10-20 16:00+00', 'OPERADOR'),
       ('bbbbbbbb-0000-0000-0000-00000000b402', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'cccccccc-0000-0000-0000-00000000c401', '900000402', 'PROVEEDOR DOS',
        'eeeeeeee-0000-0000-0000-00000000e402', 'DIAG CAYCHA', 402,
        '2026-10-21 15:00+00', 'OPERADOR', '2026-10-21 16:00+00', 'OPERADOR');

set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);

-- Al revés: la correcta queda fusionada en la mal escrita.
select public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e401',
                                               'eeeeeeee-0000-0000-0000-00000000e402');

reset role;

-- Un equipo atrasado registra otro con la correcta (ya fusionada): pasa a la
-- mal escrita y queda anotado.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_id,
                                       empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b403', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'cccccccc-0000-0000-0000-00000000c401', '900000403', 'PROVEEDOR TRES',
        'eeeeeeee-0000-0000-0000-00000000e401', 'DIAG CAYCHA DEL NORTE', 403,
        '2026-10-31 15:00+00', 'OPERADOR');

-- Un ingreso de antes de la anotación (fusión vieja): simulado borrándola
-- con los triggers apagados.
insert into public.ingresos_proveedor (id, sitio_id, dispositivo_entrada_id, cedula, nombre, empresa_id,
                                       empresa_nombre, gafete_numero, hora_entrada, usuario_entrada_nombre,
                                       hora_salida, usuario_salida_nombre)
values ('bbbbbbbb-0000-0000-0000-00000000b404', 'aaaaaaaa-0000-0000-0000-00000000a401',
        'cccccccc-0000-0000-0000-00000000c401', '900000404', 'PROVEEDOR CUATRO',
        'eeeeeeee-0000-0000-0000-00000000e401', 'DIAG CAYCHA DEL NORTE', 404,
        '2026-10-19 15:00+00', 'OPERADOR', '2026-10-19 16:00+00', 'OPERADOR');
set local session_replication_role = replica;
update public.ingresos_proveedor set empresa_original_id = null
 where id = 'bbbbbbbb-0000-0000-0000-00000000b404';
set local session_replication_role = origin;

do $$
declare
  v_ingreso record;
begin
  for v_ingreso in
    select * from public.ingresos_proveedor
     where id in ('bbbbbbbb-0000-0000-0000-00000000b401', 'bbbbbbbb-0000-0000-0000-00000000b403')
  loop
    if v_ingreso.empresa_id is distinct from 'eeeeeeee-0000-0000-0000-00000000e402'::uuid
       or v_ingreso.empresa_original_id is distinct from 'eeeeeeee-0000-0000-0000-00000000e401'::uuid then
      raise exception 'La fusión no anotó de dónde venía el ingreso: %', row_to_json(v_ingreso);
    end if;
  end loop;
  if (select empresa_original_id from public.ingresos_proveedor
       where id = 'bbbbbbbb-0000-0000-0000-00000000b402') is not null then
    raise exception 'Anotó un ingreso que ya era de la empresa que queda';
  end if;

  -- Volver a subir el del equipo atrasado no cambia nada.
  update public.ingresos_proveedor
     set empresa_id = 'eeeeeeee-0000-0000-0000-00000000e401', empresa_nombre = 'DIAG CAYCHA DEL NORTE'
   where id = 'bbbbbbbb-0000-0000-0000-00000000b403';

  -- A mano no se mueve: ni la empresa ni la anotación.
  begin
    update public.ingresos_proveedor set empresa_original_id = 'eeeeeeee-0000-0000-0000-00000000e401'
     where id = 'bbbbbbbb-0000-0000-0000-00000000b402';
    raise exception 'Dejó cambiar la anotación de un ingreso a mano';
  exception when raise_exception then
    if sqlerrm not like 'Los datos de entrada%' then
      raise;
    end if;
  end;
end $$;

set local role authenticated;

-- Quien no es administrador del panel no deshace.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text, true);
do $$
begin
  perform public.panel_deshacer_fusion_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e401');
  raise exception 'Un usuario que no es administrador pudo deshacer una fusión';
exception when insufficient_privilege then null;
end $$;

select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text, true);
do $$
begin
  begin
    perform public.panel_deshacer_fusion_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e402');
    raise exception 'Deshizo la fusión de una empresa que no estaba fusionada';
  exception when invalid_parameter_value then null;
  end;

  perform public.panel_deshacer_fusion_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e401');
end $$;

reset role;

do $$
declare
  v_empresa record;
begin
  select * into v_empresa from public.empresas_proveedor where id = 'eeeeeeee-0000-0000-0000-00000000e401';
  if v_empresa.fusionada_en is not null or not v_empresa.activa or v_empresa.revisada_en is not null then
    raise exception 'Deshacer no la dejó vigente y por revisar: %', row_to_json(v_empresa);
  end if;
  if (select count(*) from public.ingresos_proveedor
       where id in ('bbbbbbbb-0000-0000-0000-00000000b401', 'bbbbbbbb-0000-0000-0000-00000000b403')
         and empresa_id = 'eeeeeeee-0000-0000-0000-00000000e401'
         and empresa_nombre = 'DIAG CAYCHA DEL NORTE'
         and empresa_original_id is null) <> 2 then
    raise exception 'Deshacer no le devolvió sus ingresos';
  end if;
  if (select empresa_id from public.ingresos_proveedor where id = 'bbbbbbbb-0000-0000-0000-00000000b402')
     is distinct from 'eeeeeeee-0000-0000-0000-00000000e402'::uuid then
    raise exception 'Deshacer se llevó un ingreso de la que quedó';
  end if;
  if (select empresa_id from public.ingresos_proveedor where id = 'bbbbbbbb-0000-0000-0000-00000000b404')
     is distinct from 'eeeeeeee-0000-0000-0000-00000000e402'::uuid then
    raise exception 'El ingreso sin anotación debería quedarse en la que quedó';
  end if;
end $$;

-- Ahora en el sentido correcto: todos pasan a la correcta.
set local role authenticated;
select public.panel_fusionar_empresa_proveedor('eeeeeeee-0000-0000-0000-00000000e402',
                                               'eeeeeeee-0000-0000-0000-00000000e401');
reset role;

do $$
begin
  if (select count(*) from public.ingresos_proveedor
       where id::text like 'bbbbbbbb-0000-0000-0000-00000000b40%'
         and empresa_id = 'eeeeeeee-0000-0000-0000-00000000e401') <> 4 then
    raise exception 'La fusión corregida no juntó los 4 ingresos en la correcta';
  end if;
  if (select fusionada_en from public.empresas_proveedor where id = 'eeeeeeee-0000-0000-0000-00000000e402')
     is distinct from 'eeeeeeee-0000-0000-0000-00000000e401'::uuid then
    raise exception 'La mal escrita no quedó fusionada en la correcta';
  end if;
end $$;

rollback;
