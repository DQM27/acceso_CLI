-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
--
-- Cubre la migración reconexion_dispositivos_retirados.
--   1.  Un código de reconexión sin unidad o sin etiqueta no se puede crear;
--       uno de alta no puede traerlas.
--   2.  Un código de reconexión no da de alta (`canjear_codigo_vinculacion`).
--   3.  Con un equipo vigente (no retirado) el código no se consulta ni se
--       canjea, y no queda gastado.
--   4.  Consultar dice a dónde va el equipo y NO gasta el código.
--   5.  Canjear en la misma unidad: reactiva, ata la clave nueva, cambia la
--       etiqueta, muestra el equipo en el panel y deja el evento.
--   6.  El mismo código no sirve dos veces.
--   7.  Canjear para otra unidad: el equipo pasa a esa unidad.
--   8.  Un código vencido no se canjea.
--   9.  Un código de alta no reconecta.
--   10. Clientes (anon/authenticated) no pueden llamar a las funciones.
begin;

insert into public.sitios (id, nombre) values
  (gen_random_uuid(), 'Diagnóstico Reconexión A'),
  (gen_random_uuid(), 'Diagnóstico Reconexión B');
select set_config('diag.sitio_a', (select id::text from public.sitios where nombre = 'Diagnóstico Reconexión A'), true),
       set_config('diag.sitio_b', (select id::text from public.sitios where nombre = 'Diagnóstico Reconexión B'), true);

-- R1 y R2: retirados (con su clave vieja). V: vigente.
insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella, revoked_at, oculto_en_panel) values
  (gen_random_uuid(), current_setting('diag.sitio_a')::uuid, 'pc', 'Diag Retirado R1',
   '{"kty":"EC","crv":"P-256","x":"vieja1","y":"vieja1"}', 'diag-huella-vieja-1', now() - interval '1 day', true),
  (gen_random_uuid(), current_setting('diag.sitio_a')::uuid, 'mobile', 'Diag Retirado R2',
   '{"kty":"EC","crv":"P-256","x":"vieja2","y":"vieja2"}', 'diag-huella-vieja-2', now() - interval '1 day', false),
  (gen_random_uuid(), current_setting('diag.sitio_a')::uuid, 'pc', 'Diag Vigente V',
   '{"kty":"EC","crv":"P-256","x":"v","y":"v"}', 'diag-huella-v', null, false);
select set_config('diag.r1', (select id::text from public.dispositivos where etiqueta = 'Diag Retirado R1'), true),
       set_config('diag.r2', (select id::text from public.dispositivos where etiqueta = 'Diag Retirado R2'), true),
       set_config('diag.v', (select id::text from public.dispositivos where etiqueta = 'Diag Vigente V'), true);

-- 1. Las restricciones de destino.
do $$
begin
  begin
    insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en, proposito)
    values (current_setting('diag.r1')::uuid, 'diag-sin-destino', 'diag@example.com', now() + interval '10 minutes', 'reconexion');
    raise exception '1a: se creó un código de reconexión sin unidad ni etiqueta';
  exception when check_violation then null;
  end;
  begin
    insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en, sitio_destino_id, etiqueta_destino)
    values (current_setting('diag.r1')::uuid, 'diag-alta-con-destino', 'diag@example.com', now() + interval '10 minutes',
            current_setting('diag.sitio_a')::uuid, 'x');
    raise exception '1b: un código de alta trae unidad y etiqueta';
  exception when check_violation then null;
  end;
  begin
    insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en, proposito)
    values (current_setting('diag.r1')::uuid, 'diag-otro-proposito', 'diag@example.com', now() + interval '10 minutes', 'otro');
    raise exception '1c: se aceptó un propósito desconocido';
  exception when check_violation then null;
  end;
end $$;

insert into public.codigos_vinculacion
  (dispositivo_id, codigo_hash, creado_por, expira_en, proposito, sitio_destino_id, etiqueta_destino)
