-- Permite "ocultar" un dispositivo de la lista del panel web sin borrarlo ni
-- revocarlo -- para dispositivos de prueba que ya generaron historial real
-- (contratistas/ingresos/usuarios/gafetes con dispositivo_origen_id
-- apuntando a ellos) y por eso Postgres rechaza su borrado definitivo (ver
-- admin-delete-device/index.ts, error 23503). Puramente cosmético: no
-- afecta autenticación, sincronización ni nada del funcionamiento real del
-- dispositivo -- solo si aparece en la grilla del panel.
alter table public.dispositivos
  add column if not exists oculto_en_panel boolean not null default false;
