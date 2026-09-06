-- El control de acceso sigue limitado al sitio del JWT y a Broadcast.
-- La fila de prueba interna no lleva private=true.
alter policy "dispositivos reciben broadcast de su sitio"
on realtime.messages
to authenticated
using (
  realtime.messages.extension = 'broadcast'
  and (select realtime.topic()) = ('sitio:' || ((select auth.jwt()) ->> 'sitio_id'))
);
