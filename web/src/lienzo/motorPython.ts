import type { Fila } from "./calculo";
import { URL_SANDBOX } from "./sandbox";

/**
 * Python del panel "analisis". El cálculo NO corre en el panel: va a
 * `lienzo.megabrisas.com/python.html` (carpeta `sandbox-lienzo/`), en un iframe
 * oculto con sandbox="allow-scripts" (origen anónimo: no ve la sesión, las
 * cookies ni esta página), donde Pyodide corre en un Web Worker sin red.
 *
 * El iframe se abre una sola vez, con el primer análisis, y queda abierto
 * mientras la pestaña siga abierta: Python arranca una vez por visita y los
 * cálculos siguientes son casi inmediatos. Los pedidos se reparten por `id`.
 */

export type EstadoPython = "apagado" | "cargando" | "listo" | "error";

export interface PedidoAnalisis {
  codigo: string;
  /** Las filas del lienzo (su fuente con sus filtros). */
  datos: Fila[];
  /** Las columnas de esas filas, para que el código sepa qué hay. */
  columnas: { nombre: string; tipo: string; etiqueta: string }[];
  /** Las columnas que tiene que devolver (las que declara el análisis). */
  salida: string[];
}

/** Respaldo por si el sandbox no contesta (incluye bajar Python la primera vez). */
const ESPERA_MAXIMA_MS = 120_000;
// Sin ".html": Cloudflare sirve la página así (con la extensión, redirige).
const URL_PYTHON = new URL("python", URL_SANDBOX).href;

let marco: HTMLIFrameElement | null = null;
let preparado = false;
let estado: EstadoPython = "apagado";
let siguienteId = 1;
const enEspera: { id: number; pedido: PedidoAnalisis }[] = [];
const pendientes = new Map<number, { resolver: (filas: Fila[]) => void; rechazar: (error: Error) => void; espera: ReturnType<typeof setTimeout> }>();
/** Las columnas que pidió cada pedido: lo que vuelve se recorta a ellas. */
const salidas = new Map<number, string[]>();
const oyentes = new Set<(estado: EstadoPython) => void>();

function cambiarEstado(nuevo: EstadoPython) {
  estado = nuevo;
  for (const oyente of oyentes) oyente(nuevo);
}

/** Avisa cuándo Python está cargando, listo o falló (para mostrar "Preparando Python…"). */
export function escucharEstadoPython(oyente: (estado: EstadoPython) => void): () => void {
  oyentes.add(oyente);
  oyente(estado);
  return () => oyentes.delete(oyente);
}

/** Una fila del resultado: sólo valores simples, con las columnas pedidas. */
function filaLimpia(valor: unknown, salida: string[]): Fila | null {
  if (!valor || typeof valor !== "object" || Array.isArray(valor)) return null;
  const fila: Fila = {};
  for (const columna of salida) {
    const v = (valor as Record<string, unknown>)[columna];
    fila[columna] = typeof v === "string" || typeof v === "boolean" || (typeof v === "number" && Number.isFinite(v)) ? v : null;
  }
  return fila;
}

function alRecibir(evento: MessageEvent) {
  if (!marco || evento.source !== marco.contentWindow) return;
  const m = evento.data as { tipo?: string; estado?: unknown; id?: unknown; filas?: unknown; error?: unknown };
  if (m?.tipo === "preparado") {
    preparado = true;
    for (const { id, pedido } of enEspera.splice(0)) enviar(id, pedido);
  } else if (m?.tipo === "estado") {
    if (m.estado === "cargando" || m.estado === "listo" || m.estado === "error") cambiarEstado(m.estado);
  } else if (m?.tipo === "resultado" && typeof m.id === "number") {
    const pendiente = pendientes.get(m.id);
    if (!pendiente) return;
    pendientes.delete(m.id);
    clearTimeout(pendiente.espera);
    if (!Array.isArray(m.filas)) {
      pendiente.rechazar(new Error(typeof m.error === "string" ? m.error.slice(0, 400) : "El análisis falló."));
      return;
    }
    const salida = salidas.get(m.id) ?? [];
    salidas.delete(m.id);
    pendiente.resolver(m.filas.map((f) => filaLimpia(f, salida)).filter((f): f is Fila => f !== null));
  }
}

function enviar(id: number, pedido: PedidoAnalisis) {
  // "*": el marco tiene origen anónimo y no hay otro destino posible; sólo esa ventana lo recibe.
  marco?.contentWindow?.postMessage({ tipo: "calcular", id, ...pedido }, "*");
}

function abrirMarco() {
  if (marco) return;
  marco = document.createElement("iframe");
  marco.src = URL_PYTHON;
  marco.title = "Python del lienzo";
  marco.setAttribute("sandbox", "allow-scripts");
  marco.setAttribute("referrerpolicy", "no-referrer");
  marco.setAttribute("aria-hidden", "true");
  marco.tabIndex = -1;
  marco.className = "lienzo-motor-python";
  window.addEventListener("message", alRecibir);
  document.body.append(marco);
  cambiarEstado("cargando");
}

/**
 * El resultado con los tipos que declara el análisis: un número que vino como
 * texto ("12") pasa a número, una fecha queda como "AAAA-MM-DD" y lo que no
 * encaja queda vacío (así un gráfico no suma textos ni ordena mal).
 */
export function ajustarTipos(filas: Fila[], columnas: { nombre: string; tipo: string }[]): Fila[] {
  return filas.map((fila) => {
    const ajustada: Fila = {};
    for (const { nombre, tipo } of columnas) {
      const v = fila[nombre] ?? null;
      if (v === null) ajustada[nombre] = null;
      else if (tipo === "numero") {
        const n = typeof v === "number" ? v : typeof v === "string" && v.trim() !== "" ? Number(v) : Number.NaN;
        ajustada[nombre] = Number.isFinite(n) ? n : null;
      } else if (tipo === "fecha") {
        ajustada[nombre] = typeof v === "string" && /^\d{4}-\d{2}-\d{2}/.test(v) ? v.slice(0, 10) : null;
      } else if (tipo === "booleano") {
        ajustada[nombre] = typeof v === "boolean" ? v : null;
      } else {
        ajustada[nombre] = String(v);
      }
    }
    return ajustada;
  });
}

/** Corre el análisis en Python con las filas dadas; devuelve las filas del resultado o un error con el motivo. */
export function calcularConPython(pedido: PedidoAnalisis): Promise<Fila[]> {
  abrirMarco();
  const id = siguienteId++;
  salidas.set(id, pedido.salida);
  return new Promise<Fila[]>((resolver, rechazar) => {
    const espera = setTimeout(() => {
      pendientes.delete(id);
      salidas.delete(id);
      rechazar(new Error(`No se pudo usar Python (${new URL(URL_SANDBOX).host} no respondió).`));
    }, ESPERA_MAXIMA_MS);
    pendientes.set(id, { resolver, rechazar, espera });
    if (preparado) enviar(id, pedido);
    else enEspera.push({ id, pedido });
  });
}

/** Sólo para las pruebas: cierra el marco y olvida todo. */
export function reiniciarMotorPython() {
  window.removeEventListener("message", alRecibir);
  marco?.remove();
  marco = null;
  preparado = false;
  estado = "apagado";
  enEspera.length = 0;
  for (const p of pendientes.values()) clearTimeout(p.espera);
  pendientes.clear();
  salidas.clear();
}
