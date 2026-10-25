// Lector del markdown del manual de uso (`docs/manual/`). Sólo entiende lo que
// el manual usa (ver las reglas de redacción en `docs/manual/README.md`):
// títulos, párrafos, listas (numeradas o no, anidadas), tablas, negrita,
// código y enlaces. Sin dependencias: lo usan el escritorio y el panel para
// el manual navegable sin internet, y lo podrá usar la Edge Function de la
// ayuda con IA.

export type EnLinea =
  | { tipo: "texto"; texto: string }
  | { tipo: "negrita"; hijos: EnLinea[] }
  | { tipo: "codigo"; texto: string }
  | { tipo: "enlace"; destino: string; hijos: EnLinea[] };

export interface ElementoLista {
  /** Primer párrafo del elemento, con su markdown en línea. */
  texto: string;
  /** Lo que viene debajo con sangría (sublistas, párrafos). */
  hijos: Bloque[];
}

export type Bloque =
  | { tipo: "titulo"; nivel: number; texto: string; ancla: string }
  | { tipo: "parrafo"; texto: string }
  | { tipo: "lista"; ordenada: boolean; inicio: number; elementos: ElementoLista[] }
  | { tipo: "tabla"; encabezado: string[]; filas: string[][] };

const TITULO = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const MARCA_LISTA = /^([-*]|\d+\.)\s+/;
const SEPARADOR_TABLA = /^\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$/;

function sangria(linea: string): number {
  return linea.length - linea.trimStart().length;
}

function enBlanco(linea: string): boolean {
  return linea.trim() === "";
}

function empiezaBloque(linea: string): boolean {
  return TITULO.test(linea) || MARCA_LISTA.test(linea) || linea.startsWith("|");
}

/** Ancla de un título igual a la que genera GitHub, para que los enlaces
 * `proceso.md#titulo` del manual funcionen en los dos lados. */
export function ancla(texto: string): string {
  return textoPlano(leerEnLinea(texto))
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    .replace(/\s/g, "-");
}

function celdas(linea: string): string[] {
  let fila = linea.trim();
  if (fila.startsWith("|")) fila = fila.slice(1);
  if (fila.endsWith("|") && !fila.endsWith("\\|")) fila = fila.slice(0, -1);
  return fila.split(/(?<!\\)\|/).map((celda) => celda.trim().replace(/\\\|/g, "|"));
}

/** Convierte el markdown en bloques. Las líneas cortadas de un párrafo se
 * unen con un espacio. */
export function leerMarkdown(markdown: string): Bloque[] {
  const usadas = new Map<string, number>();
  const bloques = leerBloques(markdown.replace(/\r\n?/g, "\n").split("\n"));
  // Títulos repetidos: GitHub agrega -1, -2… al ancla.
  const numerar = (lista: Bloque[]) => {
    for (const bloque of lista) {
      if (bloque.tipo === "titulo") {
        const veces = usadas.get(bloque.ancla) ?? 0;
        usadas.set(bloque.ancla, veces + 1);
        if (veces > 0) bloque.ancla = `${bloque.ancla}-${veces}`;
      } else if (bloque.tipo === "lista") {
        for (const elemento of bloque.elementos) numerar(elemento.hijos);
      }
    }
  };
  numerar(bloques);
  return bloques;
}

