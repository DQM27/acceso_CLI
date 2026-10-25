import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { ipDelCliente, json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";
import { VIGENCIA_RECONEXION_MINUTOS, emitirCodigoVinculacion, registrarEvento } from "../_shared/dispositivos.ts";

// Un administrador autoriza que un equipo RETIRADO vuelva a conectarse (ver
// docs/features-futuras/plan-reconexion-equipo-retirado.md):
//
//   { "dispositivo_id": uuid, "sitio_id": uuid, "etiqueta": texto }
//
// Devuelve un código de reconexión de un solo uso para ese equipo, con la
// unidad y la etiqueta que va a tener (pueden ser las mismas). Un código de
// reconexión nunca sirve para dar de alta otro equipo, ni uno de alta para
// reconectar. Anula los códigos pendientes del equipo y deja el evento
// `reconexion_autorizada` con el correo del administrador.

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "unauthorized" }, 401);

  const cuerpo = await leerCuerpo(req);
  const dispositivoId = textoOpcional(cuerpo?.dispositivo_id);
  const sitioId = textoOpcional(cuerpo?.sitio_id);
  const etiqueta = textoOpcional(cuerpo?.etiqueta, 120);
  if (!dispositivoId || !sitioId || !etiqueta) {
    return json({ error: "bad_request", detail: "faltan campos o son inválidos" }, 400);
  }

  const { data: dispositivo } = await supabase
    .from("dispositivos")
    .select("id, revoked_at, sitio_id")
    .eq("id", dispositivoId)
    .maybeSingle();
  if (!dispositivo) return json({ error: "dispositivo_inexistente" }, 404);
  // Sólo un equipo retirado se reconecta; uno vigente sigue conectado.
  if (!dispositivo.revoked_at) return json({ error: "dispositivo_vigente" }, 409);

  const { data: sitio } = await supabase.from("sitios").select("id, nombre").eq("id", sitioId).maybeSingle();
  if (!sitio) return json({ error: "sitio_inexistente" }, 404);

  try {
    const emitido = await emitirCodigoVinculacion(supabase, dispositivo.id, correo, VIGENCIA_RECONEXION_MINUTOS, {
      sitio_destino_id: sitio.id,
      etiqueta_destino: etiqueta,
    });
    await registrarEvento(supabase, {
      tipo: "reconexion_autorizada",
      dispositivo_id: dispositivo.id,
      ip: ipDelCliente(req),
      detalle: {
        autorizado_por: correo,
        sitio_actual: dispositivo.sitio_id,
        sitio_destino: sitio.id,
        etiqueta_destino: etiqueta,
      },
    });
    return json({ dispositivo_id: dispositivo.id, sitio_id: sitio.id, sitio_nombre: sitio.nombre, etiqueta, ...emitido });
  } catch (errorCodigo) {
    // El detalle queda en el log de la función; al panel, un mensaje fijo.
    console.error("admin-autorizar-reconexion: no se pudo emitir el código:", errorCodigo);
    return json({ error: "codigo_error", detail: "No se pudo generar el código de reconexión" }, 500);
  }
});
