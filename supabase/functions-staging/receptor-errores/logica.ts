// Lógica pura del receptor de errores propio (sin Deno ni red, para poder
// probarla con Node): leer los reportes que manda el registro de errores de
// las apps (`control_acceso::errores`), sacarles sólo lo que nos interesa,
// limpiar datos personales y calcular la huella con que se agrupan los
// errores iguales.
//
// SÓLO staging. Plan: docs/features-futuras/plan-telemetria-y-registro-de-errores.md.

export type Frame = {
  funcion: string | null;
  archivo: string | null;
  linea: number | null;
  /** Código nuestro (no de una librería ni del sistema). */
  propio: boolean;
};

/** Una de las últimas operaciones antes del error. */
export type OperacionLimpia = { nombre: string; ok: boolean | null; en: string | null };

export type EventoLimpio = {
  event_id: string;
  ocurrido_en: string | null;
  plataforma: "movil" | "escritorio" | "otra";
  version_app: string | null;
  entorno: string | null;
  nivel: "fatal" | "error";
  tipo: "excepcion";
  titulo: string;
  /** Del marco más viejo al más nuevo (donde ocurrió). */
  traza: Frame[];
  contexto: Record<string, string>;
  /** `panico`, `excepcion`, `anr`, `nativo`, `js` o `nucleo`. */
  origen: string;
  pantalla: string | null;
  operaciones: OperacionLimpia[];
};

export const MAX_BYTES = 512 * 1024;
/** Reportes por petición (las apps mandan de a 10). */
export const MAX_REPORTES = 10;
const MAX_FRAMES = 30;
const MAX_TITULO = 300;
const MAX_CAMPO = 120;
const MAX_OPERACIONES = 20;

const ORIGENES = ["panico", "excepcion", "anr", "nativo", "js", "nucleo"] as const;
/** Los que tiran la app (o el hilo): se guardan como `fatal`. */
const ORIGENES_FATALES = new Set(["panico", "excepcion", "anr", "nativo"]);
/** Claves de contexto que se aceptan: datos del equipo. */
const CLAVES_CONTEXTO = ["os", "os_version", "modelo", "fabricante", "arquitectura", "android_sdk", "ambiente"];

/**
 * Quita de un texto lo que puede identificar a una persona o dar acceso:
 * tokens, correos, usuarios del sistema en las rutas, cédulas, teléfonos,
 * placas y cualquier número largo. La app ya limpia con las mismas reglas
 * (`errores::limpieza` del núcleo): dos defensas, porque un mensaje de error
 * puede traer una cédula sin que nadie lo decida.
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

/**
 * La ruta de un archivo de la traza sin el usuario del sistema. Sólo eso: la
 * regla de tokens de `limpiarTexto` se comería las rutas largas del código
 * (`desktop/src-tauri/src/comandos/nube.rs`). Igual que `limpiar_ruta` del
 * núcleo.
 */
