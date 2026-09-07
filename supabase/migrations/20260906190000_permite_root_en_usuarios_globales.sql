-- Decisión revertida: ROOT ahora SÍ viaja por la nube, junto con
-- ADMINISTRADOR/OPERADOR (ver migración crea_usuarios_globales, que lo
-- excluía a propósito). Motivo real: al armar la pantalla de arranque sin
-- login (pegar el secreto trae el catálogo, ver docs/pendientes.md
-- "Escritorio/móvil sin onboarding por GUI"), el único usuario que bajaba
-- era un ADMINISTRADOR -- un dispositivo nuevo necesita también poder
-- recibir su ROOT real, no sólo administradores/operadores.
--
-- password_hash sigue sin viajar nunca (columna que ni existe acá) --
-- ROOT entra al mismo mecanismo SIN_PASSWORD_LOCAL que ya usan
-- ADMINISTRADOR/OPERADOR: el hash se fija localmente la primera vez que
-- esa cédula inicia sesión en cada dispositivo.
alter table public.usuarios drop constraint usuarios_rol_check;
alter table public.usuarios add constraint usuarios_rol_check
  check (rol in ('ROOT', 'ADMINISTRADOR', 'OPERADOR'));

comment on table public.usuarios is
  'Usuarios globales (ROOT/ADMINISTRADOR/OPERADOR) -- ver crea_usuarios_globales y permite_root_en_usuarios_globales. Sin password_hash: eso se fija por dispositivo, ver SIN_PASSWORD_LOCAL en src/services/password.rs.';
