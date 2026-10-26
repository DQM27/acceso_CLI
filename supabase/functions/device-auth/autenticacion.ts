// Qué responde `device-auth` a una aserción firmada, sin red ni base de
// datos: usa los `puertos` para buscar el equipo y verificar la firma.
// Separada de `index.ts` para probarla con `deno test` (autenticacion.test.ts).
//
// Regla de fondo (incidente del 2026-10-26: un 401 pasajero de la base
// interna salió como "credenciales inválidas" y el teléfono quedó marcado
// como retirado sin estarlo): **"no sé" nunca sale como "no"**.
//
// - 200: equipo vigente y firma suya.
// - 403 `dispositivo_retirado`: SÓLO con prueba -- la fila del equipo
//   existe, tiene `revoked_at` y la aserción está firmada con su clave. Es
//   la única respuesta que autoriza a un equipo a mostrarse como retirado.
// - 401: la nube no reconoce la clave, o la firma no es de esa clave. No es
//   un retiro: el equipo no se marca.
// - 400 `desafio_invalido`: el desafío venció o no es de este servidor. El
//   equipo pide otro y reintenta. (No es 401 a propósito: las apps viejas,
//   hasta v1.7.0, tomaban cualquier 401 como retiro.)
// - 503: cualquier cosa que la nube no pudo comprobar (la base no respondió,
//   sus claves no cargaron), después de reintentar. El equipo lo toma como
//   falta de conexión.

import type { ClavePublicaP256, ResultadoFirma } from "../_shared/dispositivos.ts";

/** Lo que `device-auth` necesita de la fila del equipo. */
export interface FilaEquipo {
  id: string;
  sitio_id: string;
  tipo: string;
  clave_huella: string | null;
  clave_publica_jwk: ClavePublicaP256 | null;
  identificador_hardware: string | null;
  etiqueta: string | null;
  sitios: { nombre: string } | null;
  revoked_at: string | null;
}

export interface Puertos {
  /** `kid` del encabezado de la aserción (la huella de la clave del
   * equipo), o null si no se puede leer. */
  huellaDe(asercion: string): string | null;
  /** La fila del equipo con esa huella, retirado o no; null sólo si de
   * verdad no existe. Lanza si la base no respondió bien: eso nunca es "no
   * existe". */
  buscarPorHuella(huella: string): Promise<FilaEquipo | null>;
  /** Lanza si el servidor no pudo verificar (por ejemplo, sus claves). */
  verificarFirma(asercion: string, clave: ClavePublicaP256): Promise<ResultadoFirma>;
  /** Evento de seguridad para el panel. Nunca debe cambiar la respuesta. */
  registrarFirmaInvalida(dispositivoId: string): Promise<void>;
  esperar(ms: number): Promise<void>;
}

export type Decision =
  | { tipo: "vigente"; equipo: FilaEquipo }
  | { tipo: "retirado"; dispositivo_id: string; retirado_en: string }
  | { tipo: "no_reconocido"; motivo: "asercion_ilegible" | "equipo_desconocido" | "firma_invalida" }
  | { tipo: "desafio_invalido" }
  | { tipo: "no_verificable"; detalle: string };

/** Esperas entre intentos de leer la base: tres intentos en total, ~0,6 s
 * como mucho antes de rendirse con 503. */
export const ESPERAS_REINTENTO_MS = [150, 450];

export async function decidirAutenticacion(asercion: string, puertos: Puertos): Promise<Decision> {
  const huella = puertos.huellaDe(asercion);
  if (!huella) return { tipo: "no_reconocido", motivo: "asercion_ilegible" };

  let equipo: FilaEquipo | null;
  try {
    equipo = await conReintentos(() => puertos.buscarPorHuella(huella), puertos);
  } catch (error) {
    return { tipo: "no_verificable", detalle: `no se pudo leer el equipo: ${mensajeDe(error)}` };
  }
  if (!equipo?.clave_publica_jwk) return { tipo: "no_reconocido", motivo: "equipo_desconocido" };

  let firma: ResultadoFirma;
  try {
    firma = await puertos.verificarFirma(asercion, equipo.clave_publica_jwk);
  } catch (error) {
    return { tipo: "no_verificable", detalle: `no se pudo verificar la firma: ${mensajeDe(error)}` };
  }
  if (firma === "desafio_invalido") return { tipo: "desafio_invalido" };
  if (firma === "firma_invalida") {
    try {
      await puertos.registrarFirmaInvalida(equipo.id);
    } catch (error) {
      console.error("no se pudo registrar la firma inválida:", mensajeDe(error));
    }
    return { tipo: "no_reconocido", motivo: "firma_invalida" };
  }

  // La firma es de la clave de esta fila: ahora sí, lo que diga la fila es
  // la verdad sobre este equipo.
  if (equipo.revoked_at) {
    return { tipo: "retirado", dispositivo_id: equipo.id, retirado_en: equipo.revoked_at };
  }
  return { tipo: "vigente", equipo };
}

/** Respuesta HTTP de todo lo que no es "vigente" (ese emite el token). */
export function respuestaDeRechazo(
  decision: Exclude<Decision, { tipo: "vigente" }>,
): { estado: 400 | 401 | 403 | 503; cuerpo: Record<string, string> } {
  switch (decision.tipo) {
    case "retirado":
      return { estado: 403, cuerpo: { error: "dispositivo_retirado", retirado_en: decision.retirado_en } };
    case "no_reconocido":
      return { estado: 401, cuerpo: { error: decision.motivo } };
    case "desafio_invalido":
      return { estado: 400, cuerpo: { error: "desafio_invalido" } };
    case "no_verificable":
      return { estado: 503, cuerpo: { error: "no_verificable" } };
  }
}

async function conReintentos<T>(operacion: () => Promise<T>, puertos: Puertos): Promise<T> {
  for (const espera of ESPERAS_REINTENTO_MS) {
    try {
      return await operacion();
    } catch (error) {
      console.warn(`device-auth: reintento en ${espera} ms:`, mensajeDe(error));
      await puertos.esperar(espera);
    }
  }
  return await operacion();
}

function mensajeDe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
