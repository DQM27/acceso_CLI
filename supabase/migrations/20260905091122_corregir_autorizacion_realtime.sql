-- La prueba interna de autorización omite private (su valor predeterminado
-- es false). La privacidad del canal se exige en el cliente; el acceso
-- sigue restringido al sitio del JWT firmado por device-auth.
alter policy "dispositivos reciben broadcast de su sitio"
on realtime.messages
to authenticated
using (
  realtime.messages.extension = 'broadcast'
  and (select realtime.topic()) = ('sitio:' || ((select auth.jwt()) ->> 'sitio_id'))
);
