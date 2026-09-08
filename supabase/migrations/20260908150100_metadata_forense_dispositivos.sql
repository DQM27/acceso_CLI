-- Metadata del telefono fisico capturada en la activacion inicial (ver
-- docs/plan-sesion-unica-dispositivos.md) + IP observada por el propio
-- servidor en cada autenticacion. Hoy el secreto es estatico y nunca se
-- registra que telefono lo consumio -- estas columnas son el primer paso
-- para que el panel distinga "el mismo telefono de siempre" de uno
-- distinto, y para tener evidencia si hace falta denunciar un intento de
-- fraude. android_id/modelo/fabricante/fingerprint/app_version viajan una
-- sola vez desde el cliente (activacion); last_ip se actualiza en cada
-- device-auth exitoso, la ve el propio edge function, no depende de lo que
-- mande el cliente.
alter table public.dispositivos
  add column if not exists android_id text,
  add column if not exists modelo text,
  add column if not exists fabricante text,
  add column if not exists fingerprint text,
  add column if not exists app_version text,
  add column if not exists last_ip text;
