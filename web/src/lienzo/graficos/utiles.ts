import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";

/**
 * Funciones y hooks que comparten los gráficos del lienzo: medida del
 * contenedor, escala del eje, formato de números, navegación con teclado y
 * animaciones. Los componentes (tooltip, leyenda, marco) están en `comun.tsx`.
 */

/** Profundizar desde un gráfico: qué marcas aceptan clic y qué hacer con ellas. */
export interface Seleccion {
  puede: (clave: string) => boolean;
  alSeleccionar: (clave: string) => void;
}

/** `useId()` sin caracteres que rompan un `url(#…)` de SVG. */
export function idSvg(id: string): string {
  return `lienzo-${id.replace(/[^a-zA-Z0-9_-]/g, "")}`;
}

/** Firma corta de los datos: cambia cuando cambian, y con ella se repite la animación de entrada. */
export function firmaDatos(datos: unknown): string {
  const texto = JSON.stringify(datos);
  let h = 0;
  for (let i = 0; i < texto.length; i++) h = (Math.imul(31, h) + texto.charCodeAt(i)) | 0;
  return String(h);
}

/** Ancho real del contenedor, para dibujar a escala 1:1 (el texto no se deforma). */
export function useAncho<T extends HTMLElement>(porOmision = 640) {
  const ref = useRef<T>(null);
  const [ancho, setAncho] = useState(porOmision);
  useLayoutEffect(() => {
    const nodo = ref.current;
    if (!nodo || typeof ResizeObserver === "undefined") return;
    const observador = new ResizeObserver(([entrada]) => {
      const medido = Math.round(entrada.contentRect.width);
      if (medido > 0) setAncho(medido);
    });
    observador.observe(nodo);
    return () => observador.disconnect();
  }, []);
  return { ref, ancho };
}

const COMPACTO = new Intl.NumberFormat("es-CR", { notation: "compact", maximumFractionDigits: 1 });
const ENTERO = new Intl.NumberFormat("es-CR");

/** Número con separador local; con `compacto`, desde 10 000 abreviado ("12 mil"). */
export function formatoNumero(valor: number, compacto = false): string {
  return compacto && Math.abs(valor) >= 10_000 ? COMPACTO.format(valor) : ENTERO.format(valor);
}

/** Marcas "redondas" del eje de valores (0, 20, 40…) y el techo del eje. */
export function marcasEje(maximo: number, cuantas = 4): { marcas: number[]; techo: number } {
  if (!(maximo > 0)) return { marcas: [0, 1], techo: 1 };
  const bruto = maximo / cuantas;
  const potencia = 10 ** Math.floor(Math.log10(bruto));
  const paso = [1, 2, 2.5, 5, 10].map((m) => m * potencia).find((p) => p >= bruto) ?? 10 * potencia;
  // Con conteos, un paso fraccionario no tiene sentido.
  const pasoEntero = Math.max(1, Math.round(paso));
  const techo = Math.ceil(maximo / pasoEntero) * pasoEntero;
  const marcas: number[] = [];
  for (let v = 0; v <= techo + 1e-9; v += pasoEntero) marcas.push(v);
  return { marcas, techo };
}

/** Cada cuántas etiquetas del eje X mostrar una, para que no se pisen. */
export function pasoEtiquetas(cantidad: number, espacio: number, anchoEtiqueta: number): number {
  if (cantidad <= 1) return 1;
  return Math.max(1, Math.ceil((cantidad * anchoEtiqueta) / Math.max(espacio, 1)));
}

/** Recorta un texto largo para el eje; el completo queda en el tooltip y en la tabla. */
export function recortar(texto: string, maximo: number): string {
  return texto.length > maximo ? `${texto.slice(0, Math.max(1, maximo - 1))}…` : texto;
}

/**
 * Índice resaltado (por puntero o por teclado). Flechas para moverse, Inicio y
 * Fin para los extremos, Escape para soltar.
 */
