// Limpia el dibujo que devuelve el código del asistente antes de mostrarlo.
// Lista blanca: sólo etiquetas de dibujo y de texto, sin scripts, sin eventos,
// sin enlaces ni nada que cargue o navegue a otro lado. La política de seguridad
// del sandbox ya bloquea casi todo esto; esto es la segunda capa.

const PERMITIDAS = new Set(
  [
    // SVG
    "svg", "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "text", "tspan", "title", "desc",
    "defs", "lineargradient", "radialgradient", "stop", "clippath", "mask", "pattern", "use", "marker", "symbol",
    // HTML de texto y tablas
    "div", "span", "p", "b", "strong", "i", "em", "small", "br", "hr", "ul", "ol", "li", "table", "thead", "tbody",
    "tfoot", "tr", "th", "td", "caption", "h1", "h2", "h3", "h4", "h5", "h6", "section", "header", "footer",
    "figure", "figcaption", "abbr", "time", "mark", "sup", "sub",
  ],
);

/** Se borran con todo su contenido. El resto de lo no permitido se quita dejando su contenido. */
const DESCARTAR = new Set(
  [
    "script", "style", "iframe", "frame", "frameset", "object", "embed", "link", "meta", "base", "form", "input",
    "button", "textarea", "select", "option", "foreignobject", "animate", "animatemotion", "animatetransform", "set",
    "image", "img", "audio", "video", "source", "track", "canvas", "template", "noscript", "portal", "dialog",
  ],
);

/** Atributos que pueden llevar una dirección: sólo se aceptan referencias internas ("#id"). */
const CON_DIRECCION = new Set(["href", "xlink:href", "src", "srcset", "action", "formaction", "background", "poster", "ping"]);

function atributoSeguro(nombre, valor) {
  const n = nombre.toLowerCase();
  if (n.startsWith("on")) return false;
  const v = valor.replace(/[\s\u0000-\u001f]/g, "").toLowerCase();
  if (v.includes("javascript:") || v.includes("vbscript:") || v.includes("data:text/html")) return false;
  if (CON_DIRECCION.has(n)) return valor.trim().startsWith("#");
  // url(...) sólo hacia algo del mismo dibujo (degradados, recortes): url(#id).
  for (const m of v.matchAll(/url\(([^)]*)\)/g)) {
    if (!m[1].replace(/['"]/g, "").startsWith("#")) return false;
  }
  if (n === "style" && (v.includes("expression(") || v.includes("@import") || v.includes("behavior:"))) return false;
  return true;
}

function limpiarNodo(nodo) {
  for (const hijo of [...nodo.childNodes]) {
    if (hijo.nodeType === 8) {
      hijo.remove();
      continue;
    }
    if (hijo.nodeType !== 1) continue;
    const nombre = hijo.localName.toLowerCase();
    if (DESCARTAR.has(nombre)) {
      hijo.remove();
      continue;
    }
    if (!PERMITIDAS.has(nombre)) {
      limpiarNodo(hijo);
      hijo.replaceWith(...hijo.childNodes);
      continue;
    }
    for (const atributo of [...hijo.attributes]) {
      if (!atributoSeguro(atributo.name, atributo.value)) hijo.removeAttribute(atributo.name);
    }
    limpiarNodo(hijo);
  }
}

/** Devuelve un fragmento ya limpio, listo para insertar. */
export function limpiarDibujo(html, documento = globalThis.document) {
  const parseado = new DOMParser().parseFromString(`<!doctype html><body><div>${html}</div></body>`, "text/html");
  const raiz = parseado.body.firstElementChild;
  limpiarNodo(raiz);
  const fragmento = documento.createDocumentFragment();
  for (const hijo of [...raiz.childNodes]) fragmento.append(documento.importNode(hijo, true));
  return fragmento;
}
