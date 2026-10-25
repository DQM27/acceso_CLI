// El manual de uso armado a partir de los archivos de `docs/manual/`: un
// proceso por archivo y el índice de cada app tal como lo ordena el README.
import { type Bloque, leerEnLinea, leerMarkdown, textoPlano } from "./markdown.ts";

export type App = "escritorio" | "movil" | "panel";

export const APPS: { id: App; nombre: string }[] = [
  { id: "escritorio", nombre: "Escritorio" },
  { id: "movil", nombre: "Móvil" },
  { id: "panel", nombre: "Panel web" },
];

export interface Proceso {
  /** Ruta dentro de `docs/manual/`, p. ej. `escritorio/registrar-salida.md`. */
  ruta: string;
  app: App;
  titulo: string;
  bloques: Bloque[];
}

export interface GrupoIndice {
  /** Vacío cuando el índice de esa app no tiene subtítulos. */
  titulo: string;
  procesos: Proceso[];
}

export interface Manual {
  procesos: Map<string, Proceso>;
  indice: Record<App, GrupoIndice[]>;
}

function appDeRuta(ruta: string): App | null {
  const carpeta = ruta.split("/")[0];
  return carpeta === "escritorio" || carpeta === "movil" || carpeta === "panel" ? carpeta : null;
}

/** La app de un título del README: "Índice — Escritorio (puesto de control)". */
function appDeTitulo(titulo: string): App | null {
  const t = titulo.toLowerCase();
  if (!t.startsWith("índice")) return null;
  if (t.includes("escritorio")) return "escritorio";
  if (t.includes("móvil")) return "movil";
  if (t.includes("panel")) return "panel";
  return null;
}

/** Destinos de los enlaces de un texto en markdown. */
function enlacesDe(texto: string): string[] {
  const destinos: string[] = [];
  const recorrer = (partes: ReturnType<typeof leerEnLinea>) => {
    for (const parte of partes) {
      if (parte.tipo === "enlace") destinos.push(parte.destino);
      if (parte.tipo === "negrita" || parte.tipo === "enlace") recorrer(parte.hijos);
    }
  };
  recorrer(leerEnLinea(texto));
  return destinos;
}

/**
 * Arma el manual. `archivos` va de la ruta dentro de `docs/manual/`
 * (`README.md`, `escritorio/registrar-salida.md`…) a su contenido.
 *
 * El índice sale de los títulos "Índice — …" del README, en su orden; un
 * proceso que exista pero no figure ahí queda al final, en "Otros".
 */
export function armarManual(archivos: Record<string, string>): Manual {
  const procesos = new Map<string, Proceso>();
  for (const [ruta, contenido] of Object.entries(archivos)) {
    const app = appDeRuta(ruta);
    if (!app) continue;
    const bloques = leerMarkdown(contenido);
    const primero = bloques[0];
    const titulo = primero?.tipo === "titulo" ? textoPlano(leerEnLinea(primero.texto)) : ruta;
    procesos.set(ruta, { ruta, app, titulo, bloques });
  }

  const indice: Record<App, GrupoIndice[]> = { escritorio: [], movil: [], panel: [] };
  const enIndice = new Set<string>();
  let app: App | null = null;
  let grupo: GrupoIndice | null = null;
  for (const bloque of leerMarkdown(archivos["README.md"] ?? "")) {
    if (bloque.tipo === "titulo" && bloque.nivel <= 2) {
      app = appDeTitulo(bloque.texto);
      grupo = null;
    } else if (app && bloque.tipo === "titulo") {
      grupo = { titulo: textoPlano(leerEnLinea(bloque.texto)), procesos: [] };
      indice[app].push(grupo);
    } else if (app && bloque.tipo === "lista") {
      for (const elemento of bloque.elementos) {
        for (const destino of enlacesDe(elemento.texto)) {
          const proceso = procesos.get(destino.split("#")[0]);
          if (!proceso || enIndice.has(proceso.ruta)) continue;
          if (!grupo) {
            grupo = { titulo: "", procesos: [] };
            indice[app].push(grupo);
          }
          grupo.procesos.push(proceso);
          enIndice.add(proceso.ruta);
        }
      }
    }
  }

  for (const { id } of APPS) {
    // "Fuera del manual", "Mejoras de pantalla…": grupos sin procesos.
    indice[id] = indice[id].filter((g) => g.procesos.length > 0);
    const sueltos = [...procesos.values()].filter((p) => p.app === id && !enIndice.has(p.ruta));
    if (sueltos.length > 0) {
      sueltos.sort((a, b) => a.titulo.localeCompare(b.titulo, "es"));
      indice[id].push({ titulo: "Otros", procesos: sueltos });
    }
  }

  return { procesos, indice };
}

export interface DestinoEnlace {
  ruta: string;
  ancla: string | null;
}

/**
 * A dónde lleva un enlace escrito en el proceso `desde`. `null` para
 * direcciones de afuera (http…) o que no son del manual.
 */
export function resolverEnlace(desde: string, destino: string): DestinoEnlace | null {
  if (/^[a-z]+:/i.test(destino)) return null;
  const [camino, anclaCruda] = destino.split("#", 2);
  const ancla = anclaCruda ? decodeURIComponent(anclaCruda) : null;
  if (!camino) return { ruta: desde, ancla };
  if (!camino.endsWith(".md")) return null;
  const partes = desde.split("/").slice(0, -1);
  for (const parte of camino.split("/")) {
    if (parte === "..") partes.pop();
    else if (parte !== ".") partes.push(parte);
  }
  return { ruta: partes.join("/"), ancla };
}
