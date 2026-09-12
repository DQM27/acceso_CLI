-- Complementa "dispositivos y admins ven presencia de su sitio" (migracion
-- 20260908170000): esa policy de `select` alcanza para ESCUCHAR presencia,
-- pero Supabase exige una policy de `insert` aparte para poder PUBLICARLA
-- (`track()`) -- confirmado contra la documentacion oficial de Realtime
-- Authorization tras ver "UnableToHandlePresence: :unauthorized" en los
-- logs del proyecto al probar en un dispositivo real. Sin esta policy,
-- ni el propio dispositivo dueno del sitio podia marcarse presente.
-- Solo dispositivos (no admins -- el panel nunca hace track(), solo
-- escucha), limitados a su propio sitio, mismo criterio que broadcast.
create policy "dispositivos publican su propia presencia"
on realtime.messages
for insert
to authenticated
with check (
  realtime.messages.extension = 'presence'
  and (select realtime.topic()) = ('sitio:' || ((select auth.jwt()) ->> 'sitio_id'))
);
