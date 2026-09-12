-- Endurece el webhook que sincroniza administradores_panel -> Cloudflare
-- Access (ver agrega_webhook_sync_access_policy.sql). Dos problemas
-- reales encontrados en la auditoría de seguridad del panel web
-- (2026-09-06):
--
-- 1. El JWT anon quedaba hardcodeado en texto plano en una migración
--    versionada -- queda para siempre en el historial de git aunque se
--    saque de acá. No es un secreto de alto valor (es un anon key, se
--    diseñan para ser públicos), pero sí es frágil: si Supabase revoca
--    ese JWT legado en particular, el trigger queda roto en silencio sin
--    ningún error visible hasta que alguien note que Cloudflare Access
--    dejó de actualizarse.
-- 2. Más importante: la Edge Function `sync-access-policy` sólo dependía
--    de `verify_jwt` de la plataforma para "autorizar" -- eso lo satisface
--    CUALQUIER anon key, que es pública a propósito (está en el bundle
--    del panel web). Cualquiera podía invocar la función directo y recibir
--    de vuelta la lista completa de correos de administradores.
--
-- Fix: el token que ya estaba hardcodeado se mueve a Vault (ver el paso
-- manual en docs/plan-panel-administrativo-web.md -- este archivo NUNCA
-- debe contener el valor real del secreto). Se agrega además un header
-- `x-webhook-secret` con un secreto propio (no un key de Supabase) que la
-- Edge Function exige de verdad para hacer cualquier cosa -- ver
-- supabase/functions/sync-access-policy/index.ts.
create or replace function sync_access_policy()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_apikey text;
  v_webhook_secret text;
begin
  select decrypted_secret into v_apikey
    from vault.decrypted_secrets where name = 'sync_access_policy_apikey';
  select decrypted_secret into v_webhook_secret
    from vault.decrypted_secrets where name = 'sync_access_policy_webhook_secret';

  if v_apikey is null or v_webhook_secret is null then
    raise exception 'Faltan secretos de Vault (sync_access_policy_apikey / sync_access_policy_webhook_secret) -- ver docs/plan-panel-administrativo-web.md';
  end if;

  perform net.http_post(
    url := 'https://xidaepyaljzkpbsxrqsm.supabase.co/functions/v1/sync-access-policy',
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'Authorization', 'Bearer ' || v_apikey,
      'x-webhook-secret', v_webhook_secret
    ),
    body := '{}'::jsonb
  );
  return null;
end;
$$;
