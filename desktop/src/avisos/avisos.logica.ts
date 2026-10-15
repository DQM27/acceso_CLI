/** Qué tipo de aviso es: decide su color, su ícono, su título por defecto y
 * cuánto se queda en pantalla. Es la única lista de tipos de la app. */
export type TipoAviso = "exito" | "info" | "advertencia" | "error";

/** Cuántos avisos se ven a la vez: al llegar uno más, se va el más viejo. */
export const MAXIMO_VISIBLES = 4;

/** Duración de un aviso que no se va solo (ver `OpcionesAviso.persistente`). */
export const DURACION_PERSISTENTE_MS = Infinity;

interface PoliticaDeTipo {
  titulo: string;
  duracionMs: number;
}

/** Título y duración de cada tipo. Un error o una advertencia se leen con más
 * calma que una confirmación. Esto es presentación: quién decide QUÉ tipo de
 * aviso corresponde a cada hecho es quien llama, nunca este módulo. */
const POLITICA: Record<TipoAviso, PoliticaDeTipo> = {
  exito: { titulo: "Listo", duracionMs: 4500 },
  info: { titulo: "Aviso", duracionMs: 4500 },
  advertencia: { titulo: "Atención", duracionMs: 8000 },
  error: { titulo: "No se pudo", duracionMs: 8000 },
};

export function tituloPorDefecto(tipo: TipoAviso): string {
  return POLITICA[tipo].titulo;
}

export function duracionDe(tipo: TipoAviso, persistente: boolean): number {
  return persistente ? DURACION_PERSISTENTE_MS : POLITICA[tipo].duracionMs;
}

/** El texto de un error tal como llega de los comandos de Tauri (casi
 * siempre un `string` ya redactado por el núcleo). No lo interpreta ni lo
 * traduce: el mensaje para el operador se decide en el núcleo (Rust), no acá. */
export function textoDeError(error: unknown): string {
  return typeof error === "string" ? error : String(error);
}
