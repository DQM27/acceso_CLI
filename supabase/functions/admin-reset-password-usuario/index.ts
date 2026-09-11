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

// Ver admin-list-devices/index.ts para el mismo patrón comentado en detalle.
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

// Mismo alfabeto/longitud que admin-create-usuario -- ver ese archivo para
// el razonamiento de entropía/legibilidad.
const ALFABETO_PASSWORD_TEMPORAL = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

function generarPasswordTemporal(longitud = 10): string {
  const bytes = new Uint8Array(longitud);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => ALFABETO_PASSWORD_TEMPORAL[b % ALFABETO_PASSWORD_TEMPORAL.length]).join("");
}

/**
 * Genera una contraseña temporal nueva para un usuario ya existente --
 * "olvidó la contraseña" o backfill de una cédula creada antes de este
 * cambio (auth_user_id todavía NULL). Mismo actor privilegiado que
 * admin-create-usuario, ver docs/plan-autenticacion-supabase-auth.md.
 */
Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response(null, { headers: CORS_HEADERS });
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  let body: { usuario_id?: string };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  const usuarioId = body.usuario_id?.trim();
  if (!usuarioId) return json({ error: "bad_request" }, 400);

  const { data: usuario, error: usuarioError } = await supabase
    .from("usuarios")
    .select("id, cedula, auth_user_id")
    .eq("id", usuarioId)
    .maybeSingle();

  if (usuarioError || !usuario) {
    return json({ error: "usuario_no_encontrado" }, 404);
  }

  const passwordTemporal = generarPasswordTemporal();

  if (usuario.auth_user_id) {
    // Caso normal: ya tenía cuenta de Auth, se le pisa la contraseña.
    const { error: updateError } = await supabase.auth.admin.updateUserById(usuario.auth_user_id, {
      password: passwordTemporal,
      user_metadata: { cedula: usuario.cedula, debe_cambiar_password: true },
    });
    if (updateError) return json({ error: "auth_error", detail: updateError.message }, 500);
  } else {
    // Backfill: usuario creado antes de este cambio, todavía sin cuenta de
    // Auth -- se crea ahora y se enlaza.
    const { data: authUser, error: createError } = await supabase.auth.admin.createUser({
      email: `${usuario.cedula}@brisas.local`,
      password: passwordTemporal,
      email_confirm: true,
      user_metadata: { cedula: usuario.cedula, debe_cambiar_password: true },
    });
    if (createError || !authUser?.user) {
      return json({ error: "auth_error", detail: createError?.message }, 500);
    }
    const { error: linkError } = await supabase
      .from("usuarios")
      .update({ auth_user_id: authUser.user.id })
      .eq("id", usuarioId);
    if (linkError) {
      await supabase.auth.admin.deleteUser(authUser.user.id);
      return json({ error: "usuario_error", detail: linkError.message }, 500);
    }
  }

  return json({ usuario_id: usuarioId, password_temporal: passwordTemporal });
});
