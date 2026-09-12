-- Hallazgo P1 de docs/reporte-seguridad-web-2026-09-09.md: `web/src/App.tsx`
-- retiró la pantalla de administradores explícitamente "para que un
-- compromiso del panel no pueda agregar administradores", pero las
-- políticas INSERT/DELETE seguían vivas en la base -- ocultar un botón no
-- restringe la API, cualquier sesión de admin (robada o no) podía seguir
-- agregando/quitando administradores por PostgREST directo. La migración
-- original (`crea_administradores_panel`) ya documentaba la intención
-- correcta: "Alta/baja de admins queda fuera de esta migración a
-- propósito... se gestiona con service_role (dashboard/SQL directo) hasta
-- que exista una pantalla para eso" -- esto sólo restaura esa intención.
drop policy "admin_global agrega admins" on public.administradores_panel;
drop policy "admin_global borra otros admins" on public.administradores_panel;
