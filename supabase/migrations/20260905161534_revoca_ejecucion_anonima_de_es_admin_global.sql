-- Advisor de seguridad: el rol anon (sin login) podia ejecutar
-- es_admin_global() directo via /rest/v1/rpc/es_admin_global. No filtra
-- datos (solo devuelve true/false), pero no hacia falta exponerlo sin
-- login. Se mantiene el permiso para `authenticated` a proposito: TODAS
-- las politicas RLS de admin_global (contratistas, ingresos, usuarios,
-- sitios, administradores_panel) dependen de que ese rol pueda ejecutar
-- esta funcion -- revocarselo tambien rompe Realtime otra vez. Verificado
-- con la suite de supabase/tests/ y una prueba de Realtime end-to-end.
revoke execute on function public.es_admin_global() from anon;
