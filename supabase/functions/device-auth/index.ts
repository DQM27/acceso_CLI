import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";
import { SignJWT, importJWK } from "npm:jose@5";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SERVICE_ROLE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
const SIGNING_KEY_JSON = Deno.env.get("DEVICE_SIGNING_KEY")!;

const TOKEN_TTL_SECONDS = 3600;

// Permite llamadas desde un navegador (la mini web de historial, u otro
// visor futuro) -- las apps nativas (Rust/Kotlin) nunca pasaron por CORS,
// por eso esto no hacia falta antes. El secreto va en el body, no en una
// credencial ambiente (cookie), asi que un origen abierto no expone nada:
// sin el secreto correcto, la respuesta es 401 para cualquiera.
const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, x-client-info, apikey, content-type",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
};

async function sha256Hex(text: string): Promise<string> {
  const data = new TextEncoder().encode(text);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  return Array.from(new Uint8Array(hashBuffer))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

Deno.serve(async (req: Request) => {
  if (req.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: CORS_HEADERS });
  }

  if (req.method !== "POST") {
    return new Response(JSON.stringify({ error: "method_not_allowed" }), {
      status: 405,
      headers: { "Content-Type": "application/json", ...CORS_HEADERS },
    });
  }

  let body: {
    secret?: string;
    metadata?: {
      android_id?: string;
      modelo?: string;
      fabricante?: string;
      fingerprint?: string;
      app_version?: string;
    };
  };
  try {
    body = await req.json();
  } catch {
    return new Response(JSON.stringify({ error: "bad_request" }), {
      status: 400,
      headers: { "Content-Type": "application/json", ...CORS_HEADERS },
    });
  }

  const secret = body.secret;
  if (!secret || typeof secret !== "string") {
    return new Response(JSON.stringify({ error: "bad_request" }), {
      status: 400,
      headers: { "Content-Type": "application/json", ...CORS_HEADERS },
    });
  }

  // La IP la observa el propio servidor -- no depende de lo que mande el
  // cliente (que se podria alterar con un APK modificado), asi que es el
  // dato mas confiable para evidencia forense. Supabase Edge Functions
  // corre detras de su proxy, que agrega este header; no hay acceso directo
  // al socket TCP en Deno Deploy.
  const ip = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ?? null;

  const secretHash = await sha256Hex(secret);

  const supabase = createClient(SUPABASE_URL, SERVICE_ROLE_KEY);
  const { data: dispositivo, error } = await supabase
    .from("dispositivos")
    .select("id, sitio_id, tipo, revoked_at, suspended_at")
    .eq("secret_hash", secretHash)
    .is("revoked_at", null)
    .maybeSingle();

  if (error || !dispositivo) {
    return new Response(JSON.stringify({ error: "invalid_credentials" }), {
      status: 401,
      headers: { "Content-Type": "application/json", ...CORS_HEADERS },
    });
  }

  // Suspension temporal (distinta de revoked_at, que es baja permanente):
  // el dispositivo sigue existiendo con su secreto, pero no puede loguear
  // hasta que un admin lo reactive. Un token ya emitido antes de suspender
  // sigue valido hasta que expire (TOKEN_TTL_SECONDS) -- igual que pasa hoy
  // con revocar.
  if (dispositivo.suspended_at) {
    return new Response(JSON.stringify({ error: "device_suspended" }), {
      status: 403,
      headers: { "Content-Type": "application/json", ...CORS_HEADERS },
    });
  }

  const jwk = JSON.parse(SIGNING_KEY_JSON);
  const privateKey = await importJWK(jwk, "ES256");

  const now = Math.floor(Date.now() / 1000);
  const token = await new SignJWT({
    role: "authenticated",
    sitio_id: dispositivo.sitio_id,
    tipo: dispositivo.tipo,
  })
    .setProtectedHeader({ alg: "ES256", kid: jwk.kid, typ: "JWT" })
    .setSubject(dispositivo.id)
    .setIssuedAt(now)
    .setExpirationTime(now + TOKEN_TTL_SECONDS)
    .sign(privateKey);

  // No bloquea la respuesta: si falla, no vale la pena tumbar el login por
  // esto -- last_seen_at/last_ip/metadata son informativos, no un mecanismo
  // de seguridad. metadata sólo viaja en la activación inicial (ver
  // Nucleo.configurarDispositivoInicialConSecreto) -- sólo se pisan los
  // campos que de verdad vinieron, para no borrar lo ya guardado en cada
  // renovación de token de rutina, que no manda nada de esto.
  const actualizacion: Record<string, string> = {
    last_seen_at: new Date().toISOString(),
  };
  if (ip) actualizacion.last_ip = ip;
  const metadata = body.metadata;
  if (metadata && typeof metadata === "object") {
    if (metadata.android_id) actualizacion.android_id = metadata.android_id;
    if (metadata.modelo) actualizacion.modelo = metadata.modelo;
    if (metadata.fabricante) actualizacion.fabricante = metadata.fabricante;
    if (metadata.fingerprint) actualizacion.fingerprint = metadata.fingerprint;
    if (metadata.app_version) actualizacion.app_version = metadata.app_version;
  }
  supabase
    .from("dispositivos")
    .update(actualizacion)
    .eq("id", dispositivo.id)
    .then(() => {});

  return new Response(
    JSON.stringify({
      access_token: token,
      expires_in: TOKEN_TTL_SECONDS,
      sitio_id: dispositivo.sitio_id,
      dispositivo_id: dispositivo.id,
      tipo: dispositivo.tipo,
    }),
    { status: 200, headers: { "Content-Type": "application/json", ...CORS_HEADERS } },
  );
});
