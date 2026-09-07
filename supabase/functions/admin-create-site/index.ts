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

// Mismo patron que admin-list-devices/index.ts -- identidad real via
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

// Crea un sitio suelto, sin dispositivo -- para el desplegable de "Sitio" en
// el alta de dispositivos (ver web/src/pantallas/Dispositivos.tsx). Antes de
// esto, el unico camino para crear un sitio era admin-provision-device (que
// de paso crea un dispositivo); esta funcion existe para separar ambas
// cosas.
Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response(null, { headers: CORS_HEADERS });
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  let body: { nombre?: string; direccion?: string };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  const nombre = body.nombre?.trim();
  if (!nombre) return json({ error: "bad_request", detail: "falta el nombre" }, 400);

  const { data: sitio, error } = await supabase
    .from("sitios")
    .upsert({ nombre, direccion: body.direccion?.trim() || null }, { onConflict: "nombre" })
    .select("id, nombre")
    .single();

  if (error || !sitio) return json({ error: "sitio_error", detail: error?.message }, 500);

  return json(sitio);
});
