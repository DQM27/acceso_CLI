// Mantiene sincronizada la política de Cloudflare Access ("Panel Brisas")
// con `administradores_panel` -- se dispara vía Database Webhook en
// INSERT/DELETE/UPDATE de esa tabla (ver migración
// `agrega_webhook_sync_access_policy`). Ignora el payload del webhook y
// relee la tabla completa en cada llamada: más simple que reconciliar
// deltas, y evita que un evento perdido (Cloudflare caído, retry fallido)
// deje la política desincronizada silenciosamente -- cualquier disparo
// futuro la vuelve a poner en el estado correcto.
//
// Autorización propia con secreto compartido (`x-webhook-secret`), NO el
// `apikey`/`Authorization` de Supabase -- ese es el anon/publishable key,
// PÚBLICO a propósito (está embebido en el bundle del panel web). Antes
// esta función sólo dependía de `verify_jwt` de la plataforma, que
// cualquier anon key (o sea, cualquiera) satisface -- eso significaba que
// cualquier persona podía invocar esta función directo y recibir de
// vuelta la lista completa de correos de admins en el body de la
// respuesta (hallazgo de seguridad, auditoría 2026-09-06). El secreto acá
// es propio, generado a mano, nunca commiteado -- ver
// `docs/planes-implementados/plan-panel-administrativo-web.md` para el paso manual de
// configurarlo (Vault del lado de Postgres + `supabase secrets set` del
// lado de esta función).
//
// Variables de entorno necesarias (`supabase secrets set`):
//   CF_API_TOKEN         token con permiso "Access: Apps and Policies" Edit
//   CF_ACCOUNT_ID        id de cuenta de Cloudflare
//   CF_ACCESS_APP_ID     id de la app de Access ("Panel Brisas")
//   CF_ACCESS_POLICY_ID  id de la política dentro de esa app
//   WEBHOOK_SHARED_SECRET  mismo valor que el secreto de Vault que manda
//                           sync_access_policy() -- ver la migración.
import { createClient } from "npm:@supabase/supabase-js@2";

function tiempoConstanteIguales(a: string, b: string): boolean {
  // Comparación en tiempo constante -- un `===` normal corta apenas
  // encuentra la primera diferencia, lo que en teoría permite medir por
  // temporización cuántos caracteres iniciales acertó un atacante
  // probando el secreto a fuerza bruta. No es la parte más probable de
  // explotar acá, pero comparar secretos es exactamente el caso para el
  // que existe este patrón.
  if (a.length !== b.length) return false;
  let diferencia = 0;
  for (let i = 0; i < a.length; i++) {
    diferencia |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diferencia === 0;
}

Deno.serve(async (req) => {
  const secretoEsperado = Deno.env.get("WEBHOOK_SHARED_SECRET");
  const secretoRecibido = req.headers.get("x-webhook-secret");
  if (!secretoEsperado || !secretoRecibido || !tiempoConstanteIguales(secretoRecibido, secretoEsperado)) {
    return new Response(JSON.stringify({ error: "No autorizado" }), { status: 401 });
  }

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

  // Sólo confirma cuántos correos aplicó -- el llamador real (el trigger
  // de Postgres, vía `perform net.http_post`) descarta la respuesta de
  // todas formas; no hace falta devolver la lista completa de correos de
  // vuelta, ni siquiera a un llamador ya autenticado con el secreto.
  return new Response(JSON.stringify({ ok: true, cantidad: correos.length }), {
    headers: { "Content-Type": "application/json" },
  });
});
