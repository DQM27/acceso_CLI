-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
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
insert into public.empresas (id, sitio_id, dispositivo_origen_id, nombre, activa)
values ('eeeeeeee-0000-0000-0000-00000000e001', 'aaaaaaaa-0000-0000-0000-00000000a002', null, 'EMPRESA DE PRUEBA', true);

-- 2. Quien no es administrador del panel no puede crear nada.
set local role authenticated;
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  begin
    perform public.panel_crear_contratista('112340567', 'Ana', 'eeeeeeee-0000-0000-0000-00000000e001', 'SWAT');
    raise exception 'Un usuario sin permiso pudo crear un contratista';
  exception when insufficient_privilege then null;
  end;
  begin
    perform public.panel_crear_empresa('Empresa nueva');
    raise exception 'Un usuario sin permiso pudo crear una empresa';
  exception when insufficient_privilege then null;
  end;
end $$;

-- 3. Un administrador del panel sí, con las reglas del núcleo.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);
do $$
declare
  fila public.contratistas;
  empresa public.empresas;
  mensaje text;
begin
  -- Bloqueado al crear: cédula en forma única, nombre en mayúsculas, sin PRAIND.
  fila := public.panel_crear_contratista(
    '1-1234-0567', '  josé  ñandú ', 'eeeeeeee-0000-0000-0000-00000000e001', 'PRAIND',
    null, false);
  if fila.identificacion <> '112340567' then raise exception 'cédula guardada: %', fila.identificacion; end if;
  if fila.nombre <> 'JOSÉ ÑANDÚ' then raise exception 'nombre guardado: %', fila.nombre; end if;
  if fila.activo then raise exception 'debía quedar con el acceso denegado'; end if;
  if fila.es_personal_ruta is distinct from false then raise exception 'personal de ruta debía ser false'; end if;
  if fila.dispositivo_origen_id is not null then raise exception 'el panel no tiene equipo de origen'; end if;
  if fila.sitio_id is not null then raise exception 'un contratista es global, no de una unidad'; end if;
  if fila.empresa_nombre <> 'EMPRESA DE PRUEBA' then raise exception 'empresa_nombre: %', fila.empresa_nombre; end if;

  -- El mismo documento escrito de otra forma es la misma persona.
  begin
    perform public.panel_crear_contratista('01-1234-0567', 'Otro', 'eeeeeeee-0000-0000-0000-00000000e001', 'SWAT');
    raise exception 'Se creó dos veces la misma cédula';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'La cédula del contratista ya existe' then raise; end if;
  end;

  -- Con acceso: PRAIND obligatorio y vigente para PRAIND e IN HOUSE.
  begin
    perform public.panel_crear_contratista('223450678', 'Luis', 'eeeeeeee-0000-0000-0000-00000000e001', 'PRAIND');
    raise exception 'Se aceptó PRAIND sin fecha';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'La fecha de PRAIND es obligatoria' then raise; end if;
  end;
  begin
    perform public.panel_crear_contratista('223450678', 'Luis', 'eeeeeeee-0000-0000-0000-00000000e001', 'IN_HOUSE', date '2020-01-01');
    raise exception 'Se aceptó un PRAIND vencido';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'El PRAIND está vencido' then raise; end if;
  end;
  -- POR CORREO y SWAT no piden PRAIND.
  fila := public.panel_crear_contratista('223450678', 'Luis', 'eeeeeeee-0000-0000-0000-00000000e001', 'SWAT');
  if not fila.activo then raise exception 'por defecto debía quedar con acceso'; end if;

  -- Cédula, nombre, empresa y tipo inválidos.
  begin
    perform public.panel_crear_contratista('12345678', 'Ana', 'eeeeeeee-0000-0000-0000-00000000e001', 'SWAT');
    raise exception 'Se aceptó una cédula de 8 dígitos';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'La cédula debe tener sólo números, entre 9 y 13 dígitos' then raise; end if;
  end;
  begin
    perform public.panel_crear_contratista('A12345678', 'Ana', 'eeeeeeee-0000-0000-0000-00000000e001', 'SWAT');
    raise exception 'Se aceptó un pasaporte como contratista';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'La cédula debe tener sólo números, entre 9 y 13 dígitos' then raise; end if;
  end;
  begin
    perform public.panel_crear_contratista('334560789', 'Ana 2', 'eeeeeeee-0000-0000-0000-00000000e001', 'SWAT');
    raise exception 'Se aceptó un nombre con números';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'El nombre no puede tener números ni símbolos' then raise; end if;
  end;
  begin
    perform public.panel_crear_contratista('334560789', 'Ana', gen_random_uuid(), 'SWAT');
    raise exception 'Se aceptó una empresa inexistente';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'Empresa no encontrada' then raise; end if;
  end;
  begin
    perform public.panel_crear_contratista('334560789', 'Ana', 'eeeeeeee-0000-0000-0000-00000000e001', 'OTRO');
    raise exception 'Se aceptó un tipo de ingreso inválido';
  exception when raise_exception then
    get stacked diagnostics mensaje = message_text;
    if mensaje <> 'El tipo de ingreso no es válido' then raise; end if;
  end;

  -- Empresa: en mayúsculas y sin duplicar aunque cambien tildes o mayúsculas.
  empresa := public.panel_crear_empresa('  nueva   compañía ');
  if empresa.nombre <> 'NUEVA COMPAÑÍA' then raise exception 'nombre de empresa: %', empresa.nombre; end if;
  if empresa.sitio_id is not null then raise exception 'una empresa es global, no de una unidad'; end if;
  if (public.panel_crear_empresa('NUEVA COMPANIA')).id <> empresa.id then
    raise exception 'la misma empresa se duplicó';
  end if;
end $$;

-- 4. El catálogo es global: el aviso en vivo del alta sale por el canal de
--    TODAS las unidades, no sólo el de una.
reset role;
do $$
begin
  if (
    select count(distinct topic) from realtime.messages
    where event = 'cambio_nube'
      and payload ->> 'table' = 'contratistas'
      and payload ->> 'id' = (select id::text from public.contratistas where identificacion = '112340567')
  ) <> (select count(*) from public.sitios) then
    raise exception 'El alta de un contratista no avisó a todas las unidades';
  end if;
end $$;

-- 5. Un usuario anónimo no puede ni llamarlas.
set local role anon;
do $$
begin
  begin
    perform public.panel_crear_contratista('112340567', 'Ana', gen_random_uuid(), 'SWAT');
    raise exception 'anon pudo llamar panel_crear_contratista';
  exception when insufficient_privilege then null;
  end;
end $$;

rollback;
