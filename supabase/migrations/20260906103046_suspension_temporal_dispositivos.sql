-- Suspensión temporal de dispositivos, distinta de revocar (baja
-- permanente). `suspended_at` bloquea el login en device-auth sin perder
-- el registro ni el secreto -- útil mientras se repara un equipo o se
-- investiga un secreto sospechoso, sin tener que re-provisionar.
--
-- `last_seen_at` responde "¿hace cuánto no se usa este dispositivo?" sin
-- necesidad de heartbeat: se actualiza en cada login exitoso contra
-- device-auth, que ya ocurre.

alter table public.dispositivos add column suspended_at timestamptz;
alter table public.dispositivos add column last_seen_at timestamptz;
