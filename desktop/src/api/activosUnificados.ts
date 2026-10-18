import { cerrarFilaActiva, claveFilaActiva, listarTodosLosActivos } from "./activos";
import type { FilaActiva } from "./activos";
import { textoTipoIngreso } from "./contratistas";
import { cerrarFilaCorreoActiva, claveFilaCorreoActiva, listarTodosLosCorreosActivos } from "./correo";
import type { FilaCorreoActiva } from "./correo";
import {
  cerrarFilaGafeteProvisionalActiva,
  claveFilaGafeteProvisionalActiva,
  listarTodosLosGafetesProvisionalesActivos,
} from "./gafetesProvisionales";
import type { FilaGafeteProvisionalActiva } from "./gafetesProvisionales";
import { textoMedioConPlaca } from "./ingresos";
import type { NivelPermanencia } from "./ingresos";
import {
  cerrarFilaProveedorActiva,
  claveFilaProveedorActiva,
  listarTodosLosProveedoresActivos,
} from "./proveedores";
import type { FilaProveedorActiva } from "./proveedores";

// Vista unificada de "quién está adentro" (pantalla Activos): contratistas,
// proveedores, visitas autorizadas por correo y préstamos de gafete
// provisional KOF. Cada clase ya tenía su propio listado -- local más lo que
// abrió el otro dispositivo del sitio -- y su propia forma de cerrarse; acá sólo
// se llevan a una fila común, sin tocar el núcleo ni la base. Rutas y visitas
// agendadas quedan fuera a propósito.

export type ClaseActivo = "contratista" | "proveedor" | "correo" | "kof";

/** La fila original de cada clase: es lo que `cerrarActivo` necesita para saber
 * cómo cerrarla (local en este equipo, o remota contra la nube). */
export type FuenteActivo =
  | { clase: "contratista"; fila: FilaActiva }
  | { clase: "proveedor"; fila: FilaProveedorActiva }
  | { clase: "correo"; fila: FilaCorreoActiva }
  | { clase: "kof"; fila: FilaGafeteProvisionalActiva };

export interface ActivoUnificado {
  clase: ClaseActivo;
  /** Única entre clases: un `id` local 7 de contratistas y uno de proveedores
   * no pueden chocar en la grilla. */
  clave: string;
  /** `null` donde no hay (el préstamo KOF es de un encargado con código de
   * empleado, no con cédula). */
  cedula: string | null;
  nombre: string;
  /** Empresa (contratista, proveedor), motivo (por correo) o código de empleado
   * (KOF): lo que acompaña al nombre para reconocer a la persona. */
  detalle: string | null;
  /** Texto de la columna "Tipo", ya en el formato de la grilla. */
  tipo: string;
  medio: string;
  gafete_numero: number | null;
  /** ISO 8601 (UTC): la entrada, o la entrega del gafete en KOF. */
  fecha_hora_ingreso: string;
  usuario_ingreso_nombre: string;
  /** Sólo los contratistas tienen nivel: el núcleo calcula los avisos de
   * permanencia únicamente para ellos. */
  nivel_permanencia: NivelPermanencia | null;
  fuente: FuenteActivo;
}

const SIN_DATO = "—";

/** Proveedores y visitas por correo sólo guardan la placa: con placa entraron en
 * vehículo, sin ella caminando -- mismo texto que el medio de los contratistas. */
function medioDesdePlaca(placa: string | null): string {
  return placa !== null && placa.trim() !== ""
    ? textoMedioConPlaca("Vehiculo", placa)
    : textoMedioConPlaca("Caminando", null);
}

export function activoDesdeContratista(fila: FilaActiva): ActivoUnificado {
  return {
    clase: "contratista",
    clave: `contratista-${claveFilaActiva(fila)}`,
    cedula: fila.cedula,
    nombre: fila.contratista_nombre,
    detalle: fila.empresa_nombre,
    tipo: textoTipoIngreso(fila.tipo_ingreso),
    medio:
      fila.medio_ingreso == null
        ? SIN_DATO
        : textoMedioConPlaca(fila.medio_ingreso, fila.placa ?? null),
    gafete_numero: fila.gafete_numero,
    fecha_hora_ingreso: fila.fecha_hora_ingreso,
    usuario_ingreso_nombre: fila.usuario_ingreso_nombre,
    nivel_permanencia: fila.nivel_permanencia,
    fuente: { clase: "contratista", fila },
  };
}

