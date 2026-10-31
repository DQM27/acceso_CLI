import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { createClient } from "jsr:@supabase/supabase-js@2";
import { calcularHuella, claveValida, extraerReporte, MAX_BYTES, MAX_REPORTES } from "./logica.ts";

// Receptor de errores propio. SÓLO staging.
//
// Recibe lo que manda el registro de errores de las apps
// (`control_acceso::errores`): `POST .../receptor-errores/v1/errores` con la
// clave en `x-clave-errores` y el cuerpo `{"reportes": [...]}` (ver
// docs/telemetria-contrato.md). Cada error se limpia de datos personales
// (logica.ts), se agrupa por huella y se guarda con `registrar_error_app`
// (supabase/scripts/errores_app_staging.sql).
//
// Pública a propósito (`verify_jwt = false`): el gancho de pánicos no tiene un
// JWT de Supabase. Se autentica con la clave, que viaja dentro de las apps
// (sale del DSN que se compila en ellas; no es un secreto) y sólo identifica
// al emisor: el control real es el límite de tamaño y el de ritmo (120
// errores por minuto, en `registrar_error_app`). Nunca se registra el
// contenido de una petición.
//
// Secreto de la función: RECEPTOR_ERRORES_CLAVE (la clave del DSN).

const CLAVE = Deno.env.get("RECEPTOR_ERRORES_CLAVE") ?? "";
const supabase = createClient(
  Deno.env.get("SUPABASE_URL") ?? "",
  Deno.env.get("SUPABASE_SERVICE_ROLE_KEY") ?? "",
  { auth: { persistSession: false } },
);

function responder(estado: number, cuerpo: unknown = {}, extra: HeadersInit = {}): Response {
  return new Response(JSON.stringify(cuerpo), {
    status: estado,
    headers: { "Content-Type": "application/json", ...extra },
  });
}

Deno.serve(async (req: Request) => {
  const url = new URL(req.url);
  if (req.method !== "POST" || !/\/v1\/errores\/?$/.test(url.pathname)) {
    return responder(404, { error: "no_encontrado" });
  }
  if (!claveValida(req.headers.get("x-clave-errores"), CLAVE)) {
    return responder(401, { error: "clave_invalida" });
  }

  if (Number(req.headers.get("content-length") ?? 0) > MAX_BYTES) {
    return responder(413, { error: "demasiado_grande" });
  }
  const cuerpo = await req.arrayBuffer();
  if (cuerpo.byteLength > MAX_BYTES) return responder(413, { error: "demasiado_grande" });

  let reportes: unknown;
  try {
    reportes = (JSON.parse(new TextDecoder().decode(cuerpo)) as { reportes?: unknown })?.reportes;
  } catch {
    return responder(400, { error: "json_invalido" });
  }
  if (!Array.isArray(reportes)) return responder(400, { error: "sin_reportes" });

  const eventos = reportes.slice(0, MAX_REPORTES).map(extraerReporte).filter((e) => e !== null);
  for (const evento of eventos) {
    const huella = await calcularHuella(evento);
    const { data, error } = await supabase.rpc("registrar_error_app", { p: { ...evento, huella } });
    if (error) {
      // Sin el contenido: sólo el código, para no volcar datos al registro.
      console.error("registrar_error_app falló:", error.code ?? "sin_codigo");
      return responder(503, { error: "no_disponible" }, { "Retry-After": "60" });
    }
    if (data === "limite") return responder(429, { error: "limite" }, { "Retry-After": "60" });
  }
  return responder(200, { guardados: eventos.length });
});
