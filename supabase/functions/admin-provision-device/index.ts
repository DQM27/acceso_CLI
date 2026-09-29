import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";
import { emitirCodigoVinculacion, vigenciaSolicitada } from "../_shared/dispositivos.ts";

// Alta de un dispositivo nuevo desde el panel. Ya no genera un secreto
// permanente: crea el dispositivo sin credencial y devuelve un código de
// vinculación de un solo uso (ver device-vincular). El equipo genera su
// propia clave al canjearlo.
//
//   { "sitio_id": uuid, "tipo": "pc" | "mobile" | "visor", "etiqueta": texto,
//     "vigencia_minutos"?: 5..1440 }
//
// El sitio se recibe por id y debe existir: antes se recibía por nombre con
// un upsert, y un error de tipeo creaba un sitio nuevo en silencio.

const TIPOS = ["pc", "mobile", "visor"];

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const sitioId = textoOpcional(cuerpo?.sitio_id);
  const tipo = textoOpcional(cuerpo?.tipo);
  const etiqueta = textoOpcional(cuerpo?.etiqueta, 120);
  const vigencia = vigenciaSolicitada(cuerpo?.vigencia_minutos);
  if (!sitioId || !tipo || !TIPOS.includes(tipo) || !etiqueta || vigencia === null) {
    return json({ error: "bad_request", detail: "faltan campos o son inválidos" }, 400);
  }

  const { data: sitio } = await supabase.from("sitios").select("id, nombre").eq("id", sitioId).maybeSingle();
  if (!sitio) return json({ error: "sitio_inexistente" }, 404);

  const { data: dispositivo, error } = await supabase
    .from("dispositivos")
    .insert({ sitio_id: sitio.id, tipo, etiqueta })
    .select("id")
    .single();
  if (error || !dispositivo) return json({ error: "dispositivo_error", detail: error?.message }, 500);

  try {
    const emitido = await emitirCodigoVinculacion(supabase, dispositivo.id, correo, vigencia);
    return json({
      sitio_id: sitio.id,
      sitio_nombre: sitio.nombre,
      dispositivo_id: dispositivo.id,
      ...emitido,
    });
  } catch (errorCodigo) {
    // Sin código, el dispositivo recién creado no serviría de nada.
    await supabase.from("dispositivos").delete().eq("id", dispositivo.id);
    return json({ error: "codigo_error", detail: String(errorCodigo) }, 500);
  }
});