function leerBloques(lineas: string[]): Bloque[] {
  const bloques: Bloque[] = [];
  let i = 0;
  while (i < lineas.length) {
    const linea = lineas[i];
    if (enBlanco(linea)) {
      i++;
      continue;
    }

    const titulo = TITULO.exec(linea);
    if (titulo) {
      bloques.push({ tipo: "titulo", nivel: titulo[1].length, texto: titulo[2], ancla: ancla(titulo[2]) });
      i++;
      continue;
    }

    if (linea.startsWith("|")) {
      const filas: string[][] = [];
      while (i < lineas.length && lineas[i].trimStart().startsWith("|")) {
        if (!SEPARADOR_TABLA.test(lineas[i])) filas.push(celdas(lineas[i]));
        i++;
      }
      const [encabezado = [], ...resto] = filas;
      bloques.push({ tipo: "tabla", encabezado, filas: resto });
      continue;
    }

    const marca = MARCA_LISTA.exec(linea);
    if (marca) {
      const ordenada = marca[1] !== "-" && marca[1] !== "*";
      const lista: Bloque & { tipo: "lista" } = {
        tipo: "lista",
        ordenada,
        inicio: ordenada ? Number.parseInt(marca[1], 10) : 1,
        elementos: [],
      };
      while (i < lineas.length) {
        const actual = MARCA_LISTA.exec(lineas[i]);
        if (!actual || (actual[1] !== "-" && actual[1] !== "*") === !ordenada) break;
        const anchoMarca = actual[0].length;
        const cuerpo = [lineas[i].slice(anchoMarca)];
        i++;
        while (i < lineas.length) {
          const siguiente = lineas[i];
          if (enBlanco(siguiente)) {
            // Una línea en blanco sigue dentro del elemento sólo si lo que
            // viene después tiene sangría.
            let j = i + 1;
            while (j < lineas.length && enBlanco(lineas[j])) j++;
            if (j < lineas.length && sangria(lineas[j]) > 0) {
              cuerpo.push("");
              i++;
              continue;
            }
            break;
          }
          if (sangria(siguiente) > 0) {
            cuerpo.push(siguiente.slice(Math.min(sangria(siguiente), anchoMarca)));
            i++;
            continue;
          }
          // Sin sangría: otra marca cierra el elemento; texto suelto
          // pegado a la línea anterior la continúa.
          if (empiezaBloque(siguiente) || enBlanco(cuerpo[cuerpo.length - 1])) break;
          cuerpo.push(siguiente);
          i++;
        }
        lista.elementos.push(elementoLista(cuerpo));
        // Una línea en blanco entre elementos de la misma lista no la corta.
        let j = i;
        while (j < lineas.length && enBlanco(lineas[j])) j++;
        if (j > i && j < lineas.length && MARCA_LISTA.test(lineas[j])) i = j;
      }
      bloques.push(lista);
      continue;
    }

    const parrafo: string[] = [];
    while (i < lineas.length && !enBlanco(lineas[i]) && (parrafo.length === 0 || !empiezaBloque(lineas[i]))) {
      parrafo.push(lineas[i].trim());
      i++;
    }
    bloques.push({ tipo: "parrafo", texto: parrafo.join(" ") });
  }
  return bloques;
}

function elementoLista(cuerpo: string[]): ElementoLista {
  let fin = 1;
  while (fin < cuerpo.length && !enBlanco(cuerpo[fin]) && !empiezaBloque(cuerpo[fin].trimStart())) fin++;
  return {
    texto: cuerpo.slice(0, fin).map((linea) => linea.trim()).join(" "),
    hijos: leerBloques(cuerpo.slice(fin)),
  };
}

const ENLACE = /^\[([^\]]+)\]\(([^)\s]+)\)/;

/** Markdown en línea: `**negrita**`, `` `código` `` y `[texto](destino)`. */
export function leerEnLinea(texto: string): EnLinea[] {
  const partes: EnLinea[] = [];
  let suelto = "";
  const cerrarSuelto = () => {
    if (suelto) partes.push({ tipo: "texto", texto: suelto });
    suelto = "";
  };
  let i = 0;
  while (i < texto.length) {
    if (texto.startsWith("**", i)) {
      const fin = texto.indexOf("**", i + 2);
      if (fin > i + 2) {
        cerrarSuelto();
        partes.push({ tipo: "negrita", hijos: leerEnLinea(texto.slice(i + 2, fin)) });
        i = fin + 2;
        continue;
      }
    }
    if (texto[i] === "`") {
      const fin = texto.indexOf("`", i + 1);
      if (fin > i + 1) {
        cerrarSuelto();
        partes.push({ tipo: "codigo", texto: texto.slice(i + 1, fin) });
        i = fin + 1;
        continue;
      }
    }
    if (texto[i] === "[") {
      const enlace = ENLACE.exec(texto.slice(i));
      if (enlace) {
        cerrarSuelto();
        partes.push({ tipo: "enlace", destino: enlace[2], hijos: leerEnLinea(enlace[1]) });
        i += enlace[0].length;
        continue;
      }
    }
    if (texto[i] === "\\" && i + 1 < texto.length) {
      suelto += texto[i + 1];
      i += 2;
      continue;
    }
    suelto += texto[i];
    i++;
  }
  cerrarSuelto();
  return partes;
}

/** El texto tal como se lee, sin marcas. */
export function textoPlano(partes: EnLinea[]): string {
  return partes
    .map((parte) => (parte.tipo === "texto" || parte.tipo === "codigo" ? parte.texto : textoPlano(parte.hijos)))
    .join("");
}
