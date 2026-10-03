// Firebase Cloud Messaging (API HTTP v1): notificaciones push a los
// teléfonos aunque la app esté cerrada. Ver
// mobile/android/.../NotificacionesPush.kt (lado que recibe) y la migración
// `tokens_push` (de dónde salen los destinatarios).
//
// Credencial: la cuenta de servicio de Firebase (proyecto `lattis-f823c`),
// el JSON completo en el secreto `FCM_SERVICE_ACCOUNT`
// (`supabase secrets set` o panel de Supabase → Edge Functions → Secrets).
// Nunca va al repo. Con ella se firma una aserción RS256 que Google cambia
// por un token OAuth de corta vida para llamar a FCM.

import { importPKCS8, SignJWT } from "npm:jose@5";

export interface CuentaServicio {
  project_id: string;
  client_email: string;
  private_key: string;
}

/** La cuenta de servicio del secreto, o `null` si falta o está incompleta. */
export function leerCuentaServicio(): CuentaServicio | null {
  const crudo = Deno.env.get("FCM_SERVICE_ACCOUNT");
  if (!crudo) return null;
  try {
    const cuenta = JSON.parse(crudo);
    if (
      typeof cuenta?.project_id !== "string" ||
      typeof cuenta?.client_email !== "string" ||
      typeof cuenta?.private_key !== "string"
    ) {
      return null;
    }
    return cuenta;
  } catch {
    return null;
  }
}

const ALCANCE_FCM = "https://www.googleapis.com/auth/firebase.messaging";
const URL_TOKEN_GOOGLE = "https://oauth2.googleapis.com/token";

// El token OAuth dura una hora: se reusa entre peticiones mientras la
// instancia de la función siga viva, con un minuto de margen.
let tokenCacheado: { valor: string; venceEnMs: number } | null = null;

export async function tokenAccesoGoogle(cuenta: CuentaServicio): Promise<string> {
  if (tokenCacheado && tokenCacheado.venceEnMs > Date.now()) return tokenCacheado.valor;

  const clave = await importPKCS8(cuenta.private_key, "RS256");
  const ahora = Math.floor(Date.now() / 1000);
  const asercion = await new SignJWT({ scope: ALCANCE_FCM })
    .setProtectedHeader({ alg: "RS256", typ: "JWT" })
    .setIssuer(cuenta.client_email)
    .setAudience(URL_TOKEN_GOOGLE)
    .setIssuedAt(ahora)
    .setExpirationTime(ahora + 3600)
    .sign(clave);

  const respuesta = await fetch(URL_TOKEN_GOOGLE, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
      assertion: asercion,
    }),
  });
  const cuerpo = await respuesta.json().catch(() => null);
  if (!respuesta.ok || typeof cuerpo?.access_token !== "string") {
    throw new Error(`Google no entregó el token OAuth (${respuesta.status}): ${JSON.stringify(cuerpo)}`);
  }
  const duracion = typeof cuerpo.expires_in === "number" ? cuerpo.expires_in : 3600;
  tokenCacheado = { valor: cuerpo.access_token, venceEnMs: Date.now() + (duracion - 60) * 1000 };
  return cuerpo.access_token;
}

export type TipoAviso = "normal" | "emergente";

export interface Aviso {
  titulo: string;
  cuerpo: string;
  tipo: TipoAviso;
}

/**
 * Mensaje de **datos** (sin bloque `notification`): con prioridad alta llega
 * a `onMessageReceived` aunque la app esté cerrada, y la notificación la arma
 * la app (canal "avisos" o "emergencias" según `tipo`, ícono, toque). FCM
 * exige que todos los valores de `data` sean texto.
 *
 * `ttl`: si el teléfono está apagado, un aviso normal espera hasta un día;
 * uno emergente, una hora (pasado eso ya no sirve y confundiría).
 */
export function mensajeFcm(token: string, aviso: Aviso) {
  return {
    message: {
      token,
      data: { titulo: aviso.titulo, cuerpo: aviso.cuerpo, tipo: aviso.tipo },
      android: { priority: "HIGH", ttl: aviso.tipo === "emergente" ? "3600s" : "86400s" },
    },
  };
}

/**
 * `token_invalido`: FCM dice que ese token ya no existe (app desinstalada,
 * datos borrados): quien llama lo borra de `tokens_push`.
 */
export type ResultadoEnvio = "enviado" | "token_invalido" | "error";

export async function enviarMensaje(
  cuenta: CuentaServicio,
  tokenAcceso: string,
  token: string,
  aviso: Aviso,
): Promise<ResultadoEnvio> {
  const respuesta = await fetch(`https://fcm.googleapis.com/v1/projects/${cuenta.project_id}/messages:send`, {
    method: "POST",
    headers: { "Authorization": `Bearer ${tokenAcceso}`, "Content-Type": "application/json" },
    body: JSON.stringify(mensajeFcm(token, aviso)),
  });
  if (respuesta.ok) return "enviado";

  const cuerpo = await respuesta.json().catch(() => null);
  const detalles: Array<{ errorCode?: string }> = Array.isArray(cuerpo?.error?.details) ? cuerpo.error.details : [];
  const codigo = detalles.find((d) => typeof d?.errorCode === "string")?.errorCode;
  if (respuesta.status === 404 || codigo === "UNREGISTERED") return "token_invalido";

  console.error(`FCM rechazó el envío (${respuesta.status}):`, JSON.stringify(cuerpo));
  return "error";
}
