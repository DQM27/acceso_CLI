import type { ContratistaResumen } from "../api";

/** El gafete a mandar al núcleo: `null` = sin gafete (S/G). Si el
 * contratista requiere gafete (`requiere_gafete` de `prepararIngreso`), hace
 * falta el número o marcar "Sin gafete" (pedido del usuario 2026-10-03,
 * igual que el personal de ruta): así nunca queda S/G por olvido. */
export function gafeteParaRegistrar(
  texto: string,
  requiereGafete: boolean,
  sinGafete: boolean,
): { gafete: number | null } | { error: string } {
  if (!requiereGafete || sinGafete) return { gafete: null };
  const recortado = texto.trim();
  if (!recortado) return { error: "Ingrese el número de gafete o marque «Sin gafete»" };
  const numero = Number.parseInt(recortado, 10);
  return Number.isNaN(numero) ? { error: "Ingrese un número de gafete válido" } : { gafete: numero };
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