export function limpiarRuta(ruta: string): string {
  return ruta
    .replace(/([A-Za-z]:[\\/]+Users[\\/]+)[^\\/\s"']+/gi, "$1<usuario>")
    .replace(/(\/(?:home|Users)\/)[^/\s"']+/g, "$1<usuario>");
}

function texto(valor: unknown, largo = MAX_CAMPO): string | null {
  if (typeof valor !== "string") return null;
  const limpio = limpiarTexto(valor.trim());
  return limpio ? limpio.slice(0, largo) : null;
}

function ruta(valor: unknown): string | null {
  if (typeof valor !== "string") return null;
  const limpia = limpiarRuta(valor.trim());
  return limpia ? limpia.slice(0, MAX_CAMPO) : null;
}

/**
 * La versión que declara la app (`desktop-1.10.6+abc1234`, `1.8.12 (57)`): es
 * un dato nuestro, no de una persona, y el limpiador la confundiría con otra
 * cosa. Se acepta sólo con un patrón estricto; lo que no encaja se descarta.
 */
function etiquetaSegura(valor: unknown): string | null {
  return typeof valor === "string" && /^[\w.@+() -]{1,80}$/.test(valor.trim()) ? valor.trim() : null;
}

/** Un nombre técnico de operación (`registrar_ingreso`, `nube::sincronizar`). */
function nombreOperacion(valor: unknown): string | null {
  return typeof valor === "string" && /^[A-Za-z0-9_.:-]{1,60}$/.test(valor) ? valor : null;
}

/**
 * Un reporte (`control_acceso::errores::Reporte`, ver
 * `docs/telemetria-contrato.md`) ya limpio, o `null` si no sirve. La app ya lo
 * limpió; acá se limpia otra vez y se descarta todo lo que no esté en la
 * lista. La traza llega del marco más nuevo al más viejo y se guarda al revés.
 */
export function extraerReporte(valor: unknown): EventoLimpio | null {
  if (!valor || typeof valor !== "object" || Array.isArray(valor)) return null;
  const r = valor as Record<string, unknown>;
  if (typeof r.id !== "string" || !/^[\w-]{8,64}$/.test(r.id)) return null;
  const origen = ORIGENES.find((o) => o === r.origen);
  if (!origen) return null;
  const titulo = texto(r.titulo, MAX_TITULO);
  if (!titulo) return null;

  const marcos = Array.isArray(r.traza) ? r.traza.slice(0, MAX_FRAMES) : [];
  const traza: Frame[] = marcos
    .filter((m): m is Record<string, unknown> => !!m && typeof m === "object")
    .map((m) => ({
      funcion: texto(m.funcion),
      archivo: ruta(m.archivo),
      linea: typeof m.linea === "number" && Number.isInteger(m.linea) ? m.linea : null,
      propio: m.propio === true,
    }))
    .reverse();

  const operaciones: OperacionLimpia[] = (Array.isArray(r.operaciones) ? r.operaciones : [])
    .slice(-MAX_OPERACIONES)
    .flatMap((o) => {
      const op = (o ?? {}) as Record<string, unknown>;
      const nombre = nombreOperacion(op.nombre);
      if (!nombre) return [];
      const en = typeof op.en === "string" && !Number.isNaN(Date.parse(op.en)) ? new Date(op.en).toISOString() : null;
      return [{ nombre, ok: typeof op.ok === "boolean" ? op.ok : null, en }];
    });

  const contextoBruto = (r.contexto ?? {}) as Record<string, unknown>;
  const contexto: Record<string, string> = {};
  for (const clave of CLAVES_CONTEXTO) {
    const t = texto(contextoBruto[clave], 80);
    if (t) contexto[clave] = t;
  }

  const ocurrido = typeof r.ocurrido_en === "string" ? new Date(r.ocurrido_en) : null;
  return {
    event_id: r.id,
    ocurrido_en: ocurrido && !Number.isNaN(ocurrido.getTime()) ? ocurrido.toISOString() : null,
    plataforma: r.plataforma === "movil" || r.plataforma === "escritorio" ? r.plataforma : "otra",
    version_app: etiquetaSegura(r.version_app),
    entorno: contexto.ambiente ?? null,
    nivel: ORIGENES_FATALES.has(origen) ? "fatal" : "error",
    tipo: "excepcion",
    titulo,
    traza,
    contexto,
    origen,
    pantalla: texto(r.pantalla, 60),
    operaciones,
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
 * Huella con que se agrupan los errores iguales: plataforma, origen, título
 * sin números y los últimos tres marcos de código propio (o de cualquiera si
 * no hay propios). 16 caracteres bastan para este volumen.
 */
export async function calcularHuella(evento: EventoLimpio): Promise<string> {
  const propios = evento.traza.filter((f) => f.propio);
  const marcos = (propios.length > 0 ? propios : evento.traza)
    .slice(-3)
    .map((f) => `${f.funcion ?? ""}@${f.archivo ?? ""}`);
  const base = [evento.plataforma, evento.origen, normalizarTitulo(evento.titulo), ...marcos].join("|");
  return (await sha256Hex(base)).slice(0, 16);
}

/** Comparación en tiempo constante: la clave no se adivina por el tiempo. */
export function claveValida(recibida: string | null, esperada: string): boolean {
  if (!recibida || !esperada || recibida.length !== esperada.length) return false;
  let diferencia = 0;
  for (let i = 0; i < esperada.length; i++) diferencia |= recibida.charCodeAt(i) ^ esperada.charCodeAt(i);
  return diferencia === 0;
}
