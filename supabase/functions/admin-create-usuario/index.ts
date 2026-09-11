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

// Sin 0/O/1/I/l -- se transcribe a mano una sola vez (WhatsApp, papel), nunca
// se vuelve a mostrar después de esta respuesta. 10 caracteres de un alfabeto
// de 30 símbolos son ~49 bits de entropía, de sobra para un secreto que
// vive minutos hasta el primer login y fuerza cambio inmediato.
const ALFABETO_PASSWORD_TEMPORAL = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

function generarPasswordTemporal(longitud = 10): string {
  const bytes = new Uint8Array(longitud);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => ALFABETO_PASSWORD_TEMPORAL[b % ALFABETO_PASSWORD_TEMPORAL.length]).join("");
}

/** Email sintético interno -- nunca se manda correo real a esto, sólo sirve
 * como identificador de login para Supabase Auth (que exige email/phone).
 * Ver docs/plan-autenticacion-supabase-auth.md. */
function emailSinteticoParaCedula(cedula: string): string {
  return `${cedula.trim()}@brisas.local`;
}

Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") return new Response(null, { headers: CORS_HEADERS });
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);

  if (!(await correoAdminAutorizado(req, supabase))) {
    return json({ error: "unauthorized" }, 401);
  }

  let body: {
    sitio_id?: string;
    cedula?: string;
    nombre?: string;
    rol?: string;
  };
  try {
    body = await req.json();
  } catch {
    return json({ error: "bad_request" }, 400);
  }

  const sitioId = body.sitio_id?.trim();
  const cedula = body.cedula?.trim();
  const nombre = body.nombre?.trim();
  const rol = body.rol?.trim();

  if (!sitioId || !cedula || !nombre || !rol || !["ROOT", "ADMINISTRADOR", "OPERADOR"].includes(rol)) {
    return json({ error: "bad_request", detail: "faltan campos o rol invalido" }, 400);
  }

  const passwordTemporal = generarPasswordTemporal();

  const { data: authUser, error: authError } = await supabase.auth.admin.createUser({
    email: emailSinteticoParaCedula(cedula),
    password: passwordTemporal,
    email_confirm: true,
    user_metadata: { cedula, debe_cambiar_password: true },
  });

  if (authError || !authUser?.user) {
    if (authError?.code === "email_exists") {
      return json({ error: "cedula_ya_existe" }, 409);
    }
    return json({ error: "auth_error", detail: authError?.message }, 500);
  }

  const { data: usuario, error: usuarioError } = await supabase
    .from("usuarios")
    .insert({ sitio_id: sitioId, cedula, nombre, rol, auth_user_id: authUser.user.id })
    .select("id")
    .single();

  if (usuarioError || !usuario) {
    // Sin la fila en `usuarios` este usuario de Auth queda huérfano -- se
    // revierte para no dejar una cuenta que puede loguear pero a la que
    // ningún dispositivo reconoce.
    await supabase.auth.admin.deleteUser(authUser.user.id);
    if (usuarioError?.code === "23505") {
      return json({ error: "cedula_ya_existe" }, 409);
    }
    return json({ error: "usuario_error", detail: usuarioError?.message }, 500);
  }

  return json({
    usuario_id: usuario.id,
    cedula,
    password_temporal: passwordTemporal,
  });
});
