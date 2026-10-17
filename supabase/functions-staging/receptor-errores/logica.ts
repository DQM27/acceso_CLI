// Lógica pura del receptor de errores propio (sin Deno ni red, para poder
// probarla con Node): leer el envelope que manda el SDK de Sentry, sacarle
// sólo lo que nos interesa, limpiar datos personales y calcular la huella con
// que se agrupan los errores iguales.
//
// SÓLO staging. Plan: docs/features-futuras/plan-telemetria-ajustable.md.

export type ItemEnvelope = { cabecera: Record<string, unknown>; payload: Uint8Array };
export type Envelope = { encabezado: Record<string, unknown>; items: ItemEnvelope[] };

export type Frame = {
  funcion: string | null;
  modulo: string | null;
  archivo: string | null;
  linea: number | null;
  /** Código nuestro (no de una librería): el SDK lo marca `in_app`. */
  propio: boolean;
};

export type EventoLimpio = {
  event_id: string;
  ocurrido_en: string | null;
  plataforma: "movil" | "escritorio" | "otra";
  version_app: string | null;
  entorno: string | null;
  nivel: string;
  tipo: "excepcion" | "mensaje";
  titulo: string;
  traza: Frame[];
  contexto: Record<string, string>;
};

export const MAX_BYTES_COMPRIMIDOS = 200 * 1024;
export const MAX_BYTES_ENVELOPE = 1024 * 1024;
export const MAX_ITEMS = 5;
const MAX_FRAMES = 30;
const MAX_TITULO = 300;
const MAX_CAMPO = 120;

const decodificador = new TextDecoder();
const SALTO = 10;

function leerJson(bytes: Uint8Array): Record<string, unknown> | null {
  try {
    const valor = JSON.parse(decodificador.decode(bytes));
    return valor && typeof valor === "object" && !Array.isArray(valor) ? valor : null;
  } catch {
    return null;
  }
}

/**
 * Lee un envelope de Sentry: una línea de encabezado y, por cada ítem, una
 * línea de cabecera (`{"type":"event","length":123}`) seguida de su contenido.
 * Si la cabecera trae `length` el contenido son exactamente esos bytes; si no,
 * llega hasta el siguiente salto de línea. `null` si no es un envelope.
 */
export function leerEnvelope(bytes: Uint8Array): Envelope | null {
  let posicion = 0;
  const siguienteLinea = (): Uint8Array | null => {
    if (posicion >= bytes.length) return null;
    let fin = bytes.indexOf(SALTO, posicion);
    if (fin === -1) fin = bytes.length;
    const linea = bytes.subarray(posicion, fin);
    posicion = fin + 1;
    return linea;
  };

  const primera = siguienteLinea();
  const encabezado = primera ? leerJson(primera) : null;
  if (!encabezado) return null;

  const items: ItemEnvelope[] = [];
  while (posicion < bytes.length) {
    const linea = siguienteLinea();
    if (!linea || linea.length === 0) continue;
    const cabecera = leerJson(linea);
    if (!cabecera) return null;
    let payload: Uint8Array;
    const largo = cabecera.length;
    if (typeof largo === "number" && Number.isInteger(largo) && largo >= 0) {
      if (posicion + largo > bytes.length) return null;
      payload = bytes.subarray(posicion, posicion + largo);
      posicion += largo + 1; // el salto que cierra el contenido
    } else {
      payload = siguienteLinea() ?? new Uint8Array();
    }
    items.push({ cabecera, payload });
  }
  return { encabezado, items };
}

/**
 * Quita de un texto lo que puede identificar a una persona o dar acceso:
 * tokens, correos, usuarios del sistema en las rutas, cédulas, teléfonos,
 * placas y cualquier número largo. Se usa en el receptor aunque la app ya
 * limpie por su lado: dos defensas, porque un mensaje de error puede traer
 * una cédula sin que nadie lo decida.
 */
