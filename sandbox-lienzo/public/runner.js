// Sandbox del panel "libre" del lienzo (lienzo.megabrisas.com).
//
// El panel lo abre en un iframe con sandbox="allow-scripts" y le manda, por
// postMessage, el código que escribió el asistente y las filas ya consultadas.
// El código corre en un Web Worker (sin DOM ni red, con tiempo límite); lo que
// devuelve se limpia y se muestra. Al panel sólo vuelve el alto del dibujo (y
// un mensaje si falló): nada de lo que pasa acá puede tocar la sesión del panel.
import { crearAyudas } from "./ayudas.js";
import { limpiarDibujo } from "./limpiar.js";
import { fuenteDelTrabajador } from "./trabajador.js";

// El Worker se arma en memoria (blob:): con origen anónimo no se puede cargar desde un archivo.
const URL_TRABAJADOR = URL.createObjectURL(new Blob([fuenteDelTrabajador(crearAyudas)], { type: "text/javascript" }));

/** Únicos orígenes que pueden pedir un dibujo: el panel y el panel en desarrollo. */
const ORIGENES_PERMITIDOS = new Set([
  "https://panel.megabrisas.com",
  "http://127.0.0.1:5173",
  "http://localhost:5173",
]);
const TIEMPO_MAXIMO_MS = 4000;

const raiz = document.getElementById("raiz");
const tip = document.getElementById("tip");
let origenPadre = null;
let ultimoPedido = null;
let trabajador = null;
let ultimoAncho = 0;

function avisar(mensaje) {
  if (origenPadre) window.parent.postMessage(mensaje, origenPadre);
}

let ultimoAlto = 0;
/** El alto del dibujo (no el de la ventana, que es el del marco): así el panel ajusta el marco al contenido. */
function avisarAlto() {
  const alto = Math.ceil(raiz.getBoundingClientRect().height) + 4;
  if (alto === ultimoAlto) return;
  ultimoAlto = alto;
  avisar({ tipo: "alto", alto });
}

