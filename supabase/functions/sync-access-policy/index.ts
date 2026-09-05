// Mantiene sincronizada la política de Cloudflare Access ("Panel Brisas")
// con `administradores_panel` -- se dispara vía Database Webhook en
// INSERT/DELETE/UPDATE de esa tabla (ver migración
// `agrega_webhook_sync_access_policy`). Ignora el payload del webhook y
// relee la tabla completa en cada llamada: más simple que reconciliar
// deltas, y evita que un evento perdido (Cloudflare caído, retry fallido)
// deje la política desincronizada silenciosamente -- cualquier disparo
// futuro la vuelve a poner en el estado correcto.
//
// Variables de entorno necesarias (`supabase secrets set`):
//   CF_API_TOKEN         token con permiso "Access: Apps and Policies" Edit
//   CF_ACCOUNT_ID        id de cuenta de Cloudflare
//   CF_ACCESS_APP_ID     id de la app de Access ("Panel Brisas")
//   CF_ACCESS_POLICY_ID  id de la política dentro de esa app
import { createClient } from "npm:@supabase/supabase-js@2";

Deno.serve(async () => {
  const supabaseUrl = Deno.env.get("SUPABASE_URL")!;
  const serviceRoleKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
  const cfToken = Deno.env.get("CF_API_TOKEN")!;
  const accountId = Deno.env.get("CF_ACCOUNT_ID")!;
  const appId = Deno.env.get("CF_ACCESS_APP_ID")!;
  const policyId = Deno.env.get("CF_ACCESS_POLICY_ID")!;

  const supabase = createClient(supabaseUrl, serviceRoleKey);
  const { data, error } = await supabase.from("administradores_panel").select("correo");
  if (error) {
    return new Response(JSON.stringify({ error: error.message }), { status: 500 });
  }

  const correos = (data ?? []).map((fila) => fila.correo);
  if (correos.length === 0) {
    // Nunca dejar la política sin nadie -- una tabla vacía (borrado por
    // error, migración a medio correr) no debe trabar a todo el mundo
    // afuera del panel sin nadie que pueda entrar a arreglarlo.
    return new Response(
      JSON.stringify({ error: "administradores_panel está vacía, no se actualizó la política" }),
      { status: 412 },
    );
  }

  const respuesta = await fetch(
    `https://api.cloudflare.com/client/v4/accounts/${accountId}/access/apps/${appId}/policies/${policyId}`,
    {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${cfToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        name: "Administradores del panel",
        decision: "allow",
        include: correos.map((correo) => ({ email: { email: correo } })),
      }),
    },
  );

  const resultado = await respuesta.json();
  if (!respuesta.ok || !resultado.success) {
    return new Response(JSON.stringify({ error: resultado.errors ?? resultado }), { status: 502 });
  }

  return new Response(JSON.stringify({ correos }), {
    headers: { "Content-Type": "application/json" },
  });
});