export function activoDesdeProveedor(fila: FilaProveedorActiva): ActivoUnificado {
  return {
    clase: "proveedor",
    clave: `proveedor-${claveFilaProveedorActiva(fila)}`,
    cedula: fila.cedula,
    nombre: fila.nombre,
    detalle: fila.empresa_nombre,
    tipo: "PROVEEDOR",
    medio: medioDesdePlaca(fila.placa),
    gafete_numero: fila.gafete_numero,
    fecha_hora_ingreso: fila.fecha_hora_ingreso,
    usuario_ingreso_nombre: fila.usuario_ingreso_nombre,
    nivel_permanencia: null,
    fuente: { clase: "proveedor", fila },
  };
}

export function activoDesdeCorreo(fila: FilaCorreoActiva): ActivoUnificado {
  return {
    clase: "correo",
    clave: `correo-${claveFilaCorreoActiva(fila)}`,
    cedula: fila.cedula,
    nombre: fila.nombre,
    detalle: fila.motivo,
    // No es el tipo "Por correo" de un contratista (vía de ingreso): es una
    // visita autorizada por correo, que el guarda registra a mano.
    tipo: "VISITA POR CORREO",
    medio: medioDesdePlaca(fila.placa),
    gafete_numero: fila.gafete_numero,
    fecha_hora_ingreso: fila.fecha_hora_ingreso,
    usuario_ingreso_nombre: fila.usuario_ingreso_nombre,
    nivel_permanencia: null,
    fuente: { clase: "correo", fila },
  };
}

export function activoDesdeKof(fila: FilaGafeteProvisionalActiva): ActivoUnificado {
  return {
    clase: "kof",
    clave: `kof-${claveFilaGafeteProvisionalActiva(fila)}`,
    cedula: null,
    nombre: fila.encargado_nombre,
    detalle: `Cód. ${fila.encargado_codigo_empleado}`,
    tipo: "KOF",
    medio: SIN_DATO,
    gafete_numero: fila.gafete_numero,
    fecha_hora_ingreso: fila.fecha_hora_entrega,
    usuario_ingreso_nombre: fila.usuario_entrega_nombre,
    nivel_permanencia: null,
    fuente: { clase: "kof", fila },
  };
}

/** Las cuatro listas en paralelo. Si una falla, falla todo: mostrar la pantalla
 * sin una de las clases haría creer que esa gente ya salió. `total` conserva el
 * criterio de contratistas (su `total` puede pasar de las filas si el núcleo
 * recorta la lista). */
export async function listarActivosUnificados(): Promise<{
  activos: ActivoUnificado[];
  total: number;
}> {
  const [contratistas, proveedores, correos, kof] = await Promise.all([
    listarTodosLosActivos(),
    listarTodosLosProveedoresActivos(),
    listarTodosLosCorreosActivos(),
    listarTodosLosGafetesProvisionalesActivos(),
  ]);
  return {
    activos: [
      ...contratistas.filas.map(activoDesdeContratista),
      ...proveedores.map(activoDesdeProveedor),
      ...correos.map(activoDesdeCorreo),
      ...kof.map(activoDesdeKof),
    ],
    total: contratistas.total + proveedores.length + correos.length + kof.length,
  };
}

/** Da la salida de un contratista, proveedor o visita por correo, o registra la
 * devolución de un gafete KOF, cada uno por su camino (local o remoto). */
export function cerrarActivo(activo: ActivoUnificado): Promise<void> {
  const { fuente } = activo;
  switch (fuente.clase) {
    case "contratista":
      return cerrarFilaActiva(fuente.fila);
    case "proveedor":
      return cerrarFilaProveedorActiva(fuente.fila);
    case "correo":
      return cerrarFilaCorreoActiva(fuente.fila);
    case "kof":
      return cerrarFilaGafeteProvisionalActiva(fuente.fila);
  }
}
