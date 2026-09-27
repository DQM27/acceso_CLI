-- El aviso en vivo `cambio_nube` lleva los datos, no sólo "cambió la tabla X".
--
-- Antes el aviso decía qué tabla cambió y cada equipo tenía que ir a buscar
-- los datos a la nube (un viaje de ida y vuelta más, que era la demora que
-- se notaba). Ahora:
-- - `id`: el id de la fila, siempre (también en DELETE).
-- - `registro`: la fila completa, por ahora sólo para `ingresos` (la
--   pantalla Activos). El equipo la guarda directo en su SQLite
--   (`nube::en_vivo` en el núcleo Rust).
--
-- Compatible hacia atrás: las apps que no conocen estos campos los ignoran
-- y siguen sincronizando como antes. El canal es privado por sitio (RLS en
-- `realtime.messages`), y la fila es la misma que cada equipo del sitio ya
-- puede leer por REST.

create or replace function private.emitir_cambio_nube_sitio()
 returns trigger
 language plpgsql
 security definer
 set search_path to ''
as $function$
declare
  v_sitio_id uuid;
  v_dispositivo_id text;
  v_fila jsonb;
begin
  if tg_op = 'DELETE' then
    v_sitio_id := old.sitio_id;
    v_fila := pg_catalog.to_jsonb(old);
  else
    v_sitio_id := new.sitio_id;
    v_fila := pg_catalog.to_jsonb(new);
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
      'changed_at', pg_catalog.now(),
      'id', v_fila ->> 'id',
      'registro', case
        when tg_table_name = 'ingresos' and tg_op <> 'DELETE' then v_fila
        else null
      end
    ),
    'cambio_nube',
    'sitio:' || v_sitio_id::text,
    true
  );

  return null;
end;
$function$;
