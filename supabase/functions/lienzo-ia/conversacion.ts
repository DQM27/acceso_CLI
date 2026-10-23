import type Anthropic from "npm:@anthropic-ai/sdk@0.132.1";
import { validarLienzo, type Lienzo } from "../_shared/lienzo/esquema.ts";
import { herramientaEditarLienzo, INSTRUCCIONES } from "./instrucciones.ts";

/**
 * Un pedido al asistente del lienzo, sin red ni base de datos (llegan como
 * `Puertos`, para probarlo con un modelo simulado):
 *
 * 1. Quien llama es administrador del panel; el asistente no está apagado.
 * 2. Topes diarios (por persona y global), contados en `lienzo_ia_uso`.
 * 3. Una llamada al modelo con el lienzo actual y el pedido. Si devuelve un
 *    documento que no pasa `validarLienzo`, se le devuelven los errores y se
 *    reintenta UNA vez; si tampoco, se rechaza (nada inválido llega al panel).
 * 4. Se registra la consulta (tokens y costo incluidos) y se responde.
 */
export const MODELO = "claude-haiku-5-5";
/** Precio de Claude Haiku 5.5 por millón de tokens (pedidos de hasta 100K tokens). */
const PRECIO = { entrada: 0.1, salida: 0.5, lecturaCache: 0.01, escrituraCache: 0.125 };
const MAXIMO_TOKENS = 4000;
const LARGO_PREGUNTA = 2000;
const TURNOS_HISTORIAL = 6;

export interface Topes {
  porPersona: number;
  global: number;
}

export interface Registro {
  correo: string;
  pregunta: string;
  resultado: "editado" | "respondido" | "rechazado" | "tope" | "error";
  lienzo?: Lienzo | null;
  respuesta?: string | null;
  modelo?: string | null;
  tokens_entrada: number;
  tokens_salida: number;
  tokens_cache: number;
  costo_usd: number;
  reintentos: number;
  duracion_ms: number;
  error?: string | null;
}

export interface Puertos {
  autorizar(): Promise<string | null>;
  usoDeHoy(correo: string): Promise<{ propio: number; total: number }>;
  llamarModelo(pedido: Anthropic.MessageCreateParamsNonStreaming): Promise<Anthropic.Message>;
  registrar(registro: Registro): Promise<void>;
  ahora(): number;
}

export interface Configuracion {
  apagado: boolean;
  topes: Topes;
  /** Día de hoy en Costa Rica (AAAA-MM-DD), para que el modelo arme rangos de fechas. */
  hoy: string;
}

export type Respuesta =
  | { estado: 200; cuerpo: { tipo: "editado"; lienzo: Lienzo; mensaje: string } | { tipo: "respondido"; mensaje: string } }
  | { estado: 400 | 401 | 429 | 502 | 503; cuerpo: { error: string; detail: string } };

interface Turno {
  rol: "usuario" | "asistente";
  texto: string;
}

interface Entrada {
  pregunta: string;
  lienzo: Lienzo | null;
  historial: Turno[];
}

function leerEntrada(cuerpo: Record<string, unknown> | null): Entrada | string {
  if (!cuerpo) return "Falta el cuerpo del pedido.";
  const pregunta = typeof cuerpo.pregunta === "string" ? cuerpo.pregunta.trim() : "";
  if (!pregunta) return "Escriba qué quiere ver.";
  if (pregunta.length > LARGO_PREGUNTA) return `El pedido es muy largo (máximo ${LARGO_PREGUNTA} caracteres).`;
  // Un lienzo actual inválido no se le pasa al modelo: arranca de cero.
  const validado = cuerpo.lienzo == null ? null : validarLienzo(cuerpo.lienzo);
  const historial = Array.isArray(cuerpo.historial)
    ? cuerpo.historial
      .filter((t): t is Turno =>
        typeof t === "object" && t !== null && (t.rol === "usuario" || t.rol === "asistente") && typeof t.texto === "string"
      )
      .slice(-TURNOS_HISTORIAL * 2)
      .map((t) => ({ rol: t.rol, texto: t.texto.slice(0, LARGO_PREGUNTA) }))
    : [];
  return { pregunta, lienzo: validado?.ok ? validado.lienzo : null, historial };
}

