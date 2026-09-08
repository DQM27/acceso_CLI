import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SERVICE_ROLE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;

const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
};

function json(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...CORS_HEADERS },
  });
}

// Reemplaza la clave compartida (x-admin-key) por identidad real: quien
// llama debe traer un JWT de sesión de Supabase Auth (Google OAuth desde
// el panel web) Y estar en `administradores_panel` -- mismo criterio que
// usa el resto del panel via RLS (es_admin_global()), pero acá se chequea
// a mano porque esta función usa el service role para leer/escribir sin
// pasar por RLS.
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

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  const { data: sitios, error: sitiosError } = await supabase
    .from("sitios")
    .select("id, nombre, direccion, created_at")
    .order("nombre");

  if (sitiosError) return json({ error: "sitios_error", detail: sitiosError.message }, 500);

  const { data: dispositivos, error: dispositivosError } = await supabase
    .from("dispositivos")
    .select(
      "id, sitio_id, tipo, etiqueta, created_at, revoked_at, suspended_at, last_seen_at, oculto_en_panel, " +
        "identificador_hardware, nombre_dispositivo, plataforma, version_build, app_version, last_ip",
    )
    .order("created_at", { ascending: false });

  if (dispositivosError) {
    return json({ error: "dispositivos_error", detail: dispositivosError.message }, 500);
  }

  return json({ sitios, dispositivos });
});
