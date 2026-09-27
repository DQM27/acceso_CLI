import type { HistorialIngresoProveedorRemoto } from "../api/proveedores";

/** Un proveedor que ya ingresó alguna vez al sitio, con los datos de su
 * ingreso más reciente. */
export interface ProveedorConocido {
  cedula: string;
  nombre: string;
  empresa_nombre: string | null;
}

/** Un proveedor por cédula, con el nombre y la empresa de su ingreso más
 * reciente, ordenados del más reciente al más viejo. */
export function proveedoresConocidos(
  historial: Pick<HistorialIngresoProveedorRemoto, "cedula" | "nombre" | "empresa_nombre" | "fecha_hora_ingreso">[],
): ProveedorConocido[] {
  const ordenado = [...historial].sort((a, b) =>
    b.fecha_hora_ingreso.localeCompare(a.fecha_hora_ingreso),
  );
  const vistos = new Set<string>();
  const conocidos: ProveedorConocido[] = [];
  for (const ingreso of ordenado) {
    const cedula = ingreso.cedula.trim();
    if (!cedula || vistos.has(cedula)) continue;
    vistos.add(cedula);
    conocidos.push({ cedula, nombre: ingreso.nombre, empresa_nombre: ingreso.empresa_nombre });
  }
  return conocidos;
}

/** "Nuevo proveedor" con lo que se escribió en el buscador: si parece una
 * cédula (sólo dígitos, espacios o guiones) va a la cédula; si no, al
 * nombre. */
export function datosNuevoDesdeBusqueda(texto: string): { cedula: string; nombre: string } {
  const recortado = texto.trim();
  return /^[\d\s-]+$/.test(recortado)
    ? { cedula: recortado, nombre: "" }
    : { cedula: "", nombre: recortado.toUpperCase() };
}