values
  (current_setting('diag.r1')::uuid, 'diag-reco-misma', 'diag@example.com', now() + interval '10 minutes',
   'reconexion', current_setting('diag.sitio_a')::uuid, 'Diag R1 renombrado'),
  (current_setting('diag.r2')::uuid, 'diag-reco-otra', 'diag@example.com', now() + interval '10 minutes',
   'reconexion', current_setting('diag.sitio_b')::uuid, 'Diag R2 en B'),
  (current_setting('diag.v')::uuid, 'diag-reco-vigente', 'diag@example.com', now() + interval '10 minutes',
   'reconexion', current_setting('diag.sitio_a')::uuid, 'Diag V'),
  (current_setting('diag.r2')::uuid, 'diag-reco-vencido', 'diag@example.com', now() - interval '1 minute',
   'reconexion', current_setting('diag.sitio_a')::uuid, 'Diag R2 vencido');
insert into public.codigos_vinculacion (dispositivo_id, codigo_hash, creado_por, expira_en)
values (current_setting('diag.r2')::uuid, 'diag-alta-de-retirado', 'diag@example.com', now() + interval '10 minutes');

set local role service_role;
do $$
declare
  v_filas int;
  v_fila record;
begin
  -- 2. Un código de reconexión no da de alta.
  select count(*) into v_filas from public.canjear_codigo_vinculacion(
    'diag-reco-misma', '{"kty":"EC"}', 'diag-huella-alta', '{}', '127.0.0.1');
  if v_filas <> 0 then
    raise exception '2: un código de reconexión dio de alta';
  end if;

  -- 3. Equipo vigente: ni consulta ni canje, y el código sigue sin usar.
  select count(*) into v_filas from public.consultar_codigo_reconexion('diag-reco-vigente');
  if v_filas <> 0 then
    raise exception '3a: se consultó la reconexión de un equipo vigente';
  end if;
  select count(*) into v_filas from public.canjear_codigo_reconexion(
    'diag-reco-vigente', '{"kty":"EC"}', 'diag-huella-v2', '{}', '127.0.0.1', '{}');
  if v_filas <> 0 then
    raise exception '3b: se reconectó un equipo que no estaba retirado';
  end if;
  if exists (select 1 from public.codigos_vinculacion
              where codigo_hash = 'diag-reco-vigente' and usado_en is not null) then
    raise exception '3c: el código rechazado quedó gastado';
  end if;

  -- 4. Consultar no gasta el código.
  select * into v_fila from public.consultar_codigo_reconexion('diag-reco-misma');
  if v_fila.dispositivo_id is distinct from current_setting('diag.r1')::uuid
     or v_fila.sitio_destino_id is distinct from current_setting('diag.sitio_a')::uuid
     or v_fila.sitio_destino_nombre is distinct from 'Diagnóstico Reconexión A'
     or v_fila.etiqueta_destino is distinct from 'Diag R1 renombrado' then
    raise exception '4a: la consulta no dice a dónde va el equipo: %', row_to_json(v_fila);
  end if;
  if exists (select 1 from public.codigos_vinculacion
              where codigo_hash = 'diag-reco-misma' and usado_en is not null) then
    raise exception '4b: consultar gastó el código';
  end if;

  -- 5. Canje en la misma unidad.
  select * into v_fila from public.canjear_codigo_reconexion(
    'diag-reco-misma', '{"kty":"EC","crv":"P-256","x":"nueva1","y":"nueva1"}', 'diag-huella-nueva-1',
    '{"nombre_dispositivo":"PC Diag"}', '127.0.0.1', '{"pendientes_descartados":0}');
  if v_fila.dispositivo_id is distinct from current_setting('diag.r1')::uuid or v_fila.cambio_de_unidad then
    raise exception '5a: el canje en la misma unidad no devolvió lo esperado: %', row_to_json(v_fila);
  end if;
  if not exists (select 1 from public.dispositivos
                  where id = current_setting('diag.r1')::uuid
                    and revoked_at is null
                    and oculto_en_panel = false
                    and clave_huella = 'diag-huella-nueva-1'
                    and etiqueta = 'Diag R1 renombrado'
                    and sitio_id = current_setting('diag.sitio_a')::uuid
                    and nombre_dispositivo = 'PC Diag') then
    raise exception '5b: el equipo no quedó reactivado con su clave nueva';
  end if;
  if not exists (select 1 from public.eventos_seguridad_dispositivos
                  where dispositivo_id = current_setting('diag.r1')::uuid
                    and tipo = 'reconexion_canjeada'
                    and detalle ->> 'pendientes_descartados' = '0'
                    and detalle ->> 'sitio_nuevo' = current_setting('diag.sitio_a')) then
    raise exception '5c: el canje no dejó su evento';
  end if;

  -- 6. Un solo uso.
  select count(*) into v_filas from public.canjear_codigo_reconexion(
    'diag-reco-misma', '{"kty":"EC"}', 'diag-huella-otra', '{}', '127.0.0.1', '{}');
  if v_filas <> 0 then
    raise exception '6: el mismo código reconectó dos veces';
  end if;

  -- 7. Canje para otra unidad.
  select * into v_fila from public.canjear_codigo_reconexion(
    'diag-reco-otra', '{"kty":"EC","crv":"P-256","x":"nueva2","y":"nueva2"}', 'diag-huella-nueva-2',
    '{}', '127.0.0.1', '{"pendientes_descartados":3,"confirmado_por":"Diag Operador"}');
  if not v_fila.cambio_de_unidad or v_fila.sitio_id is distinct from current_setting('diag.sitio_b')::uuid then
    raise exception '7a: el canje para otra unidad no la cambió: %', row_to_json(v_fila);
  end if;
  if not exists (select 1 from public.eventos_seguridad_dispositivos
                  where dispositivo_id = current_setting('diag.r2')::uuid
                    and tipo = 'reconexion_canjeada'
                    and detalle ->> 'pendientes_descartados' = '3'
                    and detalle ->> 'confirmado_por' = 'Diag Operador'
                    and detalle ->> 'sitio_anterior' = current_setting('diag.sitio_a')) then
    raise exception '7b: el evento no dice qué se descartó ni de qué unidad venía';
  end if;
