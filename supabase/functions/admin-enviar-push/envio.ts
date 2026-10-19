import type { ResultadoEnvio, TipoAviso } from "../_shared/fcm.ts";
import { textoOpcional } from "../_shared/http.ts";

// Lógica pura de `admin-enviar-push` (sin red ni base): validar el pedido y
// resumir lo que respondió FCM. Probada en envio.test.ts.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const MAX_IDS = 500;

export type Destino = { todos: true } | { sitio_ids: string[]; dispositivo_ids: string[] };

export interface PedidoAviso {
  titulo: string;
  cuerpo: string;
  tipo: TipoAviso;
  destino: Destino;
}

/** Lista de UUID válidos, `[]` si no vino, o `null` si vino mal. */
function listaDeIds(valor: unknown): string[] | null {
  if (valor === undefined || valor === null) return [];
  if (!Array.isArray(valor) || valor.length > MAX_IDS) return null;
  if (!valor.every((id) => typeof id === "string" && UUID.test(id))) return null;
  return [...new Set(valor as string[])];
}

/** El pedido validado, o el código de error para responder 400. */
export function validarPedido(
  cuerpo: Record<string, unknown> | null,
): PedidoAviso | { error: "bad_request" | "sin_destino" } {
  const titulo = textoOpcional(cuerpo?.titulo, 100);
  const texto = textoOpcional(cuerpo?.cuerpo, 500);
  const tipo = cuerpo?.tipo ?? "normal";
  if (!titulo || !texto || (tipo !== "normal" && tipo !== "emergente")) return { error: "bad_request" };

  const destino = cuerpo?.destino as Record<string, unknown> | undefined;
  if (destino?.todos === true) return { titulo, cuerpo: texto, tipo, destino: { todos: true } };

  const sitio_ids = listaDeIds(destino?.sitio_ids);
  const dispositivo_ids = listaDeIds(destino?.dispositivo_ids);
  if (sitio_ids === null || dispositivo_ids === null) return { error: "bad_request" };
  if (sitio_ids.length === 0 && dispositivo_ids.length === 0) return { error: "sin_destino" };
  return { titulo, cuerpo: texto, tipo, destino: { sitio_ids, dispositivo_ids } };
}

export interface Destinatario {
  token: string;
  etiqueta: string;
}

export interface ResumenEnvio {
  enviados: number;
  /** Etiquetas de los equipos a los que no llegó (error o token muerto), sin
   * repetir y en orden alfabético. */
  no_recibieron: string[];
  /** Tokens que FCM dio por muertos: se borran de `tokens_push`. */
  tokens_invalidos: string[];
}

/** `resultados[i]` es la respuesta de FCM para `destinatarios[i]`. */
export function resumirEnvio(destinatarios: readonly Destinatario[], resultados: readonly ResultadoEnvio[]): ResumenEnvio {
  let enviados = 0;
  const noRecibieron = new Set<string>();
  const tokensInvalidos: string[] = [];
  destinatarios.forEach((destinatario, i) => {
    const resultado = resultados[i] ?? "error";
    if (resultado === "enviado") {
      enviados++;
      return;
    }
    noRecibieron.add(destinatario.etiqueta);
    if (resultado === "token_invalido") tokensInvalidos.push(destinatario.token);
  });
  return {
    enviados,
    no_recibieron: [...noRecibieron].sort((a, b) => a.localeCompare(b, "es")),
    tokens_invalidos: tokensInvalidos,
  };
}
