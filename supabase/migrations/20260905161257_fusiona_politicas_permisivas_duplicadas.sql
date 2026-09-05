-- Advisor "Multiple Permissive Policies": estas tablas tenian dos politicas
-- permisivas para el mismo rol/accion, que Postgres combina con OR. Fusionar
-- en una sola no cambia ningun permiso (A or B via dos politicas = A or B
-- en una) -- solo evita evaluar dos politicas por fila. Verificado con la
-- suite completa de supabase/tests/ y una prueba de Realtime end-to-end
-- despues de aplicar esto.

-- contratistas/usuarios: la politica "(global)" ya usa qual/with_check
-- true, que ya cubre a es_admin_global() por completo -- la version
-- admin_global-only queda redundante, se borra sin alterar nada.
drop policy "admin_global lee contratistas" on public.contratistas;
drop policy "admin_global actualiza contratistas" on public.contratistas;
drop policy "admin_global lee usuarios" on public.usuarios;
drop policy "admin_global gestiona usuarios" on public.usuarios;

-- ingresos: acá SÍ hace falta fusionar la condición, "por sitio" y
-- "admin_global" no se subsumen entre sí.
alter policy "leer ingresos del propio sitio" on public.ingresos
using (
  sitio_id = (((select auth.jwt()) ->> 'sitio_id'))::uuid
  or public.es_admin_global()
);
drop policy "admin_global lee todo el historial" on public.ingresos;

-- administradores_panel: mismo caso, fusionar "propia fila" con "admin_global".
alter policy "cada admin lee su propia fila" on public.administradores_panel
using ((select auth.email()) = correo or public.es_admin_global());
drop policy "admin_global ve todos los admins" on public.administradores_panel;
