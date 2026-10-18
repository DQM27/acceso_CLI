import type { ActivoUnificado } from "../api/activosUnificados";

/** "GAFETE 2" / "S/G" (sin gafete), en mayúsculas como el resto de los
 * datos del contratista. */
export function textoGafete(numero: number | null): string {
  return numero == null ? "S/G" : `GAFETE ${numero}`;
}

/** Por nombre, cédula o lo que acompaña al nombre (empresa, motivo o código de
 * empleado en KOF, que no tiene cédula). */
export function coincideTexto(activo: ActivoUnificado, textoBuscado: string): boolean {
  const buscado = textoBuscado.toLowerCase();
  return (
    activo.nombre.toLowerCase().includes(buscado) ||
    (activo.cedula?.toLowerCase().includes(buscado) ?? false) ||
    (activo.detalle?.toLowerCase().includes(buscado) ?? false)
  );
}

/** Los activos por número de gafete. Un número puede tener MÁS DE UNO: los
 * gafetes son de conjuntos distintos (contratista, proveedor, visita, KOF) y
 * cada conjunto numera por su cuenta, así que el 1 de un proveedor y el 1 de un
 * contratista son dos gafetes físicos distintos y pueden estar adentro a la
 * vez. Quien da la salida elige a cuál se refiere. */
export function agruparPorGafete(activos: readonly ActivoUnificado[]): Map<number, ActivoUnificado[]> {
  const mapa = new Map<number, ActivoUnificado[]>();
  for (const activo of activos) {
    if (activo.gafete_numero === null) continue;
    mapa.set(activo.gafete_numero, [...(mapa.get(activo.gafete_numero) ?? []), activo]);
  }
  return mapa;
}

/** A quién se le da la salida con ese número: el único que lo tiene, o el que
 * se eligió si hay varios. `undefined` si nadie lo tiene, o si hay varios y
 * todavía no se eligió. */
export function activoParaGafete(
  coincidencias: readonly ActivoUnificado[] | undefined,
  claveElegida: string | undefined,
): ActivoUnificado | undefined {
  if (!coincidencias || coincidencias.length === 0) return undefined;
  if (coincidencias.length === 1) return coincidencias[0];
  return coincidencias.find((activo) => activo.clave === claveElegida);
}
