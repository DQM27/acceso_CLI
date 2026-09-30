// Identidad de dispositivos: códigos de vinculación, claves públicas de los
// equipos y tokens de sesión. Ver
// docs/features-futuras/propuesta-registro-dispositivos.md.

import type { SupabaseClient } from "jsr:@supabase/supabase-js@2";
import { SignJWT, importJWK, jwtVerify, type JWK } from "npm:jose@5";

const SIGNING_KEY_JSON = Deno.env.get("DEVICE_SIGNING_KEY")!;

/**
 * Vida del token de sesión. Antes eran 12 h; con la política restrictiva
 * `solo dispositivos vigentes` la revocación ya corta al instante, pero un
 * token corto sigue siendo la red de seguridad si algo falla, y renovarlo
 * es barato (el cliente lo pide bajo demanda y lo cachea).
 */
export const TTL_TOKEN_SEGUNDOS = 60 * 60;

/** Vida del desafío que el equipo firma para autenticarse. */
export const TTL_DESAFIO_SEGUNDOS = 120;

/** `aud` que debe llevar la aserción firmada por el equipo. */
export const AUDIENCIA_ASERCION = "device-auth";

const TIPO_DESAFIO = "desafio_dispositivo";

// ---- Códigos de vinculación ----

/**
 * Sin 0/O/1/I: se leen de una pantalla y a veces se escriben a mano. 32
 * símbolos dividen 256 exacto, así que `byte % 32` no sesga la elección.
 * 10 símbolos son 50 bits: imposible de adivinar en los minutos que vive.
 */
const ALFABETO_CODIGO = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
export const LARGO_CODIGO = 10;
export const VIGENCIA_CODIGO_MINUTOS = { porDefecto: 15, minima: 5, maxima: 24 * 60 };

function generarCodigo(): string {
  const bytes = new Uint8Array(LARGO_CODIGO);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => ALFABETO_CODIGO[b % ALFABETO_CODIGO.length]).join("");
}

/** Mayúsculas y sin separadores; lo que no sea del alfabeto se descarta. */
export function normalizarCodigo(entrada: string): string {
  return entrada.toUpperCase().replace(/[^0-9A-Z]/g, "");
}

/** `K7QMR4XT2P` → `K7QM-R4XT-2P`, para mostrar. */
export function formatearCodigo(codigo: string): string {
  return [codigo.slice(0, 4), codigo.slice(4, 8), codigo.slice(8)].filter(Boolean).join("-");
}

export async function sha256Hex(texto: string): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(texto));
  return Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Minutos de vigencia pedidos, acotados; el valor por defecto si no vino. */
export function vigenciaSolicitada(valor: unknown): number | null {
  if (valor === undefined || valor === null) return VIGENCIA_CODIGO_MINUTOS.porDefecto;
  if (typeof valor !== "number" || !Number.isInteger(valor)) return null;
  if (valor < VIGENCIA_CODIGO_MINUTOS.minima || valor > VIGENCIA_CODIGO_MINUTOS.maxima) return null;
  return valor;
}

export interface CodigoEmitido {
  codigo: string;
  expira_en: string;
}

/**
 * Emite un código nuevo para `dispositivoId` y anula los pendientes: sólo
 * puede haber uno vigente por dispositivo.
 */
export async function emitirCodigoVinculacion(
  supabase: SupabaseClient,
  dispositivoId: string,
  creadoPor: string,
  vigenciaMinutos: number,
): Promise<CodigoEmitido> {
  const ahora = new Date();
  const { error: anularError } = await supabase
    .from("codigos_vinculacion")
    .update({ anulado_en: ahora.toISOString() })
    .eq("dispositivo_id", dispositivoId)
    .is("usado_en", null)
    .is("anulado_en", null);
  if (anularError) throw anularError;

  const codigo = generarCodigo();
  const expiraEn = new Date(ahora.getTime() + vigenciaMinutos * 60_000).toISOString();
  const { error } = await supabase.from("codigos_vinculacion").insert({
    dispositivo_id: dispositivoId,
    codigo_hash: await sha256Hex(codigo),
    creado_por: creadoPor,
    expira_en: expiraEn,
  });
  if (error) throw error;

  return { codigo: formatearCodigo(codigo), expira_en: expiraEn };
}

