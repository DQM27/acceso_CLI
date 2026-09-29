import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, preflight } from "../_shared/http.ts";

// Todo lo que el panel necesita para la pantalla de Dispositivos:
// sitios, dispositivos (con su forma de autenticarse), el código de
// vinculación pendiente de cada uno y los eventos de seguridad recientes.

const EVENTOS_RECIENTES = 100;

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;

  const supabase = clienteServicio();
  if (!(await correoAdminAutorizado(req, supabase))) return json({ error: "unauthorized" }, 401);

  const [sitios, dispositivos, codigos, eventos] = await Promise.all([
    supabase.from("sitios").select("id, nombre, created_at").order("nombre"),
    supabase
      .from("dispositivos")
      .select(
        "id, sitio_id, tipo, etiqueta, created_at, revoked_at, suspended_at, last_seen_at, oculto_en_panel, " +
          "identificador_hardware, nombre_dispositivo, plataforma, version_build, app_version, last_ip, " +
          "clave_huella, vinculado_en, secret_hash",
      )
      .order("created_at", { ascending: false }),
    supabase
      .from("codigos_vinculacion")
      .select("dispositivo_id, expira_en, creado_en, creado_por")
      .is("usado_en", null)
      .is("anulado_en", null)
      .gt("expira_en", new Date().toISOString()),
    supabase
      .from("eventos_seguridad_dispositivos")
      .select("id, ocurrido_en, dispositivo_id, tipo, ip, detalle")
      .order("ocurrido_en", { ascending: false })
      .limit(EVENTOS_RECIENTES),
  ]);

  const fallo = sitios.error ?? dispositivos.error ?? codigos.error ?? eventos.error;
  if (fallo) return json({ error: "consulta_error", detail: fallo.message }, 500);

  // `secret_hash` nunca sale de acá: sólo se traduce a la forma en que el
  // equipo se autentica hoy.
  const filas = (dispositivos.data as unknown as Record<string, unknown>[]).map(({ secret_hash, ...fila }) => ({
    ...fila,
    credencial: fila.clave_huella ? "clave" : secret_hash ? "secreto_legado" : "sin_vincular",
  }));

  return json({
    sitios: sitios.data,
    dispositivos: filas,
    codigos_pendientes: codigos.data,
    eventos: eventos.data,
  });
});