function aplicarTema(tema) {
  const estilo = document.documentElement.style;
  for (const [variable, valor] of [
    ["--texto", tema.texto],
    ["--texto-suave", tema.textoSuave],
    ["--rejilla", tema.rejilla],
    ["--fondo", tema.fondo],
    ["--tip-fondo", tema.tipFondo],
    ["--tip-texto", tema.tipTexto],
  ]) {
    if (typeof valor === "string" && /^#[0-9a-f]{3,8}$/i.test(valor)) estilo.setProperty(variable, valor);
  }
}

function mostrarError(mensaje) {
  raiz.replaceChildren();
  const p = document.createElement("p");
  p.className = "error";
  p.textContent = "No se pudo dibujar este panel.";
  raiz.append(p);
  avisar({ tipo: "error", id: ultimoPedido?.id, mensaje: String(mensaje).slice(0, 300) });
  avisarAlto();
}

function dibujar(pedido) {
  ultimoPedido = pedido;
  trabajador?.terminate();
  const ancho = Math.max(200, Math.floor(raiz.clientWidth || window.innerWidth) - 4);
  ultimoAncho = window.innerWidth;
  aplicarTema(pedido.tema ?? {});

  let actual;
  try {
    actual = new Worker(URL_TRABAJADOR);
  } catch (error) {
    return mostrarError(`No se pudo preparar el dibujo: ${error instanceof Error ? error.message : error}`);
  }
  trabajador = actual;
  const limite = setTimeout(() => {
    actual.terminate();
    if (trabajador === actual) mostrarError("El dibujo tardó demasiado y se cortó.");
  }, TIEMPO_MAXIMO_MS);

  actual.onmessage = (evento) => {
    clearTimeout(limite);
    actual.terminate();
    if (trabajador !== actual) return;
    const { ok, html, mensaje } = evento.data ?? {};
    if (!ok) return mostrarError(mensaje);
    raiz.classList.remove("entrando");
    raiz.replaceChildren(limpiarDibujo(html));
    // Reinicia la animación de entrada.
    void raiz.offsetWidth;
    raiz.classList.add("entrando");
    avisar({ tipo: "listo", id: pedido.id });
    avisarAlto();
  };
  actual.onerror = (evento) => {
    clearTimeout(limite);
    actual.terminate();
    if (trabajador === actual) mostrarError(evento.message || "Error en el código del dibujo.");
  };
  actual.postMessage({
    codigo: String(pedido.codigo ?? ""),
    datos: Array.isArray(pedido.datos) ? pedido.datos : [],
    columnas: Array.isArray(pedido.columnas) ? pedido.columnas : [],
    ancho,
    tema: pedido.tema ?? {},
  });
}

// --- Exportar el dibujo como PNG (lo pide el panel; vuelve como data: URL) ---
const PROPIEDADES_SVG = ["fill", "fill-opacity", "stroke", "stroke-width", "stroke-opacity", "stroke-dasharray", "opacity",
  "font-family", "font-size", "font-weight", "text-anchor", "dominant-baseline", "visibility", "display"];

async function exportarPng(titulo) {
  const svg = raiz.querySelector("svg");
  if (!svg) throw new Error("Este panel no tiene un dibujo para exportar como imagen.");
  const caja = svg.getBoundingClientRect();
  const ancho = Math.max(1, Math.round(caja.width));
  const alto = Math.max(1, Math.round(caja.height));
  const copia = svg.cloneNode(true);
  const originales = [svg, ...svg.querySelectorAll("*")];
  const copias = [copia, ...copia.querySelectorAll("*")];
  originales.forEach((original, i) => {
    const estilo = getComputedStyle(original);
    copias[i].setAttribute("style", PROPIEDADES_SVG.map((p) => `${p}:${estilo.getPropertyValue(p)}`).join(";"));
    copias[i].removeAttribute("class");
  });
  copia.setAttribute("width", String(ancho));
  copia.setAttribute("height", String(alto));
  const imagen = new Image();
  await new Promise((resolver, rechazar) => {
    imagen.onload = resolver;
    imagen.onerror = () => rechazar(new Error("No se pudo preparar la imagen."));
    imagen.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(copia))}`;
  });
  const estiloRaiz = getComputedStyle(document.documentElement);
  const margen = 24;
  const altoTitulo = titulo ? 30 : 0;
  const escala = 2;
  const canvas = document.createElement("canvas");
  canvas.width = (ancho + margen * 2) * escala;
  canvas.height = (alto + altoTitulo + margen * 2) * escala;
  const ctx = canvas.getContext("2d");
  ctx.scale(escala, escala);
  ctx.fillStyle = estiloRaiz.getPropertyValue("--fondo").trim() || "#ffffff";
  ctx.fillRect(0, 0, ancho + margen * 2, alto + altoTitulo + margen * 2);
  if (titulo) {
    ctx.fillStyle = estiloRaiz.getPropertyValue("--texto").trim() || "#1d2733";
    ctx.font = '600 16px system-ui, -apple-system, "Segoe UI", Roboto, sans-serif';
    ctx.textBaseline = "top";
    ctx.fillText(String(titulo).slice(0, 120), margen, margen);
  }
  ctx.drawImage(imagen, margen, margen + altoTitulo, ancho, alto);
  return canvas.toDataURL("image/png");
}

window.addEventListener("message", (evento) => {
  if (evento.source !== window.parent || !ORIGENES_PERMITIDOS.has(evento.origin)) return;
  origenPadre = evento.origin;
  const pedido = evento.data;
  if (pedido?.tipo === "dibujar" && typeof pedido.codigo === "string") dibujar(pedido);
  if (pedido?.tipo === "exportar_png") {
    exportarPng(pedido.titulo)
      .then((datos) => avisar({ tipo: "png", datos }))
      .catch((error) => avisar({ tipo: "png_error", mensaje: error instanceof Error ? error.message : String(error) }));
  }
});

// Al cambiar el ancho se vuelve a dibujar (el código recibe el ancho disponible).
let espera = 0;
window.addEventListener("resize", () => {
  if (!ultimoPedido || Math.abs(window.innerWidth - ultimoAncho) < 8) return;
  clearTimeout(espera);
  espera = setTimeout(() => dibujar(ultimoPedido), 180);
});

new ResizeObserver(() => avisarAlto()).observe(raiz);

// Tooltips: cualquier elemento con data-tip, con el mouse o con el teclado.
function mostrarTip(objetivo, x, y) {
  tip.textContent = objetivo.getAttribute("data-tip");
  tip.hidden = false;
  const caja = tip.getBoundingClientRect();
  const izquierda = Math.min(Math.max(4, x + 12), window.innerWidth - caja.width - 4);
  const arriba = y - caja.height - 10 < 4 ? y + 14 : y - caja.height - 10;
  tip.style.left = `${izquierda}px`;
  tip.style.top = `${arriba}px`;
}
raiz.addEventListener("pointermove", (e) => {
  const objetivo = e.target instanceof Element ? e.target.closest("[data-tip]") : null;
  if (objetivo) mostrarTip(objetivo, e.clientX, e.clientY);
  else tip.hidden = true;
});
raiz.addEventListener("pointerleave", () => (tip.hidden = true));
raiz.addEventListener("focusin", (e) => {
  const objetivo = e.target instanceof Element ? e.target.closest("[data-tip]") : null;
  if (!objetivo) return;
  const caja = objetivo.getBoundingClientRect();
  mostrarTip(objetivo, caja.left + caja.width / 2, caja.top);
});
raiz.addEventListener("focusout", () => (tip.hidden = true));

// Avisa que ya puede recibir (sin datos: cualquier origen puede saberlo).
window.parent.postMessage({ tipo: "preparado" }, "*");
