// Buscador local del manual: sin internet y sin IA. Busca palabras sueltas
// sin importar tildes ni mayúsculas, y premia el mensaje pegado entero (lo
// que más se busca es "qué significa este mensaje").
import type { Bloque } from "./markdown.ts";
import { leerEnLinea, textoPlano } from "./markdown.ts";
import type { Proceso } from "./manual.ts";

export interface Resultado {
  proceso: Proceso;
  puntaje: number;
  /** El trozo donde más coincide, para mostrar debajo del título. */
  fragmento: string;
  /** Título de la sección de ese trozo y su ancla, para ir directo. */
  seccion: string | null;
  ancla: string | null;
}

interface Trozo {
  texto: string;
  normalizado: string;
  seccion: string | null;
  ancla: string | null;
}

/** Minúsculas, sin tildes ni signos, con un solo espacio entre palabras. */
export function normalizar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
}

// Palabras que no ayudan a encontrar nada ("cómo registro la salida de…").
const VACIAS = new Set(
  (
    "a al como con cual cuando de del donde el en es esta este hacer hago la las lo los mi mis " +
    "no o para pero por puedo que qué se si sin su sus un una uno unos y ya"
  ).split(" ").map(normalizar),
);

function plano(texto: string): string {
  return textoPlano(leerEnLinea(texto));
}

/** El proceso en trozos (párrafos, elementos de lista, filas de tabla), cada
 * uno con la sección donde está. */
function trozos(proceso: Proceso): Trozo[] {
  const lista: Trozo[] = [];
  let seccion: string | null = null;
  let anclaSeccion: string | null = null;
  const agregar = (texto: string) => {
    if (texto) lista.push({ texto, normalizado: normalizar(texto), seccion, ancla: anclaSeccion });
  };
  const recorrer = (bloques: Bloque[]) => {
    for (const bloque of bloques) {
      if (bloque.tipo === "titulo") {
        if (bloque.nivel > 1) {
          seccion = plano(bloque.texto);
          anclaSeccion = bloque.ancla;
        }
      } else if (bloque.tipo === "parrafo") {
        agregar(plano(bloque.texto));
      } else if (bloque.tipo === "lista") {
        for (const elemento of bloque.elementos) {
          agregar(plano(elemento.texto));
          recorrer(elemento.hijos);
        }
      } else {
        for (const fila of bloque.filas) agregar(fila.map(plano).join(" — "));
      }
    }
  };
  recorrer(proceso.bloques);
  return lista;
}

const cache = new WeakMap<Proceso, { titulo: string; trozos: Trozo[] }>();

function indexado(proceso: Proceso) {
  let datos = cache.get(proceso);
  if (!datos) {
    datos = { titulo: normalizar(proceso.titulo), trozos: trozos(proceso) };
    cache.set(proceso, datos);
  }
  return datos;
}

const LARGO_FRAGMENTO = 180;

function recortar(texto: string, palabras: string[]): string {
  if (texto.length <= LARGO_FRAGMENTO) return texto;
  // Arranca un poco antes de la primera palabra encontrada.
  const normal = normalizar(texto);
  const posiciones = palabras.map((p) => normal.indexOf(p)).filter((p) => p >= 0);
  // `normalizar` puede acortar el texto (signos); la posición es aproximada.
  const proporcion = normal.length > 0 ? texto.length / normal.length : 1;
  const inicio = Math.max(0, Math.round((posiciones.length ? Math.min(...posiciones) : 0) * proporcion) - 40);
  const desde = inicio === 0 ? 0 : texto.indexOf(" ", inicio) + 1;
  const corte = texto.slice(desde, desde + LARGO_FRAGMENTO);
  const fin = desde + LARGO_FRAGMENTO < texto.length ? corte.slice(0, corte.lastIndexOf(" ")) : corte;
  return `${desde > 0 ? "…" : ""}${fin}${desde + LARGO_FRAGMENTO < texto.length ? "…" : ""}`;
}

/**
 * Procesos que tienen **todas** las palabras buscadas (en el título o en el
 * texto), de más a menos relevante. Si ninguno las tiene todas ("marco" no
 * está, el manual dice "Marque"), los que tienen más, con un mínimo de la
 * mitad. Una palabra coincide también como comienzo o parte de otra
 * ("gafet" encuentra "gafetes").
 */
export function buscarEnManual(procesos: Iterable<Proceso>, consulta: string, limite = 20): Resultado[] {
  const frase = normalizar(consulta);
  if (frase.length < 2) return [];
  const todas = frase.split(" ");
  const utiles = todas.filter((p) => !VACIAS.has(p) && p.length > 1);
  const palabras = utiles.length > 0 ? utiles : todas;

  const candidatos: { resultado: Resultado; coincidencias: number }[] = [];
  for (const proceso of procesos) {
    const { titulo, trozos } = indexado(proceso);
    const todoElTexto = `${titulo} ${trozos.map((t) => t.normalizado).join(" ")}`;
    const coincidencias = palabras.filter((p) => todoElTexto.includes(p)).length;
    if (coincidencias === 0) continue;

    let puntaje = 0;
    for (const p of palabras) {
      if (titulo.includes(p)) puntaje += 10;
    }
    if (titulo.includes(frase)) puntaje += 15;

    let mejor: Trozo | null = null;
    let mejorPuntaje = 0;
    for (const trozo of trozos) {
      let p = 0;
      for (const palabra of palabras) {
        if (trozo.normalizado.includes(palabra)) p += 2;
        if (trozo.seccion && normalizar(trozo.seccion).includes(palabra)) p += 1;
      }
      // El mensaje o la frase entera, tal como se ve en pantalla.
      if (palabras.length > 1 && trozo.normalizado.includes(frase)) p += 6 * palabras.length;
      if (p > mejorPuntaje) {
        mejor = trozo;
        mejorPuntaje = p;
      }
    }
    puntaje += mejorPuntaje;

    candidatos.push({
      coincidencias,
      resultado: {
        proceso,
        puntaje,
        fragmento: mejor ? recortar(mejor.texto, palabras) : "",
        seccion: mejor?.seccion ?? null,
        ancla: mejor?.ancla ?? null,
      },
    });
  }

  const maximo = Math.max(0, ...candidatos.map((c) => c.coincidencias));
  if (maximo < Math.ceil(palabras.length / 2)) return [];
  return candidatos
    .filter((c) => c.coincidencias === maximo)
    .map((c) => c.resultado)
    .sort((a, b) => b.puntaje - a.puntaje || a.proceso.titulo.localeCompare(b.proceso.titulo, "es"))
    .slice(0, limite);
}
