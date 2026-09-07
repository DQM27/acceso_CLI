// NOTA (2026-09-07, al versionar esto por primera vez): no se encontró
// ningún llamador en web/src -- docs/pendientes.md documenta que "cambiar
// sitio" se sacó de la UI del panel ("no aplica"). Queda desplegada y
// activa en Supabase igual; se versiona acá para no perder el código
// fuente, no porque el panel la use hoy. Si en algún momento se confirma
// que de verdad no hace falta, hay que borrarla también del lado de
// Supabase (no sólo dejar de llamarla).
import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SERVICE_ROLE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;

const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
};

function json(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...CORS_HEADERS },
  });
}

// Mismo patrón que admin-list-devices/index.ts -- identidad real via
// Supabase Auth + administradores_panel, sin clave compartida.
async function correoAdminAutorizado(
  req: Request,
  admin: ReturnType<typeof createClient>,
): Promise<string | null> {
  const token = (req.headers.get("authorization") ?? "").replace(/^Bearer\s+/i, "").trim();
  if (!token) return null;

  const { data: userData, error: userError } = await admin.auth.getUser(token);
  const correo = userData?.user?.email;
  if (userError || !correo) return null;

  const { data: fila } = await admin
    .from("administradores_panel")
    .select("correo")
    .eq("correo", correo)
    .maybeSingle();

  return fila ? correo : null;
}

Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response(null, { headers: CORS_HEADERS });
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  let body: { dispositivo_id?: string; sitio_nombre?: string; sitio_direccion?: string };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  const dispositivoId = body.dispositivo_id?.trim();
  const sitioNombre = body.sitio_nombre?.trim();
  if (!dispositivoId || !sitioNombre) {
    return json({ error: "bad_request", detail: "faltan campos" }, 400);
  }

  const { data: sitio, error: sitioError } = await supabase
    .from("sitios")
    .upsert(
      { nombre: sitioNombre, direccion: body.sitio_direccion?.trim() || null },
      { onConflict: "nombre" },
    )
    .select("id, nombre")
    .single();

  if (sitioError || !sitio) {
    return json({ error: "sitio_error", detail: sitioError?.message }, 500);
  }

  const { error: dispositivoError } = await supabase
    .from("dispositivos")
    .update({ sitio_id: sitio.id })
    .eq("id", dispositivoId);

  if (dispositivoError) {
    return json({ error: "dispositivo_error", detail: dispositivoError.message }, 500);
  }

  return json({ sitio_id: sitio.id, sitio_nombre: sitio.nombre });
});
