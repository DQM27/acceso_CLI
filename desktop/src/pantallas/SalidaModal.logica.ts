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

/** A quién se le da la salida con ese número: el único que lo tiene, o el que
 * se eligió si hay varios. `undefined` si nadie lo tiene, o si hay varios y
 * todavía no se eligió. Quién tiene cada número (y que pueden ser varios, de
 * conjuntos de gafetes distintos) lo contesta el núcleo
 * (`tenedores_del_gafete`); acá sólo se aplica la elección de quien opera. */
export function activoParaGafete(
  coincidencias: readonly ActivoUnificado[] | undefined,
  claveElegida: string | undefined,
): ActivoUnificado | undefined {
  if (!coincidencias || coincidencias.length === 0) return undefined;
  if (coincidencias.length === 1) return coincidencias[0];
  return coincidencias.find((activo) => activo.clave === claveElegida);
}
