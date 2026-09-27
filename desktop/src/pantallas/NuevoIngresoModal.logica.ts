import type { ContratistaResumen } from "../api";
import { fechaYMD } from "../tiempo";

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
 * núcleo al elegir (`prepararIngreso`), no estos chips. `hoy` en
 * "AAAA-MM-DD", inyectable para el test. */
export function avisosContratista(
  contratista: Pick<
    ContratistaResumen,
    "tiene_ingreso_activo" | "tiene_acceso" | "fecha_vencimiento_praind"
  >,
  hoy: string = fechaYMD(new Date()),
): AvisoContratista[] {
  const avisos: AvisoContratista[] = [];
  if (contratista.tiene_ingreso_activo) avisos.push({ texto: "Adentro", color: "var(--acento)" });
  if (!contratista.tiene_acceso) avisos.push({ texto: "Sin acceso", color: "var(--error)" });
  if (contratista.fecha_vencimiento_praind && contratista.fecha_vencimiento_praind < hoy) {
    avisos.push({ texto: "PRAIND vencido", color: "var(--error)" });
  }
  return avisos;
}