/** El pedido del turno: contexto y datos marcados como datos, el pedido al final. */
function mensajeDelPedido(entrada: Entrada, hoy: string): string {
  const historial = entrada.historial.length
    ? `<historial>\n${entrada.historial.map((t) => `${t.rol === "usuario" ? "Administrador" : "Asistente"}: ${t.texto}`).join("\n")}\n</historial>\n`
    : "";
  const lienzo = entrada.lienzo ? JSON.stringify(entrada.lienzo) : "ninguno (está en la pantalla de resumen)";
  return `Hoy es ${hoy} (Costa Rica).\n${historial}<lienzo_actual>${lienzo}</lienzo_actual>\n<pedido>${entrada.pregunta}</pedido>`;
}

export function costoEnDolares(uso: Anthropic.Usage): number {
  const millon = 1_000_000;
  return (
    (uso.input_tokens * PRECIO.entrada +
      uso.output_tokens * PRECIO.salida +
      (uso.cache_read_input_tokens ?? 0) * PRECIO.lecturaCache +
      (uso.cache_creation_input_tokens ?? 0) * PRECIO.escrituraCache) /
    millon
  );
}

type Interpretacion =
  | { tipo: "editado"; lienzo: Lienzo; mensaje: string }
  | { tipo: "respondido"; mensaje: string }
  | { tipo: "invalido"; errores: string[]; idHerramienta: string | null };

function interpretar(respuesta: Anthropic.Message): Interpretacion {
  if (respuesta.stop_reason === "refusal") {
    return { tipo: "respondido", mensaje: "No puedo ayudar con eso. Puedo mostrarle datos de contratistas e ingresos." };
  }
  const texto = respuesta.content
    .filter((b): b is Anthropic.TextBlock => b.type === "text")
    .map((b) => b.text)
    .join("\n")
    .trim();
  const uso = respuesta.content.find((b): b is Anthropic.ToolUseBlock => b.type === "tool_use" && b.name === "editar_lienzo");
  if (!uso) {
    if (respuesta.stop_reason === "max_tokens" || !texto) {
      return { tipo: "invalido", errores: ["La respuesta quedó incompleta."], idHerramienta: null };
    }
    return { tipo: "respondido", mensaje: texto.slice(0, 4000) };
  }
  const entrada = uso.input as { mensaje?: unknown; lienzo?: unknown };
  const validado = validarLienzo(entrada.lienzo);
  if (!validado.ok) return { tipo: "invalido", errores: validado.errores, idHerramienta: uso.id };
  const mensaje = typeof entrada.mensaje === "string" && entrada.mensaje.trim() ? entrada.mensaje.trim() : texto || "Listo.";
  return { tipo: "editado", lienzo: validado.lienzo, mensaje: mensaje.slice(0, 400) };
}

function sumarUso(a: Anthropic.Usage | null, b: Anthropic.Usage): Anthropic.Usage {
  if (!a) return b;
  return {
    ...b,
    input_tokens: a.input_tokens + b.input_tokens,
    output_tokens: a.output_tokens + b.output_tokens,
    cache_read_input_tokens: (a.cache_read_input_tokens ?? 0) + (b.cache_read_input_tokens ?? 0),
    cache_creation_input_tokens: (a.cache_creation_input_tokens ?? 0) + (b.cache_creation_input_tokens ?? 0),
  };
}

