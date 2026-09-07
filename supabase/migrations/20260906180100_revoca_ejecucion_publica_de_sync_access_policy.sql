-- El revoke anterior (revoca_ejecucion_directa_de_sync_access_policy) le
-- sacó el permiso a anon/authenticated puntualmente, pero el advisor la
-- seguía marcando: a diferencia de es_admin_global, esta función también
-- tenía EXECUTE otorgado a PUBLIC (proacl mostraba "=X/postgres", el "="
-- sin nombre de rol es PUBLIC) -- eso aplica a cualquier rol sin importar
-- los revokes puntuales. Confirmado con pg_proc.proacl antes de este
-- cambio.
revoke execute on function public.sync_access_policy() from public;