export function useIndiceActivo(cantidad: number, alActivar?: (indice: number) => void) {
  const [indice, setIndice] = useState<number | null>(null);
  const alPresionar = (e: KeyboardEvent) => {
    if (cantidad === 0) return;
    if (e.key === "Escape") {
      setIndice(null);
      return;
    }
    // Enter o espacio: lo mismo que un clic sobre la marca resaltada.
    if ((e.key === "Enter" || e.key === " ") && indice !== null && alActivar) {
      e.preventDefault();
      alActivar(indice);
      return;
    }
    const actual = indice ?? -1;
    const destino =
      e.key === "ArrowRight" || e.key === "ArrowDown"
        ? Math.min(cantidad - 1, actual + 1)
        : e.key === "ArrowLeft" || e.key === "ArrowUp"
          ? Math.max(0, actual - 1)
          : e.key === "Home"
            ? 0
            : e.key === "End"
              ? cantidad - 1
              : undefined;
    if (destino === undefined) return;
    e.preventDefault();
    setIndice(destino);
  };
  return {
    indice,
    alPuntero: setIndice,
    propsTeclado: {
      tabIndex: 0,
      onKeyDown: alPresionar,
      onFocus: () => setIndice((i) => i ?? 0),
      onBlur: () => setIndice(null),
    },
  };
}

function prefiereMenosMovimiento(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
    : true;
}

/** Número que sube desde 0 hasta su valor (y de un valor al siguiente) en `duracion` ms. */
export function useNumeroAnimado(destino: number, duracion = 650): number {
  const [actual, setActual] = useState(() => (prefiereMenosMovimiento() ? destino : 0));
  const desde = useRef(actual);
  useEffect(() => {
    if (prefiereMenosMovimiento() || typeof requestAnimationFrame === "undefined") {
      desde.current = destino;
      // Sin animación: salta directo, en el siguiente microtask (evita setState síncrono en el efecto).
      void Promise.resolve().then(() => setActual(destino));
      return;
    }
    const inicio = desde.current;
    const t0 = performance.now();
    let cuadro = 0;
    const paso = (ahora: number) => {
      const t = Math.min(1, (ahora - t0) / duracion);
      const suave = 1 - (1 - t) ** 3;
      const valor = inicio + (destino - inicio) * suave;
      desde.current = valor;
      setActual(valor);
      if (t < 1) cuadro = requestAnimationFrame(paso);
    };
    cuadro = requestAnimationFrame(paso);
    return () => cancelAnimationFrame(cuadro);
  }, [destino, duracion]);
  return actual;
}

/**
 * Valores que pasan suavemente del dibujo anterior al nuevo cuando cambian los
 * datos (un filtro, una edición del lienzo): cada valor se reconoce por su
 * clave, así una barra que sigue estando se mueve, una nueva crece desde 0 y
 * una que se fue desaparece. La primera vez no interpola: de eso se encarga la
 * animación de entrada.
 */
export function useInterpolado(claves: string[], valores: number[], duracion = 550): number[] {
  const firma = JSON.stringify([claves, valores]);
  const [mostrado, setMostrado] = useState<{ firma: string; valores: number[] }>({ firma, valores });
  const actual = useRef(new Map(claves.map((c, i) => [c, valores[i]])));

  useEffect(() => {
    const [nuevasClaves, destino] = JSON.parse(firma) as [string[], number[]];
    const desde = nuevasClaves.map((c) => actual.current.get(c) ?? 0);
    const fijar = (lista: number[]) => {
      actual.current = new Map(nuevasClaves.map((c, i) => [c, lista[i]]));
      setMostrado({ firma, valores: lista });
    };
    if (prefiereMenosMovimiento() || typeof requestAnimationFrame === "undefined") {
      void Promise.resolve().then(() => fijar(destino));
      return;
    }
    const t0 = performance.now();
    let cuadro = 0;
    const paso = (ahora: number) => {
      const t = Math.min(1, (ahora - t0) / duracion);
      const suave = t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2;
      fijar(destino.map((v, i) => desde[i] + (v - desde[i]) * suave));
      if (t < 1) cuadro = requestAnimationFrame(paso);
    };
    cuadro = requestAnimationFrame(paso);
    return () => cancelAnimationFrame(cuadro);
  }, [firma, duracion]);

  // Mientras la animación no arrancó (o si cambió la cantidad), se dibuja el destino.
  return mostrado.valores.length === valores.length ? mostrado.valores : valores;
}

/** Retraso de la animación de entrada de la marca `i`, escalonado y con tope. */
export function retraso(i: number, cantidad: number, total = 420): string {
  return `${Math.round((Math.min(i, 40) * total) / Math.max(cantidad, 12))}ms`;
}
