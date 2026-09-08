-- Autoriza el canal privado por sitio (ver "dispositivos reciben broadcast
-- de su sitio", migracion 20260905091122) tambien para la extension
-- 'presence' de Realtime -- hasta ahora solo 'broadcast' estaba permitido,
-- asi que ni siquiera los propios dispositivos podian usar presencia
-- todavia. Los dispositivos siguen limitados a su propio sitio (mismo
-- criterio que broadcast); los admins del panel (auth.email() en
-- administradores_panel, ver es_admin_global()) pueden ver la presencia de
-- cualquier sitio -- necesario para que el panel web muestre que
-- dispositivos estan conectados ahora mismo (ver
-- docs/plan-sesion-unica-dispositivos.md, seccion "Panel de presencia en
-- tiempo real").
create policy "dispositivos y admins ven presencia de su sitio"
on realtime.messages
for select
to authenticated
using (
  realtime.messages.extension = 'presence'
  and (
    (select realtime.topic()) = ('sitio:' || ((select auth.jwt()) ->> 'sitio_id'))
    or es_admin_global()
  )
);