export async function procesarPedido(
  cuerpo: Record<string, unknown> | null,
  config: Configuracion,
  puertos: Puertos,
): Promise<Respuesta> {
  if (config.apagado) {
    return { estado: 503, cuerpo: { error: "apagado", detail: "El asistente está apagado por ahora." } };
  }
  const correo = await puertos.autorizar();
  if (!correo) {
    return { estado: 401, cuerpo: { error: "no_autorizado", detail: "Sólo los administradores del panel pueden usar el asistente." } };
  }
  const entrada = leerEntrada(cuerpo);
  if (typeof entrada === "string") return { estado: 400, cuerpo: { error: "pedido_invalido", detail: entrada } };

  const inicio = puertos.ahora();
  const base = { correo, pregunta: entrada.pregunta, tokens_entrada: 0, tokens_salida: 0, tokens_cache: 0, costo_usd: 0, reintentos: 0 };

  const uso = await puertos.usoDeHoy(correo);
  if (uso.propio >= config.topes.porPersona || uso.total >= config.topes.global) {
    const detail = uso.propio >= config.topes.porPersona
      ? `Llegó al tope de ${config.topes.porPersona} consultas de hoy. Mañana se renueva.`
      : "El asistente llegó a su tope de consultas de hoy. Mañana se renueva.";
    await puertos.registrar({ ...base, resultado: "tope", duracion_ms: puertos.ahora() - inicio });
    return { estado: 429, cuerpo: { error: "tope", detail } };
  }

  const mensajes: Anthropic.MessageParam[] = [{ role: "user", content: mensajeDelPedido(entrada, config.hoy) }];
  const pedido = (): Anthropic.MessageCreateParamsNonStreaming => ({
    model: MODELO,
    max_tokens: MAXIMO_TOKENS,
    output_config: { effort: "low" },
    // Instrucciones y herramienta no cambian entre pedidos: van en caché.
    system: [{ type: "text", text: INSTRUCCIONES, cache_control: { type: "ephemeral" } }],
    tools: [herramientaEditarLienzo()],
    messages: mensajes,
  });

  let usoTotal: Anthropic.Usage | null = null;
  let reintentos = 0;
  let resultado: Interpretacion;
  try {
    let respuesta = await puertos.llamarModelo(pedido());
    usoTotal = sumarUso(usoTotal, respuesta.usage);
    resultado = interpretar(respuesta);
    if (resultado.tipo === "invalido") {
      // Un solo reintento, con los errores del validador.
      reintentos = 1;
      const aviso = `El documento no es válido:\n- ${resultado.errores.slice(0, 10).join("\n- ")}\nCorregilo y volvé a llamar a editar_lienzo con el documento completo.`;
      mensajes.push({ role: "assistant", content: respuesta.content });
      mensajes.push({
        role: "user",
        content: resultado.idHerramienta
          ? [{ type: "tool_result", tool_use_id: resultado.idHerramienta, is_error: true, content: aviso }]
          : aviso,
      });
      respuesta = await puertos.llamarModelo(pedido());
      usoTotal = sumarUso(usoTotal, respuesta.usage);
      resultado = interpretar(respuesta);
    }
  } catch (error) {
    const detalle = error instanceof Error ? error.message : String(error);
    await puertos.registrar({
      ...base,
      ...(usoTotal ? tokens(usoTotal) : {}),
      resultado: "error",
      modelo: MODELO,
      reintentos,
      error: detalle.slice(0, 2000),
      duracion_ms: puertos.ahora() - inicio,
    });
    return { estado: 502, cuerpo: { error: "modelo", detail: "El asistente no respondió. Intente de nuevo en un momento." } };
  }

  const comun = { ...base, ...tokens(usoTotal), modelo: MODELO, reintentos, duracion_ms: puertos.ahora() - inicio };
  if (resultado.tipo === "invalido") {
    await puertos.registrar({ ...comun, resultado: "rechazado", error: resultado.errores.join("; ").slice(0, 2000) });
    return {
      estado: 200,
      cuerpo: { tipo: "respondido", mensaje: "No pude armar esa vista. ¿Puede decirlo de otra forma o más concreto?" },
    };
  }
  if (resultado.tipo === "respondido") {
    await puertos.registrar({ ...comun, resultado: "respondido", respuesta: resultado.mensaje });
    return { estado: 200, cuerpo: { tipo: "respondido", mensaje: resultado.mensaje } };
  }
  await puertos.registrar({ ...comun, resultado: "editado", lienzo: resultado.lienzo, respuesta: resultado.mensaje });
  return { estado: 200, cuerpo: { tipo: "editado", lienzo: resultado.lienzo, mensaje: resultado.mensaje } };
}

function tokens(uso: Anthropic.Usage | null) {
  if (!uso) return { tokens_entrada: 0, tokens_salida: 0, tokens_cache: 0, costo_usd: 0 };
  return {
    tokens_entrada: uso.input_tokens + (uso.cache_creation_input_tokens ?? 0),
    tokens_salida: uso.output_tokens,
    tokens_cache: uso.cache_read_input_tokens ?? 0,
    costo_usd: Number(costoEnDolares(uso).toFixed(6)),
  };
}
