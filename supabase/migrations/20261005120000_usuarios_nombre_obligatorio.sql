-- El panel web edita el nombre y el rol de los usuarios con un UPDATE directo
-- (la RLS "actualizar usuarios (solo admin_global)" sólo se lo permite a un
-- administrador del panel). El rol ya lo restringe `usuarios_rol_check`; esto
-- agrega la otra regla del núcleo (`UsuarioService`: el nombre es
-- obligatorio) en la base, para que no dependa sólo de la pantalla.
--
-- `not valid`: rige para toda escritura nueva sin revisar las filas que ya
-- existen, así la migración no falla en una base con algún nombre viejo
-- vacío (en staging no hay ninguno).

alter table public.usuarios
  add constraint usuarios_nombre_obligatorio check (pg_catalog.btrim(nombre) <> '') not valid;
