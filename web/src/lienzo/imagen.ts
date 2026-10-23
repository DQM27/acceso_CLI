/**
 * Exportar un gráfico del lienzo como imagen PNG, en el navegador: el gráfico
 * ya es SVG, así que se copia con sus estilos calculados (los colores salen de
 * variables CSS del tema, que fuera de la página no existen), se dibuja en un
 * canvas al doble de resolución con su título y su leyenda, y se descarga.
 */

/** Propiedades que definen cómo se ve un SVG; se copian como estilo en línea. */
const PROPIEDADES = [
  "fill",
  "fill-opacity",
  "stroke",
  "stroke-width",
  "stroke-opacity",
  "stroke-dasharray",
  "stroke-linecap",
  "stroke-linejoin",
  "opacity",
  "font-family",
  "font-size",
  "font-weight",
  "text-anchor",
  "dominant-baseline",
  "visibility",
  "display",
];

/** El SVG como texto independiente de la página: estilos en línea, sin animaciones, con su tamaño en píxeles. */
export function svgIndependiente(svg: SVGSVGElement, ancho: number, alto: number): string {
  const copia = svg.cloneNode(true) as SVGSVGElement;
  const originales = [svg, ...Array.from(svg.querySelectorAll("*"))];
  const copias = [copia, ...Array.from(copia.querySelectorAll("*"))];
  originales.forEach((original, i) => {
    const destino = copias[i];
    if (!destino) return;
    const estilo = getComputedStyle(original);
    const enLinea = PROPIEDADES.map((p) => [p, estilo.getPropertyValue(p)] as const)
      .filter(([, v]) => v !== "")
      .map(([p, v]) => `${p}:${v}`)
      .join(";");
    destino.setAttribute("style", `${enLinea};animation:none;transition:none`);
    destino.removeAttribute("class");
  });
  copia.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  copia.setAttribute("width", String(ancho));
  copia.setAttribute("height", String(alto));
  return new XMLSerializer().serializeToString(copia);
}

export interface OpcionesImagen {
  titulo?: string;
  subtitulo?: string;
  leyenda?: { nombre: string; color: string }[];
  fondo: string;
  texto: string;
  textoSuave: string;
  escala?: number;
}

function cargarImagen(fuente: string): Promise<HTMLImageElement> {
  return new Promise((resolver, rechazar) => {
    const imagen = new Image();
    imagen.onload = () => resolver(imagen);
    imagen.onerror = () => rechazar(new Error("No se pudo preparar la imagen del gráfico."));
    imagen.src = fuente;
  });
}

const FUENTE = 'system-ui, -apple-system, "Segoe UI", Roboto, sans-serif';

/** PNG del gráfico con su título arriba y su leyenda abajo. */
export async function svgAPng(svg: SVGSVGElement, opciones: OpcionesImagen): Promise<Blob> {
  const caja = svg.getBoundingClientRect();
  const ancho = Math.max(1, Math.round(caja.width));
  const alto = Math.max(1, Math.round(caja.height));
  // data: (no blob:) porque la política de seguridad del panel sólo deja imágenes 'self' y data:.
  const imagen = await cargarImagen(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgIndependiente(svg, ancho, alto))}`);

  const escala = opciones.escala ?? 2;
  const margen = 24;
  const altoTitulo = opciones.titulo ? (opciones.subtitulo ? 46 : 30) : 0;
  const leyenda = opciones.leyenda ?? [];
  const lienzo = document.createElement("canvas");
  const ctx = lienzo.getContext("2d");
  if (!ctx) throw new Error("El navegador no deja dibujar la imagen.");

  // Leyenda en renglones que entren en el ancho.
  ctx.font = `12px ${FUENTE}`;
  const renglones: { nombre: string; color: string; x: number }[][] = [[]];
  let x = 0;
  for (const elemento of leyenda) {
    const largo = 18 + ctx.measureText(elemento.nombre).width + 16;
    if (x + largo > ancho && x > 0) {
      renglones.push([]);
      x = 0;
    }
    renglones[renglones.length - 1].push({ ...elemento, x });
    x += largo;
  }
  const altoLeyenda = leyenda.length ? renglones.length * 20 + 8 : 0;

  lienzo.width = (ancho + margen * 2) * escala;
  lienzo.height = (alto + altoTitulo + altoLeyenda + margen * 2) * escala;
  ctx.scale(escala, escala);
  ctx.fillStyle = opciones.fondo;
  ctx.fillRect(0, 0, ancho + margen * 2, alto + altoTitulo + altoLeyenda + margen * 2);

  if (opciones.titulo) {
    ctx.fillStyle = opciones.texto;
    ctx.font = `600 16px ${FUENTE}`;
    ctx.textBaseline = "top";
    ctx.fillText(opciones.titulo, margen, margen);
    if (opciones.subtitulo) {
      ctx.fillStyle = opciones.textoSuave;
      ctx.font = `13px ${FUENTE}`;
      ctx.fillText(opciones.subtitulo, margen, margen + 22);
    }
  }
  ctx.drawImage(imagen, margen, margen + altoTitulo, ancho, alto);

  ctx.font = `12px ${FUENTE}`;
  ctx.textBaseline = "middle";
  renglones.forEach((renglon, r) => {
    const y = margen + altoTitulo + alto + 8 + r * 20 + 10;
    for (const e of renglon) {
      ctx.fillStyle = e.color;
      ctx.fillRect(margen + e.x, y - 5, 10, 10);
      ctx.fillStyle = opciones.textoSuave;
      ctx.fillText(e.nombre, margen + e.x + 16, y);
    }
  });

  return new Promise((resolver, rechazar) =>
    lienzo.toBlob((b) => (b ? resolver(b) : rechazar(new Error("No se pudo generar el PNG."))), "image/png"),
  );
}

/**
 * Lo único que se acepta del sandbox como archivo: un PNG en data: URL (hasta
 * ~15 MB), comprobado por su firma de bytes. Cualquier otra cosa, `null`.
 */
export function pngDesdeDataUrl(datos: unknown): Blob | null {
  const prefijo = "data:image/png;base64,";
  if (typeof datos !== "string" || !datos.startsWith(prefijo) || datos.length > 20_000_000) return null;
  let binario: string;
  try {
    binario = atob(datos.slice(prefijo.length));
  } catch {
    return null;
  }
  const bytes = Uint8Array.from(binario, (c) => c.charCodeAt(0));
  const firma = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (!firma.every((b, i) => bytes[i] === b)) return null;
  return new Blob([bytes], { type: "image/png" });
}

/** Nombre de archivo a partir de un título: "Ingresos por empresa" → "ingresos-por-empresa.png". */
export function nombreDeArchivo(titulo: string, extension: string): string {
  const base = titulo
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 60);
  return `${base || "lienzo"}.${extension}`;
}
