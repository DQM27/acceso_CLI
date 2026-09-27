import type { FilaActiva } from "../api";

/** "GAFETE 2" / "S/G" (sin gafete), en mayúsculas como el resto de los
 * datos del contratista. */
export function textoGafete(numero: number | null): string {
  return numero == null ? "S/G" : `GAFETE ${numero}`;
}

export function coincideTexto(activo: FilaActiva, textoBuscado: string): boolean {
  const buscado = textoBuscado.toLowerCase();
  return (
    activo.contratista_nombre.toLowerCase().includes(buscado) ||
    (activo.cedula?.toLowerCase().includes(buscado) ?? false)
  );
}
