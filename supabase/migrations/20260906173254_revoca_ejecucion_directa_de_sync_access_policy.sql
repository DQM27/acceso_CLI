-- Advisor de seguridad: anon Y authenticated podían ejecutar
-- sync_access_policy() directo vía /rest/v1/rpc/sync_access_policy -- a
-- diferencia de es_admin_global() (ver revoca_ejecucion_anonima_de_es_
-- admin_global), acá NINGÚN rol de cliente necesita invocarla directo: es
-- `returns trigger`, sólo tiene sentido disparada por
-- trg_sync_access_policy (after insert/update/delete en
-- administradores_panel). Postgres no exige el privilegio EXECUTE de
-- quien dispara un trigger para que éste corra -- revocarlo acá no rompe
-- el trigger, sólo cierra la puerta de invocarla a mano vía RPC.
revoke execute on function public.sync_access_policy() from anon;
revoke execute on function public.sync_access_policy() from authenticated;
