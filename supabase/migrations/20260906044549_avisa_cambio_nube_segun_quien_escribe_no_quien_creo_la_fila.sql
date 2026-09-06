-- El aviso en vivo (Realtime broadcast) identificaba el origen de un cambio
-- con `dispositivo_origen_id`/`dispositivo_entrada_id` de la FILA -- un
-- campo que se fija una sola vez al crearla y nunca se actualiza en
-- ediciones posteriores (ver contratista_repository.rs::actualizar). Una
-- edición del panel web (o de cualquier otro origen) sobre una fila que un
-- dispositivo creó hace tiempo quedaba marcada con el ID de ESE dispositivo
-- -- y ese mismo dispositivo la descartaba como "eco propio" (mismo
-- `dispositivo_id` que el suyo), sin disparar nunca el resync instantáneo.
-- Reproducido en producción: el escritorio importó el catálogo entero, así
-- que NINGUNA edición futura a esos contratistas le llegaba en vivo, sólo
-- vía el pulso periódico de 2 minutos.
--
-- El JWT de quien ejecuta ESTA escritura (`auth.jwt() ->> 'sub'`, el
-- `sub` que `device-auth` fija al ID del dispositivo autenticado) sí
-- identifica correctamente quién hizo ESTE cambio puntual -- un dispositivo
-- editando localmente sigue viendo su propio ID (mismo comportamiento de
-- antes), pero el panel web (sin JWT de dispositivo) o cualquier otro
-- origen ya no arrastra el ID del creador original.
create or replace function private.emitir_cambio_nube_sitio()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_sitio_id uuid;
  v_dispositivo_id text;
begin
  if tg_op = 'DELETE' then
    v_sitio_id := old.sitio_id;
  else
    v_sitio_id := new.sitio_id;
  end if;

  if v_sitio_id is null then
    return null;
  end if;

  v_dispositivo_id := (select auth.jwt() ->> 'sub');

  perform realtime.send(
    pg_catalog.jsonb_build_object(
      'schema', tg_table_schema,
      'table', tg_table_name,
      'operation', tg_op,
      'sitio_id', v_sitio_id,
      'dispositivo_id', v_dispositivo_id,
      'changed_at', pg_catalog.now()
    ),
    'cambio_nube',
    'sitio:' || v_sitio_id::text,
    true
  );

  return null;
end;
$$;