// ---- Claves públicas de los equipos ----

export interface ClavePublicaP256 {
  kty: "EC";
  crv: "P-256";
  x: string;
  y: string;
}

const COORDENADA_BASE64URL = /^[A-Za-z0-9_-]{43}$/;

/**
 * La clave pública en forma canónica (sólo los cuatro miembros requeridos),
 * o `null` si no es una clave pública EC P-256 válida. Rechaza cualquier
 * JWK que traiga la parte privada (`d`): un cliente que la manda está roto.
 */
export async function clavePublicaValida(valor: unknown): Promise<ClavePublicaP256 | null> {
  if (!valor || typeof valor !== "object") return null;
  const jwk = valor as Record<string, unknown>;
  if (jwk.kty !== "EC" || jwk.crv !== "P-256" || "d" in jwk) return null;
  if (typeof jwk.x !== "string" || typeof jwk.y !== "string") return null;
  if (!COORDENADA_BASE64URL.test(jwk.x) || !COORDENADA_BASE64URL.test(jwk.y)) return null;

  const canonica: ClavePublicaP256 = { kty: "EC", crv: "P-256", x: jwk.x, y: jwk.y };
  try {
    // Falla si el punto no está en la curva.
    await importJWK(canonica, "ES256");
  } catch {
    return null;
  }
  return canonica;
}

function base64url(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/**
 * Huella RFC 7638: SHA-256 del JSON canónico de los miembros requeridos, en
 * orden lexicográfico y sin espacios. El equipo la calcula igual y la manda
 * como `kid` de su aserción.
 */
export async function huellaClave(clave: ClavePublicaP256): Promise<string> {
  const canonico = `{"crv":"${clave.crv}","kty":"${clave.kty}","x":"${clave.x}","y":"${clave.y}"}`;
  const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonico));
  return base64url(new Uint8Array(hash));
}

// ---- Tokens del servidor ----

let claveServidor: Promise<{ privada: CryptoKey; publica: CryptoKey; kid: string }> | null = null;

function clavesDelServidor() {
  claveServidor ??= (async () => {
    const jwk = JSON.parse(SIGNING_KEY_JSON) as JWK;
    const { d: _privada, ...publica } = jwk;
    return {
      privada: (await importJWK(jwk, "ES256")) as CryptoKey,
      publica: (await importJWK(publica, "ES256")) as CryptoKey,
      kid: jwk.kid ?? "",
    };
  })();
  return claveServidor;
}

export interface DispositivoAutenticable {
  id: string;
  sitio_id: string;
  tipo: string;
  clave_huella: string | null;
}

/**
 * Token de sesión del dispositivo, firmado con `DEVICE_SIGNING_KEY` (la
 * misma que Postgres acepta). Lleva la `huella` de la clave del equipo:
 * `private.dispositivo_vigente()` la compara con la vigente (y rechaza un
 * token sin huella), así que un equipo re-vinculado deja inválido al
 * instante el token del anterior.
 */
export async function emitirTokenDispositivo(dispositivo: DispositivoAutenticable) {
  const { privada, kid } = await clavesDelServidor();
  const ahora = Math.floor(Date.now() / 1000);
  const claims: Record<string, string> = {
    role: "authenticated",
    sitio_id: dispositivo.sitio_id,
    tipo: dispositivo.tipo,
  };
  if (dispositivo.clave_huella) claims.huella = dispositivo.clave_huella;

  const accessToken = await new SignJWT(claims)
    .setProtectedHeader({ alg: "ES256", kid, typ: "JWT" })
    .setSubject(dispositivo.id)
    .setIssuedAt(ahora)
    .setExpirationTime(ahora + TTL_TOKEN_SEGUNDOS)
    .sign(privada);

  return {
    access_token: accessToken,
    expires_in: TTL_TOKEN_SEGUNDOS,
    sitio_id: dispositivo.sitio_id,
    dispositivo_id: dispositivo.id,
    tipo: dispositivo.tipo,
  };
}

