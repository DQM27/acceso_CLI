-- Ejecutar en una sola sesión. Todas las filas de prueba se revierten.
-- Cubre `panel_historial_contratistas` (migración
-- `20261030120000_historial_panel_rendimiento`):
-- - sólo un administrador del panel puede usarla (corre sin RLS);
-- - orden por hora con desempate por id y página por cursor sin repetir ni
--   saltear filas;
-- - búsqueda por cédula, por nombre (pocas personas: por catálogo) y textos
--   con comodines o comillas tratados como literales;
-- - filtros de la grilla (texto, número, fecha, "o" entre condiciones) y
--   rechazo de columnas, operaciones, órdenes y límites fuera de la lista.
begin;

select set_config('diagnostico.correo_admin', 'diagnostico-admin@example.com', true),
       set_config('diagnostico.correo_normal', 'diagnostico-normal@example.com', true);

insert into public.administradores_panel (correo)
values (current_setting('diagnostico.correo_admin'));

insert into public.sitios (id, nombre)
values ('aaaaaaaa-0000-0000-0000-00000000a201', 'Unidad historial A'),
       ('aaaaaaaa-0000-0000-0000-00000000a202', 'Unidad historial B');

insert into public.dispositivos (id, sitio_id, tipo, etiqueta, clave_publica_jwk, clave_huella)
values ('cccccccc-0000-0000-0000-00000000c201', 'aaaaaaaa-0000-0000-0000-00000000a201',
        'pc', 'Diagnóstico historial', '{"kty":"EC"}', 'diag-huella-historial');

insert into public.contratistas (id, dispositivo_origen_id, nombre)
values ('dddddddd-0000-0000-0000-00000000d201', 'cccccccc-0000-0000-0000-00000000c201', 'María Zúñiga Solano'),
       ('dddddddd-0000-0000-0000-00000000d202', 'cccccccc-0000-0000-0000-00000000c201', 'Pedro Araya Mora'),
       ('dddddddd-0000-0000-0000-00000000d203', 'cccccccc-0000-0000-0000-00000000c201', 'Ana Solís');

-- Cinco ingresos cerrados (sin chocar con el índice de persona activa), dos
-- con la misma hora para probar el desempate por id.
insert into public.ingresos (id, sitio_id, dispositivo_entrada_id, contratista_id, contratista_cedula,
                             contratista_nombre, empresa_nombre, gafete_numero, hora_entrada, hora_salida,
                             usuario_entrada_nombre, usuario_salida_nombre)
values
  ('bbbbbbbb-0000-0000-0000-00000000b201', 'aaaaaaaa-0000-0000-0000-00000000a201',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d201', '987650001',
   'María Zúñiga Solano', 'Constructora Él Ñandú', 5,
   '2026-10-20 15:00+00', '2026-10-20 20:00+00', 'Operador Uno', 'Operador Dos'),
  ('bbbbbbbb-0000-0000-0000-00000000b202', 'aaaaaaaa-0000-0000-0000-00000000a201',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d202', '111110002',
   'Pedro Araya Mora', null, 12,
   '2026-10-20 15:00+00', '2026-10-20 21:00+00', 'Operador Uno', null),
  ('bbbbbbbb-0000-0000-0000-00000000b203', 'aaaaaaaa-0000-0000-0000-00000000a202',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d201', '987650001',
   'María Zúñiga Solano', 'Limpiezas 50%', 30,
   '2026-10-10 15:00+00', '2026-10-10 20:00+00', 'Operador Tres', 'Operador Tres'),
  ('bbbbbbbb-0000-0000-0000-00000000b204', 'aaaaaaaa-0000-0000-0000-00000000a202',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d203', '222220003',
   'Ana Solís', 'Constructora Él Ñandú', null,
   '2026-09-01 15:00+00', '2026-09-01 20:00+00', 'Operador Dos', 'Operador Dos'),
  ('bbbbbbbb-0000-0000-0000-00000000b205', 'aaaaaaaa-0000-0000-0000-00000000a201',
   'cccccccc-0000-0000-0000-00000000c201', 'dddddddd-0000-0000-0000-00000000d202', '111110002',
   'Pedro Araya Mora', 'Otra', 7,
   '2026-08-01 15:00+00', '2026-08-01 20:00+00', 'Operador Uno', 'Operador Uno');

