import type { ContratistaResumen } from "../api";

/** Convierte el texto del gafete a número -- entrada, no regla: si hace
 * falta o no lo decide el núcleo (`requiere_gafete` viene de
 * `prepararIngreso`, y `registrar_ingreso` rechaza con su propio mensaje si
 * falta). `null` = sin gafete; `undefined` = el texto no es un número. */
export function numeroDeGafete(texto: string, requiereGafete: boolean): number | null | undefined {
  const recortado = texto.trim();
  if (!requiereGafete || !recortado) return null;
  const numero = Number.parseInt(recortado, 10);
  return Number.isNaN(numero) ? undefined : numero;
}

export interface AvisoContratista {
  texto: string;
  /** Variable CSS del color del chip (ej. "var(--error)"). */
  color: string;
}

/** Chips de la lista de resultados del buscador: lo que el operador tiene
 * que ver ANTES de elegir a alguien (pedido del usuario 2026-09-23, que
 * además sacó la empresa de esa lista para darles lugar -- sigue en la
 * ficha al elegir). Sólo informativos: si puede o no entrar lo decide el
 * núcleo al elegir (`prepararIngreso`). El aviso de acceso ("ACCESO
 * DENEGADO" / "PRAIND VENCIDO") ya viene resuelto por el núcleo. */
export function avisosContratista(
  contratista: Pick<ContratistaResumen, "tiene_ingreso_activo" | "aviso_acceso">,
): AvisoContratista[] {
  const avisos: AvisoContratista[] = [];
  if (contratista.tiene_ingreso_activo) avisos.push({ texto: "Adentro", color: "var(--acento)" });
  if (contratista.aviso_acceso) avisos.push({ texto: contratista.aviso_acceso, color: "var(--error)" });
  return avisos;
}
