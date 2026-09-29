import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";
import { emitirCodigoVinculacion, vigenciaSolicitada } from "../_shared/dispositivos.ts";

// "Re-vincular" desde el panel: emite un código nuevo para un dispositivo
// que ya existe (equipo reemplazado, reinstalado o sospechoso). Conserva el
// `dispositivo_id` y todo su historial. Al canjearse, la clave del equipo
// anterior deja de servir en el acto (ver revocacion_efectiva_dispositivos).
// También sirve para reenviar un código que venció sin usarse.
//
//   { "dispositivo_id": uuid, "vigencia_minutos"?: 5..1440 }

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const dispositivoId = textoOpcional(cuerpo?.dispositivo_id);
  const vigencia = vigenciaSolicitada(cuerpo?.vigencia_minutos);
  if (!dispositivoId || vigencia === null) return json({ error: "bad_request" }, 400);

  const { data: dispositivo } = await supabase
    .from("dispositivos")
    .select("id, revoked_at")
    .eq("id", dispositivoId)
    .maybeSingle();
  if (!dispositivo) return json({ error: "dispositivo_inexistente" }, 404);
  // Revocar es definitivo: para volver a usar el equipo se da de alta uno nuevo.
  if (dispositivo.revoked_at) return json({ error: "dispositivo_revocado" }, 409);

  try {
    const emitido = await emitirCodigoVinculacion(supabase, dispositivo.id, correo, vigencia);
    return json({ dispositivo_id: dispositivo.id, ...emitido });
  } catch (error) {
    return json({ error: "codigo_error", detail: String(error) }, 500);
  }
});