end $$;

-- 8 y 9 sobre otro equipo retirado (R2 ya quedó vigente en el paso 7).
reset role;
update public.dispositivos set revoked_at = now() where id = current_setting('diag.r2')::uuid;
set local role service_role;
do $$
declare
  v_filas int;
begin
  -- 8. Vencido.
  select count(*) into v_filas from public.canjear_codigo_reconexion(
    'diag-reco-vencido', '{"kty":"EC"}', 'diag-huella-vencido', '{}', '127.0.0.1', '{}');
  if v_filas <> 0 then
    raise exception '8: un código de reconexión vencido se canjeó';
  end if;

  -- 9. Un código de alta no reconecta.
  select count(*) into v_filas from public.canjear_codigo_reconexion(
    'diag-alta-de-retirado', '{"kty":"EC"}', 'diag-huella-alta-2', '{}', '127.0.0.1', '{}');
  if v_filas <> 0 then
    raise exception '9: un código de alta reconectó un equipo retirado';
  end if;
end $$;

-- 10. Sin permiso para los clientes.
reset role;
do $$
declare
  v_rol text;
begin
  foreach v_rol in array array['anon', 'authenticated'] loop
    if has_function_privilege(v_rol, 'public.consultar_codigo_reconexion(text)', 'execute')
       or has_function_privilege(v_rol, 'public.canjear_codigo_reconexion(text, jsonb, text, jsonb, text, jsonb)', 'execute')
       or has_function_privilege(v_rol, 'public.canjear_codigo_vinculacion(text, jsonb, text, jsonb, text)', 'execute') then
      raise exception '10: % puede ejecutar una función de canje o consulta', v_rol;
    end if;
  end loop;
end $$;

select 'reconexion_dispositivos: todo bien' as resultado;
rollback;
