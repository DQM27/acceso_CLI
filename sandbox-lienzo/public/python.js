// Python del lienzo (panel "analisis"), en lienzo.megabrisas.com/python.html.
//
// El panel abre esta página UNA vez, en un iframe oculto con
// sandbox="allow-scripts" (origen anónimo: no ve la sesión ni el panel), y la
// deja abierta mientras la pestaña siga abierta. Al cargar arranca Pyodide en
// un Web Worker (sin red, salvo para bajarse a sí mismo de este sitio); después
// atiende los cálculos de a uno: el panel manda el código y las filas, y vuelve
// sólo la lista de filas del resultado (o el error). Si un cálculo tarda
// demasiado se corta el Worker y se vuelve a arrancar.
import { fuenteDelTrabajadorPython } from "./python-motor.js";

const BASE = new URL("./pyodide/", import.meta.url).href;
const URL_TRABAJADOR = URL.createObjectURL(new Blob([fuenteDelTrabajadorPython(BASE)], { type: "text/javascript" }));

/** Únicos orígenes que pueden pedir un cálculo: el panel y el panel en desarrollo. */
const ORIGENES_PERMITIDOS = new Set([
  "https://megabrisas.com",
  "https://panel.megabrisas.com",
  "http://127.0.0.1:5173",
  "http://localhost:5173",
]);
/** Bajar y arrancar Pyodide la primera vez (con la conexión lenta de un sitio). */
const TIEMPO_CARGA_MS = 90_000;
/** Un cálculo; si se pasa, se corta y Python se vuelve a arrancar. */
const TIEMPO_CALCULO_MS = 15_000;

let origenPadre = null;
let trabajador = null;
let listo = false;
let limiteCarga = 0;
let limiteCalculo = 0;
/** Pedidos en espera y el que está corriendo. */
const cola = [];
let enCurso = null;

function avisar(mensaje) {
  if (origenPadre) window.parent.postMessage(mensaje, origenPadre);
}

function responder(id, respuesta) {
  avisar({ tipo: "resultado", id, ...respuesta });
}

function arrancar() {
  listo = false;
  avisar({ tipo: "estado", estado: "cargando" });
  let actual;
  try {
    // Worker clásico: con origen anónimo, Chrome no deja crear uno de tipo módulo
    // ("Refused to cross-origin redirects of the top-level worker script").
    actual = new Worker(URL_TRABAJADOR);
  } catch (error) {
    return fallarTodo(`No se pudo preparar Python: ${error instanceof Error ? error.message : error}`);
  }
  trabajador = actual;
  clearTimeout(limiteCarga);
  limiteCarga = setTimeout(() => {
    if (trabajador === actual && !listo) fallarTodo("Python tardó demasiado en cargar.");
  }, TIEMPO_CARGA_MS);

  actual.onmessage = (evento) => {
    if (trabajador !== actual) return;
    const m = evento.data ?? {};
    if (m.tipo === "listo") {
      clearTimeout(limiteCarga);
      listo = true;
      avisar({ tipo: "estado", estado: "listo" });
      siguiente();
    } else if (m.tipo === "error_carga") {
      fallarTodo(`No se pudo cargar Python: ${m.mensaje}`);
    } else if (m.tipo === "resultado" && enCurso && m.id === enCurso.id) {
      clearTimeout(limiteCalculo);
      const { id, filas, error } = m;
      enCurso = null;
      responder(id, Array.isArray(filas) ? { filas } : { error: String(error ?? "Error desconocido") });
      siguiente();
    }
  };
  actual.onerror = (evento) => {
    if (trabajador === actual) fallarTodo(evento.message || "Python se detuvo por un error.");
  };
}

/** Python no está disponible: avisa a todos los pedidos y espera al próximo para reintentar. */
function fallarTodo(mensaje) {
  clearTimeout(limiteCarga);
  clearTimeout(limiteCalculo);
  trabajador?.terminate();
  trabajador = null;
  listo = false;
  avisar({ tipo: "estado", estado: "error", mensaje });
  const pendientes = [...(enCurso ? [enCurso] : []), ...cola.splice(0)];
  enCurso = null;
  for (const pedido of pendientes) responder(pedido.id, { error: mensaje });
}

function siguiente() {
  if (!listo || enCurso || cola.length === 0) return;
  const pedido = cola.shift();
  enCurso = pedido;
  const actual = trabajador;
  limiteCalculo = setTimeout(() => {
    if (enCurso !== pedido) return;
    // Python no se puede interrumpir a mitad de un cálculo: se corta el Worker y se arranca otro.
    actual.terminate();
    trabajador = null;
    enCurso = null;
    responder(pedido.id, { error: `El análisis tardó más de ${TIEMPO_CALCULO_MS / 1000} segundos y se cortó.` });
    arrancar();
  }, TIEMPO_CALCULO_MS);
  actual.postMessage(pedido);
}

window.addEventListener("message", (evento) => {
  if (evento.source !== window.parent || !ORIGENES_PERMITIDOS.has(evento.origin)) return;
  origenPadre = evento.origin;
  const m = evento.data;
  if (m?.tipo !== "calcular" || typeof m.codigo !== "string") return;
  cola.push({
    id: m.id,
    codigo: m.codigo,
    datos: Array.isArray(m.datos) ? m.datos : [],
    columnas: Array.isArray(m.columnas) ? m.columnas : [],
    salida: Array.isArray(m.salida) ? m.salida.filter((c) => typeof c === "string") : [],
  });
  if (!trabajador) arrancar();
  siguiente();
});

// Arranca apenas carga: mientras el panel termina de consultar, Python ya se va bajando.
arrancar();
// Avisa que ya puede recibir (sin datos: cualquier origen puede saberlo).
window.parent.postMessage({ tipo: "preparado" }, "*");