set local role authenticated;

-- Alguien autenticado que no es administrador del panel: rechazado.
select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_normal'))::text,
  true);
do $$
begin
  perform * from public.panel_historial_contratistas();
  raise exception 'Un usuario que no es administrador pudo leer el historial';
exception
  when insufficient_privilege then null;
end $$;

select set_config('request.jwt.claims',
  json_build_object('role', 'authenticated', 'email', current_setting('diagnostico.correo_admin'))::text,
  true);

do $$
declare
  v_unidades uuid[] := array['aaaaaaaa-0000-0000-0000-00000000a201',
                              'aaaaaaaa-0000-0000-0000-00000000a202']::uuid[];
  v_ids text[];
begin
  -- Orden por defecto: hora de entrada descendente, desempate por id (mismo sentido).
  select array_agg(right(id::text, 4) order by n) into v_ids
    from (select id, row_number() over () n
            from public.panel_historial_contratistas(p_sitio_ids := v_unidades)) x;
  if v_ids is distinct from array['b202', 'b201', 'b203', 'b204', 'b205'] then
    raise exception 'Orden por defecto: %', v_ids;
  end if;

  -- Cursor: la página siguiente a la fila b201 no repite ni saltea nada.
  select array_agg(right(id::text, 4) order by n) into v_ids
    from (select id, row_number() over () n
            from public.panel_historial_contratistas(
                   p_sitio_ids := v_unidades, p_limite := 2,
                   p_cursor_hora := '2026-10-20 15:00+00',
                   p_cursor_id := 'bbbbbbbb-0000-0000-0000-00000000b201')) x;
  if v_ids is distinct from array['b203', 'b204'] then
    raise exception 'Página por cursor: %', v_ids;
  end if;

  -- Desplazamiento (páginas sin cursor).
  select array_agg(right(id::text, 4) order by n) into v_ids
    from (select id, row_number() over () n
            from public.panel_historial_contratistas(
                   p_sitio_ids := v_unidades, p_limite := 2, p_desplazamiento := 2)) x;
  if v_ids is distinct from array['b203', 'b204'] then
    raise exception 'Página por desplazamiento: %', v_ids;
  end if;

  -- Período y unidad.
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(
           p_desde := '2026-10-01 06:00+00', p_hasta := '2026-10-21 06:00+00',
           p_sitio_ids := array['aaaaaaaa-0000-0000-0000-00000000a202']::uuid[]);
  if v_ids is distinct from array['b203'] then
    raise exception 'Período y unidad: %', v_ids;
  end if;

  -- Cédula por inicio, con guiones y cero a la izquierda.
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_busqueda := '0-9876-5');
  if v_ids is distinct from array['b201', 'b203'] then
    raise exception 'Búsqueda por cédula: %', v_ids;
  end if;

  -- Nombre: todas las palabras, sin tildes ni mayúsculas, en cualquier orden.
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_busqueda := 'ZUNIGA maría');
  if v_ids is distinct from array['b201', 'b203'] then
    raise exception 'Búsqueda por nombre: %', v_ids;
  end if;
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_busqueda := 'mora');
  if v_ids is distinct from array['b202', 'b205'] then
    raise exception 'Búsqueda por apellido: %', v_ids;
  end if;

  -- Nadie se llama así: vacío, sin error.
  if exists (select 1 from public.panel_historial_contratistas(p_busqueda := 'zzqx mora')) then
    raise exception 'Una búsqueda sin personas devolvió filas';
  end if;
  -- Comodines y comillas se buscan literales.
  if exists (select 1 from public.panel_historial_contratistas(p_busqueda := '%')) then
    raise exception 'Un %% suelto no debe coincidir con todo';
  end if;
  if exists (select 1 from public.panel_historial_contratistas(p_busqueda := 'o''brien')) then
    raise exception 'Una comilla en la búsqueda devolvió filas';
  end if;

  -- Filtro de texto: contiene (sin tildes), "o" entre dos condiciones.
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "empresa_p", "union": "and", "condiciones": [{"op": "contiene", "valor": "NANDU el"}]}
    ]');
  if v_ids is distinct from array['b201', 'b204'] then
    raise exception 'Filtro contiene: %', v_ids;
  end if;
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "empresa_p", "union": "or",
       "condiciones": [{"op": "empieza", "valor": "otra"}, {"op": "contiene", "valor": "50%"}]}
    ]');
  if v_ids is distinct from array['b203', 'b205'] then
    raise exception 'Filtro con "o" y %% literal: %', v_ids;
  end if;
  -- Distinto incluye a quien no tiene valor; vacío los encuentra.
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "empresa_p", "condiciones": [{"op": "distinto", "valor": "otra"}]}
    ]');
  if v_ids is distinct from array['b201', 'b202', 'b203', 'b204'] then
    raise exception 'Filtro distinto: %', v_ids;
  end if;
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "usuario_salida_p", "condiciones": [{"op": "vacio"}]}
    ]');
  if v_ids is distinct from array['b202'] then
    raise exception 'Filtro vacío: %', v_ids;
  end if;
  -- Una comilla en el valor no rompe la consulta.
  if exists (select 1 from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "empresa_p", "condiciones": [{"op": "igual", "valor": "x'' or ''1''=''1"}]}
    ]')) then
    raise exception 'Una comilla en el filtro devolvió filas';
  end if;

  -- Número: entre (extremos excluidos) y dos columnas a la vez (AND).
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "gafete_numero", "condiciones": [{"op": "entre", "valor": 5, "valor2": 30}]},
      {"columna": "unidad_p", "condiciones": [{"op": "termina", "valor": "a"}]}
    ]');
  if v_ids is distinct from array['b202', 'b205'] then
    raise exception 'Filtro número y unidad: %', v_ids;
  end if;

  -- Fecha: rango y fuera de rango.
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "hora_entrada", "condiciones": [{"op": "rango",
        "desde": "2026-10-10T06:00:00Z", "hasta": "2026-10-11T06:00:00Z"}]}
    ]');
  if v_ids is distinct from array['b203'] then
    raise exception 'Filtro fecha rango: %', v_ids;
  end if;
  select array_agg(right(id::text, 4) order by id) into v_ids
    from public.panel_historial_contratistas(p_sitio_ids := v_unidades, p_filtros := '[
      {"columna": "hora_entrada", "condiciones": [{"op": "fuera",
        "desde": "2026-08-02T06:00:00Z", "hasta": "2026-10-15T06:00:00Z"}]}
    ]');
  if v_ids is distinct from array['b201', 'b202', 'b205'] then
    raise exception 'Filtro fecha fuera: %', v_ids;
  end if;

  -- Orden por otra columna: vacíos al final, desempate por id.
  select array_agg(right(id::text, 4) order by n) into v_ids
    from (select id, row_number() over () n
            from public.panel_historial_contratistas(
                   p_sitio_ids := v_unidades, p_orden := 'empresa_nombre', p_descendente := false)) x;
  if v_ids is distinct from array['b201', 'b204', 'b203', 'b205', 'b202'] then
    raise exception 'Orden por empresa: %', v_ids;
  end if;

  -- Fuera de la lista cerrada: rechazado.
  begin
    perform * from public.panel_historial_contratistas(p_filtros := '[{"columna": "id", "condiciones": []}]');
    raise exception 'Aceptó filtrar por una columna fuera de la lista';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform * from public.panel_historial_contratistas(p_filtros := '[
      {"columna": "empresa_p", "condiciones": [{"op": "regex", "valor": "x"}]}]');
    raise exception 'Aceptó una operación fuera de la lista';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform * from public.panel_historial_contratistas(p_orden := 'id; drop table public.ingresos');
    raise exception 'Aceptó un orden fuera de la lista';
  exception when invalid_parameter_value then null;
  end;
  begin
    perform * from public.panel_historial_contratistas(p_limite := 5000);
    raise exception 'Aceptó un límite de 5000 filas';
  exception when invalid_parameter_value then null;
  end;
end $$;

rollback;
