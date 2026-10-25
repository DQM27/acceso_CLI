import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio } from "../_shared/admin.ts";
import { ipDelCliente, json, leerCuerpo, preflight } from "../_shared/http.ts";
import {
  COLUMNAS_NOMBRES,
  clavePublicaValida,
  emitirTokenDispositivo,
  huellaClave,
  metadataSaneada,
  nombresDe,
  registrarEvento,
  sha256Hex,
  type TipoEventoSeguridad,
} from "../_shared/dispositivos.ts";
import { leerPedidoReconexion } from "./pedido.ts";

// Reconexión de un equipo retirado con el código que emitió el panel
// (`admin-autorizar-reconexion`). Ver
// docs/features-futuras/plan-reconexion-equipo-retirado.md.
//
//   { "codigo": "K7QM-R4XT-2P", "confirmar": false }
//     → { sitio_id, sitio_nombre, etiqueta, cambio_de_unidad }
//       Sólo consulta a dónde va el equipo; el código NO se gasta. Con eso el
//       equipo avisa antes si al cambiar de unidad va a perder registros.
//
//   { "codigo": …, "confirmar": true, "clave_publica_jwk": {…}, "metadata"?,
//     "pendientes_descartados"?, "confirmado_por"? }
//     → el mismo token que `device-vincular`, más `cambio_de_unidad`.
//       `canjear_codigo_reconexion` reactiva el equipo con su clave NUEVA de
//       una sola vez y deja el evento con lo que se descartó y quién confirmó.
//
// Cualquier rechazo responde lo mismo (`codigo_invalido`) para no ayudar a
// adivinar; el motivo real queda en `eventos_seguridad_dispositivos` con la
// IP. Pública a propósito (`verify_jwt = false`): un equipo retirado ya no
// tiene credencial vigente.

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const cuerpo = await leerCuerpo(req);
  const supabase = clienteServicio();
  const ip = ipDelCliente(req);
  const metadata = metadataSaneada(cuerpo?.metadata);
  const pedido = leerPedidoReconexion(cuerpo);
  if (!pedido) {
    await registrarEvento(supabase, { tipo: "codigo_inexistente", ip, detalle: { reconexion: true, metadata } });
    return json({ error: "codigo_invalido" }, 401);
  }
  const codigoHash = await sha256Hex(pedido.codigo);

  if (!pedido.confirmar) {
    const { data } = await supabase.rpc("consultar_codigo_reconexion", { p_codigo_hash: codigoHash });
    const fila = (data as {
      dispositivo_id: string;
      sitio_actual_id: string;
      sitio_destino_id: string;
      sitio_destino_nombre: string;
      etiqueta_destino: string;
    }[] | null)?.[0];
    if (!fila) {
      await registrarRechazo(supabase, codigoHash, ip, metadata);
      return json({ error: "codigo_invalido" }, 401);
    }
    return json({
      sitio_id: fila.sitio_destino_id,
      sitio_nombre: fila.sitio_destino_nombre,
      etiqueta: fila.etiqueta_destino,
      cambio_de_unidad: fila.sitio_actual_id !== fila.sitio_destino_id,
    });
  }

  const clave = await clavePublicaValida(cuerpo?.clave_publica_jwk);
  if (!clave) return json({ error: "clave_publica_invalida" }, 400);
  const huella = await huellaClave(clave);

  const { data, error } = await supabase.rpc("canjear_codigo_reconexion", {
    p_codigo_hash: codigoHash,
    p_clave_publica_jwk: clave,
    p_clave_huella: huella,
    p_metadata: metadata,
    p_ip: ip,
    p_detalle: pedido.detalle,
  });

  // Índice único de `clave_huella`: esa clave ya es de otro dispositivo. Todo
  // se revirtió, incluido el uso del código.
  if (error?.code === "23505") return json({ error: "clave_en_uso" }, 409);
  if (error) {
    console.error("device-reconectar: canje fallido:", error.message);
    return json({ error: "reconexion_error" }, 500);
  }

  const reconectado = (data as { dispositivo_id: string; sitio_id: string; tipo: string; cambio_de_unidad: boolean }[] | null)
    ?.[0];
  if (!reconectado) {
    await registrarRechazo(supabase, codigoHash, ip, metadata);
    return json({ error: "codigo_invalido" }, 401);
  }

  // Sólo para mostrar en el equipo: si falla, el token sale igual sin nombres.
  const { data: nombres } = await supabase
    .from("dispositivos")
    .select(COLUMNAS_NOMBRES)
    .eq("id", reconectado.dispositivo_id)
    .maybeSingle();

  const token = await emitirTokenDispositivo({
    id: reconectado.dispositivo_id,
    sitio_id: reconectado.sitio_id,
    tipo: reconectado.tipo,
    clave_huella: huella,
    ...nombresDe(nombres),
  });
  return json({ ...token, cambio_de_unidad: reconectado.cambio_de_unidad });
});

/** Averigua por qué no se pudo usar el código y lo deja registrado. */
async function registrarRechazo(
  supabase: ReturnType<typeof clienteServicio>,
  codigoHash: string,
  ip: string | null,
  metadata: unknown,
): Promise<void> {
  const { data: fila } = await supabase
    .from("codigos_vinculacion")
    .select("dispositivo_id, proposito, usado_en, anulado_en, expira_en")
    .eq("codigo_hash", codigoHash)
    .maybeSingle();

  let tipo: TipoEventoSeguridad = "codigo_inexistente";
  let motivo: string | null = null;
  if (fila?.usado_en) tipo = "codigo_usado";
  else if (fila?.anulado_en) tipo = "codigo_anulado";
  else if (fila && new Date(fila.expira_en) <= new Date()) tipo = "codigo_vencido";
  // Un código de alta usado para reconectar, o un equipo que ya no está
  // retirado: se registra como inexistente, con el motivo en el detalle.
  else if (fila && fila.proposito !== "reconexion") motivo = "codigo_de_alta";
  else if (fila) motivo = "equipo_no_retirado";

  await registrarEvento(supabase, {
    tipo,
    dispositivo_id: fila?.dispositivo_id ?? null,
    ip,
    detalle: { reconexion: true, motivo, metadata },
  });
}
