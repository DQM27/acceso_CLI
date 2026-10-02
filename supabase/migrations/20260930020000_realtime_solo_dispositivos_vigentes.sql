-- El canal en vivo (`realtime.messages`, tema `sitio:<id>`) autorizaba sólo
-- por `sitio_id` del JWT: un dispositivo retirado con un token
-- todavía vigente (hasta 1 h) podía seguir suscribiéndose y recibiendo los
-- avisos de su sitio, que traen la fila completa (`cambio_nube`).
-- Encontrado en la prueba con equipos reales (staging, 2026-09-30).
--
-- Misma regla que las tablas de `public` (ver
-- `20260929200000_revocacion_efectiva_dispositivos`): una política
-- RESTRICTIVA que se suma con AND a las permisivas existentes. Las sesiones
-- humanas (panel, sin `sitio_id`) no se ven afectadas.
--
-- Realtime evalúa esto al unirse al canal. Para cortar un canal ya abierto,
-- los clientes lo cierran al recibir `dispositivo_expulsado` y al
-- reconectar esta política (y `device-auth`) los rechaza.

drop policy if exists "solo dispositivos vigentes" on realtime.messages;

create policy "solo dispositivos vigentes"
  on realtime.messages
  as restrictive
  for all
  to authenticated
  using ((select private.dispositivo_vigente()))
  with check ((select private.dispositivo_vigente()));
