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

async function sha256Hex(text: string): Promise<string> {
  const data = new TextEncoder().encode(text);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  return Array.from(new Uint8Array(hashBuffer))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// Reemplaza la clave compartida (x-admin-key) por identidad real -- ver
// admin-list-devices/index.ts para el mismo patrón comentado en detalle.
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

  let body: {
    sitio_nombre?: string;
    tipo?: string;
    etiqueta?: string;
  };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  const sitioNombre = body.sitio_nombre?.trim();
  const tipo = body.tipo?.trim();
  const etiqueta = body.etiqueta?.trim();

  if (!sitioNombre || !tipo || !etiqueta || !["pc", "mobile", "visor"].includes(tipo)) {
    return json({ error: "bad_request", detail: "faltan campos o tipo invalido" }, 400);
  }

  // `direccion` se elimino de `sitios` (2026-09-12) -- ver el mismo
  // comentario en admin-create-site/index.ts.
  const { data: sitio, error: sitioError } = await supabase
    .from("sitios")
    .upsert({ nombre: sitioNombre }, { onConflict: "nombre" })
    .select("id, nombre")
    .single();

  if (sitioError || !sitio) {
    return json({ error: "sitio_error", detail: sitioError?.message }, 500);
  }

  const secret = crypto.randomUUID() + crypto.randomUUID();
  const secretHash = await sha256Hex(secret);

  const { data: dispositivo, error: dispositivoError } = await supabase
    .from("dispositivos")
    .insert({
      sitio_id: sitio.id,
      tipo,
      etiqueta,
      secret_hash: secretHash,
    })
    .select("id")
    .single();

  if (dispositivoError || !dispositivo) {
    return json({ error: "dispositivo_error", detail: dispositivoError?.message }, 500);
  }

  return json({
    sitio_id: sitio.id,
    sitio_nombre: sitio.nombre,
    dispositivo_id: dispositivo.id,
    secret,
  });
});
