-- Dispara `sync-access-policy` (Edge Function) cada vez que cambia
-- `administradores_panel`, para que la política de Cloudflare Access
-- ("Panel Brisas") nunca quede desincronizada de quién puede entrar al
-- panel. Un solo trigger por sentencia (no por fila): la función relee la
-- tabla completa, así que no importa cuántas filas cambiaron.
create extension if not exists pg_net;

create or replace function sync_access_policy()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  perform net.http_post(
    url := 'https://xidaepyaljzkpbsxrqsm.supabase.co/functions/v1/sync-access-policy',
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'Authorization', 'Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InhpZGFlcHlhbGp6a3Bic3hycXNtIiwicm9sZSI6ImFub24iLCJpYXQiOjE3ODgzNTk3ODMsImV4cCI6MjEwMzkzNTc4M30.fSn0js_bztekOv6V2foD5oKvQVzQXs0CcTKCbBl8DrU'
    ),
    body := '{}'::jsonb
  );
  return null;
end;
$$;

comment on function sync_access_policy() is
  'Llama a la Edge Function sync-access-policy para reflejar administradores_panel en la política de Cloudflare Access ("Panel Brisas"). Ver supabase/functions/sync-access-policy/index.ts.';

drop trigger if exists trg_sync_access_policy on administradores_panel;
create trigger trg_sync_access_policy
  after insert or update or delete on administradores_panel
  for each statement
  execute function sync_access_policy();
