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

// Intenta un borrado definitivo. Las tablas que referencian
// dispositivo_origen_id / dispositivo_entrada_id / dispositivo_salida_id
// (contratistas, empresas, gafetes, usuarios, ingresos) NO tienen ON DELETE
// CASCADE ("NO ACTION") -- si el dispositivo ya genero historial real,
// Postgres rechaza el borrado con foreign_key_violation (23503). En ese
// caso, en vez de fallar, lo marcamos oculto_en_panel=true (ver migracion
// oculta_dispositivos_del_panel): sigue existiendo (con su historial
// intacto) pero deja de aparecer en la lista por defecto -- "al darle
// Eliminar, si no se puede borrar del todo, que al menos se oculte".
Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response(null, { headers: CORS_HEADERS });
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  let body: { dispositivo_id?: string };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  if (!body.dispositivo_id) return json({ error: "bad_request" }, 400);

  const { error: deleteError } = await supabase
    .from("dispositivos")
    .delete()
    .eq("id", body.dispositivo_id);

  if (!deleteError) return json({ ok: true, borrado: true });

  if (deleteError.code !== "23503") {
    return json({ error: "delete_error", detail: deleteError.message }, 500);
  }

  const { error: hideError } = await supabase
    .from("dispositivos")
    .update({ oculto_en_panel: true })
    .eq("id", body.dispositivo_id);

  if (hideError) return json({ error: "hide_error", detail: hideError.message }, 500);

  return json({ ok: true, borrado: false });
});
