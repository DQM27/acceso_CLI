import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import {
  type Aviso,
  enviarMensaje,
  leerCuentaServicio,
  type ResultadoEnvio,
  tokenAccesoGoogle,
} from "../_shared/fcm.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { type Destinatario, resumirEnvio, validarPedido } from "./envio.ts";

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
// Cada envío queda en `avisos_enviados` (quién, qué, a quién y a cuántos
// llegó). La fila se crea ANTES de mandar: su `id` viaja en el mensaje para
// que el teléfono no guarde dos veces el mismo aviso, y si no se puede
// guardar el historial no se manda nada (un aviso sin registro es justo lo
// que esta tabla vino a evitar).
//
// Sólo a equipos vigentes (no retirados) que hayan registrado su token
// (`tokens_push`, lo registra la app al iniciar sesión). Los tokens que FCM
// da por muertos se borran -- por el TOKEN, no por el equipo: el envío
// tarda (lotes de 10 contra FCM) y en ese lapso el teléfono puede haber
// registrado un token nuevo (`registrar_token_push` pisa la fila del
// equipo). Borrar por `dispositivo_id` se llevaba ese token nuevo y válido,
// y el equipo dejaba de recibir avisos hasta su próximo inicio de sesión.

const ENVIOS_EN_PARALELO = 10;
const TOKENS_POR_BORRADO = 50;

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "unauthorized" }, 401);

  const cuenta = leerCuentaServicio();
  if (!cuenta) return json({ error: "fcm_no_configurado" }, 500);

  const pedido = validarPedido(await leerCuerpo(req));
  if ("error" in pedido) return json({ error: pedido.error }, 400);
  const { titulo, cuerpo, tipo, destino } = pedido;

  let consulta = supabase
    .from("tokens_push")
    .select("token, dispositivos!inner(revoked_at, etiqueta)")
    .is("dispositivos.revoked_at", null);
  if (!("todos" in destino)) {
    // Los ids ya se validaron como UUID: no hay nada que escapar.
    const condiciones = [];
    if (destino.sitio_ids.length) condiciones.push(`sitio_id.in.(${destino.sitio_ids.join(",")})`);
    if (destino.dispositivo_ids.length) condiciones.push(`dispositivo_id.in.(${destino.dispositivo_ids.join(",")})`);
    consulta = consulta.or(condiciones.join(","));
  }
  const { data: filas, error } = await consulta;
  if (error) return json({ error: "consulta_error", detail: error.message }, 500);

  const destinatarios: Destinatario[] = ((filas ?? []) as unknown as Array<{
    token: string;
    dispositivos: { etiqueta: string };
  }>).map((fila) => ({ token: fila.token, etiqueta: fila.dispositivos.etiqueta }));

  let tokenAcceso: string;
  try {
    tokenAcceso = await tokenAccesoGoogle(cuenta);
  } catch (e) {
    console.error(e);
    return json({ error: "fcm_auth_error" }, 502);
  }

  const { data: registro, error: registroError } = await supabase
    .from("avisos_enviados")
    .insert({ titulo, cuerpo, tipo, destino, enviado_por: correo, destinatarios: destinatarios.length })
    .select("id, enviado_en")
    .single();
  if (registroError || !registro) {
    console.error("no se pudo guardar el aviso en el historial:", registroError?.message);
    return json({ error: "historial_error" }, 500);
  }

  const aviso: Aviso = { id: registro.id, enviado_en: registro.enviado_en, titulo, cuerpo, tipo };
  const resultados: ResultadoEnvio[] = [];
  for (let i = 0; i < destinatarios.length; i += ENVIOS_EN_PARALELO) {
    const lote = destinatarios.slice(i, i + ENVIOS_EN_PARALELO);
    resultados.push(
      ...await Promise.all(
        lote.map((d) => enviarMensaje(cuenta, tokenAcceso, d.token, aviso).catch(() => "error" as const)),
      ),
    );
  }
  const resumen = resumirEnvio(destinatarios, resultados);

  const { error: actualizarError } = await supabase
    .from("avisos_enviados")
    .update({ enviados: resumen.enviados, no_recibieron: resumen.no_recibieron })
    .eq("id", registro.id);
  if (actualizarError) console.error("no se pudo completar el historial del aviso:", actualizarError.message);

  // Sólo si la fila sigue teniendo el token muerto (ver arriba). En lotes:
  // el filtro viaja en la URL y cada token de FCM ronda los 160 caracteres.
  for (let i = 0; i < resumen.tokens_invalidos.length; i += TOKENS_POR_BORRADO) {
    const lote = resumen.tokens_invalidos.slice(i, i + TOKENS_POR_BORRADO);
    const { error: borrarError } = await supabase.from("tokens_push").delete().in("token", lote);
    if (borrarError) console.error("no se pudieron borrar tokens inválidos:", borrarError.message);
  }

  const fallidos = destinatarios.length - resumen.enviados;
  console.log(
    `admin-enviar-push: ${correo} envió el aviso ${registro.id} (${tipo}) a ${destinatarios.length} equipos ` +
      `(${resumen.enviados} enviados, ${fallidos} fallidos, ${resumen.tokens_invalidos.length} tokens inválidos)`,
  );
  return json({
    id: registro.id,
    destinatarios: destinatarios.length,
    enviados: resumen.enviados,
    fallidos,
    tokens_eliminados: resumen.tokens_invalidos.length,
    no_recibieron: resumen.no_recibieron,
  });
});
