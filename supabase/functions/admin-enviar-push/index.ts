import "jsr:@supabase/functions-js/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { type Aviso, enviarMensaje, leerCuentaServicio, tokenAccesoGoogle } from "../_shared/fcm.ts";
import { json, leerCuerpo, preflight, textoOpcional } from "../_shared/http.ts";

// Notificación push desde el panel a los teléfonos que elija el
// administrador: todos, una o varias unidades, y/o equipos concretos. Llega
// aunque la app esté cerrada (ver _shared/fcm.ts).
//
// Body:
//   {
//     "titulo": "…",                 // obligatorio, hasta 100 caracteres
//     "cuerpo": "…",                 // obligatorio, hasta 500
//     "tipo": "normal" | "emergente", // opcional, "normal" por defecto
//     "destino": {
//       "todos": true,                // o bien una o ambas listas:
//       "sitio_ids": ["<uuid>", …],
//       "dispositivo_ids": ["<uuid>", …]
//     }
//   }
//
// Sólo a equipos vigentes (no retirados) que hayan registrado su token
// (`tokens_push`, lo registra la app al iniciar sesión). Los tokens que FCM
// da por muertos se borran -- por el TOKEN, no por el equipo: el envío
// tarda (lotes de 10 contra FCM) y en ese lapso el teléfono puede haber
// registrado un token nuevo (`registrar_token_push` pisa la fila del
// equipo). Borrar por `dispositivo_id` se llevaba ese token nuevo y válido,
// y el equipo dejaba de recibir avisos hasta su próximo inicio de sesión.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const MAX_IDS = 500;
const ENVIOS_EN_PARALELO = 10;
const TOKENS_POR_BORRADO = 50;

/** Lista de UUID válidos, `[]` si no vino, o `null` si vino mal. */
function listaDeIds(valor: unknown): string[] | null {
  if (valor === undefined || valor === null) return [];
  if (!Array.isArray(valor) || valor.length > MAX_IDS) return null;
  if (!valor.every((id) => typeof id === "string" && UUID.test(id))) return null;
  return [...new Set(valor as string[])];
}

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "unauthorized" }, 401);

  const cuenta = leerCuentaServicio();
  if (!cuenta) return json({ error: "fcm_no_configurado" }, 500);

  const cuerpo = await leerCuerpo(req);
  const titulo = textoOpcional(cuerpo?.titulo, 100);
  const texto = textoOpcional(cuerpo?.cuerpo, 500);
  const tipo = cuerpo?.tipo ?? "normal";
  if (!titulo || !texto || (tipo !== "normal" && tipo !== "emergente")) {
    return json({ error: "bad_request" }, 400);
  }

  const destino = cuerpo?.destino as Record<string, unknown> | undefined;
  const todos = destino?.todos === true;
  const sitioIds = listaDeIds(destino?.sitio_ids);
  const dispositivoIds = listaDeIds(destino?.dispositivo_ids);
  if (sitioIds === null || dispositivoIds === null) return json({ error: "bad_request" }, 400);
  if (!todos && sitioIds.length === 0 && dispositivoIds.length === 0) {
    return json({ error: "sin_destino" }, 400);
  }

  let consulta = supabase
    .from("tokens_push")
    .select("dispositivo_id, token, dispositivos!inner(revoked_at)")
    .is("dispositivos.revoked_at", null);
  if (!todos) {
    // Los ids ya se validaron como UUID: no hay nada que escapar.
    const condiciones = [];
    if (sitioIds.length) condiciones.push(`sitio_id.in.(${sitioIds.join(",")})`);
    if (dispositivoIds.length) condiciones.push(`dispositivo_id.in.(${dispositivoIds.join(",")})`);
    consulta = consulta.or(condiciones.join(","));
  }
  const { data: filas, error } = await consulta;
  if (error) return json({ error: "consulta_error", detail: error.message }, 500);

  const destinatarios = (filas ?? []) as Array<{ dispositivo_id: string; token: string }>;
  const aviso: Aviso = { titulo, cuerpo: texto, tipo };

  let tokenAcceso: string;
  try {
    tokenAcceso = await tokenAccesoGoogle(cuenta);
  } catch (e) {
    console.error(e);
    return json({ error: "fcm_auth_error" }, 502);
  }

  let enviados = 0;
  let fallidos = 0;
  const invalidos: string[] = [];
  for (let i = 0; i < destinatarios.length; i += ENVIOS_EN_PARALELO) {
    const lote = destinatarios.slice(i, i + ENVIOS_EN_PARALELO);
    const resultados = await Promise.all(
      lote.map((d) => enviarMensaje(cuenta, tokenAcceso, d.token, aviso).catch(() => "error" as const)),
    );
    resultados.forEach((resultado, j) => {
      if (resultado === "enviado") enviados++;
      else if (resultado === "token_invalido") invalidos.push(lote[j].token);
      else fallidos++;
    });
  }

  // Sólo si la fila sigue teniendo el token muerto (ver arriba). En lotes:
  // el filtro viaja en la URL y cada token de FCM ronda los 160 caracteres.
  for (let i = 0; i < invalidos.length; i += TOKENS_POR_BORRADO) {
    const lote = invalidos.slice(i, i + TOKENS_POR_BORRADO);
    const { error: borrarError } = await supabase.from("tokens_push").delete().in("token", lote);
    if (borrarError) console.error("no se pudieron borrar tokens inválidos:", borrarError.message);
  }

  console.log(
    `admin-enviar-push: ${correo} envió un aviso ${tipo} a ${destinatarios.length} equipos ` +
      `(${enviados} enviados, ${fallidos} fallidos, ${invalidos.length} tokens inválidos)`,
  );
  return json({
    destinatarios: destinatarios.length,
    enviados,
    fallidos,
    tokens_eliminados: invalidos.length,
  });
});