export function limpiarTexto(texto: string): string {
  return texto
    // Tokens: JWT y cadenas largas de base64/hex (también se lleva los UUID).
    .replace(/eyJ[\w-]+\.[\w-]+\.[\w-]+/g, "<token>")
    .replace(/[A-Za-z0-9+/_-]{32,}={0,2}/g, "<token>")
    .replace(/[\w.+-]+@[\w-]+(\.[\w-]+)+/g, "<correo>")
    // Rutas con el usuario del sistema.
    .replace(/([A-Za-z]:[\\/]+Users[\\/]+)[^\\/\s"']+/gi, "$1<usuario>")
    .replace(/(\/(?:home|Users)\/)[^/\s"']+/g, "$1<usuario>")
    // Cédula con guiones (1-0123-0456) y números largos (cédula, DIMEX, tarjetas).
    .replace(/\b\d-\d{4}-\d{4}\b/g, "<num>")
    .replace(/\b\d{7,}\b/g, "<num>")
    // Teléfonos de 8 dígitos (8888-8888, 8888 8888).
    .replace(/\b\d{4}[- ]\d{4}\b/g, "<num>")
    // Placas (ABC123, ABC-1234).
    .replace(/\b[A-Z]{3}[- ]?\d{3,4}\b/g, "<placa>")
    // Los datos de una URL van en la query.
    .replace(/\?[^\s"')]+/g, "?<datos>");
}

function texto(valor: unknown, largo = MAX_CAMPO): string | null {
  if (typeof valor !== "string") return null;
  const limpio = limpiarTexto(valor.trim());
  return limpio ? limpio.slice(0, largo) : null;
}

/**
 * La versión o el entorno que declara la app (`control-acceso-desktop@1.8.11`,
 * `com.dqm27.lattis@1.4.13+35`, `production`): son datos nuestros, no de una
 * persona, y el limpiador los confundiría con un correo. Se aceptan sólo con
 * un patrón estricto; lo que no encaja se descarta.
 */
function etiquetaSegura(valor: unknown): string | null {
  return typeof valor === "string" && /^[\w.@+() -]{1,80}$/.test(valor.trim()) ? valor.trim() : null;
}

function plataformaDe(evento: Record<string, unknown>): EventoLimpio["plataforma"] {
  const sdk = (evento.sdk as { name?: unknown } | undefined)?.name;
  const nombre = typeof sdk === "string" ? sdk.toLowerCase() : "";
  if (nombre.includes("android") || nombre.includes("java")) return "movil";
  if (nombre.includes("rust")) return "escritorio";
  return "otra";
}

function framesDe(valor: unknown): Frame[] {
  const frames = (valor as { frames?: unknown } | undefined)?.frames;
  if (!Array.isArray(frames)) return [];
  // El SDK manda del más viejo al más nuevo: interesan los últimos.
  return frames.slice(-MAX_FRAMES).map((f: Record<string, unknown>) => ({
    funcion: texto(f.function),
    modulo: texto(f.module),
    archivo: texto(f.filename ?? f.abs_path),
    linea: typeof f.lineno === "number" ? f.lineno : null,
    propio: f.in_app === true,
  }));
}

function contextoDe(evento: Record<string, unknown>): Record<string, string> {
  const contextos = (evento.contexts ?? {}) as Record<string, Record<string, unknown> | undefined>;
  const salida: Record<string, string> = {};
  const poner = (clave: string, valor: unknown) => {
    const t = texto(valor, 80);
    if (t) salida[clave] = t;
  };
  // Lista blanca: nada de usuario, servidor, rastro de pasos (breadcrumbs) ni
  // petición, que son los campos donde se cuela información personal.
  poner("os", contextos.os?.name);
  poner("os_version", contextos.os?.version);
  poner("modelo", contextos.device?.model);
  poner("arquitectura", contextos.device?.arch);
  poner("runtime", contextos.runtime?.version ?? contextos.rust?.version);
  return salida;
}

/** El evento ya limpio y reducido a lo que se guarda, o `null` si no sirve. */
export function extraerEvento(payload: Uint8Array): EventoLimpio | null {
  const evento = leerJson(payload);
  if (!evento || typeof evento.event_id !== "string") return null;

  const valores = (evento.exception as { values?: unknown } | undefined)?.values;
  const principal = Array.isArray(valores) ? (valores[valores.length - 1] as Record<string, unknown>) : null;

  let tipo: EventoLimpio["tipo"];
  let titulo: string | null;
  let traza: Frame[] = [];
  if (principal) {
    tipo = "excepcion";
    const clase = typeof principal.type === "string" ? principal.type : "Error";
    const mensaje = typeof principal.value === "string" ? principal.value : "";
    titulo = texto(mensaje ? `${clase}: ${mensaje}` : clase, MAX_TITULO);
    traza = framesDe(principal.stacktrace);
  } else {
    tipo = "mensaje";
    const mensaje = evento.message;
    const formateado = typeof mensaje === "string"
      ? mensaje
      : ((mensaje as { formatted?: unknown } | undefined)?.formatted ??
        (evento.logentry as { formatted?: unknown; message?: unknown } | undefined)?.formatted ??
        (evento.logentry as { message?: unknown } | undefined)?.message);
    titulo = texto(formateado, MAX_TITULO);
  }
  if (!titulo) return null;

  const marcaDeTiempo = typeof evento.timestamp === "number"
    ? new Date(evento.timestamp * 1000)
    : typeof evento.timestamp === "string"
    ? new Date(evento.timestamp)
    : null;

  return {
    event_id: evento.event_id.slice(0, 64),
    ocurrido_en: marcaDeTiempo && !Number.isNaN(marcaDeTiempo.getTime()) ? marcaDeTiempo.toISOString() : null,
    plataforma: plataformaDe(evento),
    version_app: etiquetaSegura(evento.release),
    entorno: etiquetaSegura(evento.environment),
    nivel: texto(evento.level, 20) ?? "error",
    tipo,
    titulo,
    traza,
    contexto: contextoDe(evento),
  };
}

/** El título sin lo que cambia de un evento a otro (números), para agrupar. */
function normalizarTitulo(titulo: string): string {
  return titulo.toLowerCase().replace(/\d+/g, "#").replace(/\s+/g, " ").trim();
}

async function sha256Hex(entrada: string): Promise<string> {
  const resumen = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(entrada));
  return Array.from(new Uint8Array(resumen), (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Huella con que se agrupan los errores iguales: plataforma, tipo, título sin
 * números y los últimos tres marcos de código propio (o de cualquiera si no
 * hay propios). 16 caracteres bastan para este volumen.
 */
export async function calcularHuella(evento: EventoLimpio): Promise<string> {
  const propios = evento.traza.filter((f) => f.propio);
  const marcos = (propios.length > 0 ? propios : evento.traza)
    .slice(-3)
    .map((f) => `${f.modulo ?? ""}.${f.funcion ?? ""}`);
  const base = [evento.plataforma, evento.tipo, normalizarTitulo(evento.titulo), ...marcos].join("|");
  return (await sha256Hex(base)).slice(0, 16);
}

/**
 * La clave pública del DSN que manda el SDK: en `X-Sentry-Auth`
 * (`Sentry sentry_key=…, sentry_version=7`) o en la query (`?sentry_key=…`).
 */
export function claveDeLaPeticion(encabezados: Headers, url: URL): string | null {
  const auth = encabezados.get("x-sentry-auth");
  const dentro = auth?.match(/sentry_key=([^,\s]+)/)?.[1];
  return dentro ?? url.searchParams.get("sentry_key");
}

/** Comparación en tiempo constante: la clave no se adivina por el tiempo. */
export function claveValida(recibida: string | null, esperada: string): boolean {
  if (!recibida || !esperada || recibida.length !== esperada.length) return false;
  let diferencia = 0;
  for (let i = 0; i < esperada.length; i++) diferencia |= recibida.charCodeAt(i) ^ esperada.charCodeAt(i);
  return diferencia === 0;
}
