import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import Anthropic from "npm:@anthropic-ai/sdk@0.132.1";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { hoyCostaRica } from "../_shared/reglas.ts";
import { procesarPedido } from "./conversacion.ts";

// Asistente del lienzo de datos del panel (plan del asistente, Parte A, etapa 2).
// La lógica está en `conversacion.ts`; acá sólo se conecta con Supabase y con
// la API de Claude (clave en el secreto ANTHROPIC_API_KEY de la función).
//
// - Interruptor: el secreto LIENZO_IA_APAGADO=1 apaga el asistente sin desplegar.
// - Topes diarios: LIENZO_IA_TOPE_PERSONA (30) y LIENZO_IA_TOPE_GLOBAL (100).
// - El modelo nunca ve filas de datos ni toca la base: devuelve un documento de
//   lienzo que el panel valida otra vez y consulta con el JWT del usuario.
//
// Body: { pregunta, lienzo?, historial?: [{ rol, texto }] }.
// Responde { tipo: "editado", lienzo, mensaje } o { tipo: "respondido", mensaje },
// o { error, detail }.

const anthropic = new Anthropic({ apiKey: Deno.env.get("ANTHROPIC_API_KEY"), maxRetries: 1, timeout: 60_000 });

function entero(nombre: string, porOmision: number): number {
  const valor = Number(Deno.env.get(nombre));
  return Number.isInteger(valor) && valor >= 0 ? valor : porOmision;
}

/** Inicio del día de hoy en Costa Rica (UTC-6, sin horario de verano), en UTC. */
function inicioDeHoyUtc(hoy: string): string {
  return `${hoy}T06:00:00.000Z`;
}

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const hoy = hoyCostaRica();
  const respuesta = await procesarPedido(
    await leerCuerpo(req),
    {
      apagado: Deno.env.get("LIENZO_IA_APAGADO") === "1",
      topes: { porPersona: entero("LIENZO_IA_TOPE_PERSONA", 30), global: entero("LIENZO_IA_TOPE_GLOBAL", 100) },
      hoy,
    },
    {
      autorizar: () => correoAdminAutorizado(req, supabase),
      async usoDeHoy(correo) {
        const desde = inicioDeHoyUtc(hoy);
        // Las consultas frenadas por el tope no cuentan: no llamaron al modelo.
        const [propio, total] = await Promise.all([
          supabase.from("lienzo_ia_uso").select("id", { count: "exact", head: true })
            .eq("correo", correo).gte("creado_en", desde).neq("resultado", "tope"),
          supabase.from("lienzo_ia_uso").select("id", { count: "exact", head: true })
            .gte("creado_en", desde).neq("resultado", "tope"),
        ]);
        if (propio.error || total.error) throw new Error((propio.error ?? total.error)?.message);
        return { propio: propio.count ?? 0, total: total.count ?? 0 };
      },
      llamarModelo: (pedido) => anthropic.messages.create(pedido),
      async registrar(registro) {
        const { error } = await supabase.from("lienzo_ia_uso").insert(registro);
        if (error) console.error("lienzo-ia: no se pudo registrar la consulta:", error.message);
      },
      ahora: () => Date.now(),
    },
  );

  if (respuesta.estado === 200) console.log(`lienzo-ia: ${respuesta.cuerpo.tipo}`);
  return json(respuesta.cuerpo, respuesta.estado);
});