/**
 * Desafío sin estado: un JWT corto firmado por el servidor. El equipo lo
 * firma con su clave y lo devuelve; como lo emitió el servidor, no depende
 * del reloj del equipo (que en este sistema puede estar desfasado).
 */
export async function emitirDesafio(): Promise<string> {
  const { privada, kid } = await clavesDelServidor();
  const ahora = Math.floor(Date.now() / 1000);
  return await new SignJWT({ typ: TIPO_DESAFIO })
    .setProtectedHeader({ alg: "ES256", kid, typ: "JWT" })
    .setJti(crypto.randomUUID())
    .setIssuedAt(ahora)
    .setExpirationTime(ahora + TTL_DESAFIO_SEGUNDOS)
    .sign(privada);
}

/**
 * `true` si `asercion` está firmada con `clave` (la del equipo), va dirigida
 * a `device-auth` y contiene un desafío vigente emitido por este servidor.
 * Los tiempos que se validan son todos del servidor (los del desafío).
 */
export async function asercionValida(asercion: string, clave: ClavePublicaP256): Promise<boolean> {
  try {
    const claveEquipo = await importJWK(clave, "ES256");
    const { payload } = await jwtVerify(asercion, claveEquipo, {
      algorithms: ["ES256"],
      audience: AUDIENCIA_ASERCION,
      // El reloj del equipo no es confiable: `iat`/`exp` de la aserción no
      // se exigen; la frescura la garantiza el desafío.
      clockTolerance: Number.MAX_SAFE_INTEGER,
    });
    if (typeof payload.desafio !== "string") return false;

    const { publica } = await clavesDelServidor();
    const { payload: desafio } = await jwtVerify(payload.desafio, publica, { algorithms: ["ES256"] });
    return desafio.typ === TIPO_DESAFIO;
  } catch {
    return false;
  }
}

// ---- Auditoría ----

export type TipoEventoSeguridad =
  | "codigo_inexistente"
  | "codigo_usado"
  | "codigo_vencido"
  | "codigo_anulado"
  | "codigo_de_otro_dispositivo"
  | "firma_invalida"
  | "hardware_distinto";

export async function registrarEvento(
  supabase: SupabaseClient,
  evento: { tipo: TipoEventoSeguridad; dispositivo_id?: string | null; ip: string | null; detalle?: unknown },
): Promise<void> {
  const { error } = await supabase.from("eventos_seguridad_dispositivos").insert({
    tipo: evento.tipo,
    dispositivo_id: evento.dispositivo_id ?? null,
    ip: evento.ip,
    detalle: evento.detalle ?? null,
  });
  if (error) console.error("no se pudo registrar el evento de seguridad:", error.message);
}

// ---- Metadata del equipo ----

export interface MetadatosDispositivo {
  identificador_hardware: string | null;
  nombre_dispositivo: string | null;
  plataforma: string | null;
  version_build: string | null;
  app_version: string | null;
}

const CAMPOS_METADATA = [
  "identificador_hardware",
  "nombre_dispositivo",
  "plataforma",
  "version_build",
  "app_version",
] as const;

/** Sólo los campos conocidos, como texto recortado; lo demás se ignora. */
export function metadataSaneada(valor: unknown): MetadatosDispositivo {
  const origen = valor && typeof valor === "object" ? (valor as Record<string, unknown>) : {};
  const salida = {} as MetadatosDispositivo;
  for (const campo of CAMPOS_METADATA) {
    const texto = origen[campo];
    salida[campo] = typeof texto === "string" && texto.trim() ? texto.trim().slice(0, 200) : null;
  }
  return salida;
}
