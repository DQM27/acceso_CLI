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
import type { TenedorGafete } from "./gafetes";
import type { FilaGafeteProvisionalActiva } from "./gafetesProvisionales";
import { registrarSalidasContratistas, textoMedioConPlaca } from "./ingresos";
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

/** La `clave` que tiene en la tabla el activo al que se refiere un tenedor del
 * núcleo: la misma forma que arma cada `activoDesde*` (clase, y `local-<id>` o
 * `remoto-<uuid>`). */
export function claveDeTenedor(tenedor: TenedorGafete): string {
  const origen =
    tenedor.origen.tipo === "local" ? `local-${tenedor.origen.id}` : `remoto-${tenedor.origen.uuid}`;
  return `${tenedor.clase}-${origen}`;
}

/** Los activos ya cargados a los que se refieren los tenedores que contestó el
 * núcleo (los necesita `cerrarActivo` para saber cómo darles la salida). Un
 * tenedor que ya no está en la lista cargada se descarta: la lista y el núcleo
 * pueden estar desfasados un instante. */
export function activosDeTenedores(
  activos: readonly ActivoUnificado[],
  tenedores: readonly TenedorGafete[],
): ActivoUnificado[] {
  const porClave = new Map(activos.map((activo) => [activo.clave, activo]));
  return tenedores.flatMap((tenedor) => {
    const activo = porClave.get(claveDeTenedor(tenedor));
    return activo ? [activo] : [];
  });
}

export interface RechazoDeActivo {
  activo: ActivoUnificado;
  mensaje: string;
}

export interface ResultadoCierreDeActivos {
  /** En el orden en que llegaron. */
  cerrados: ActivoUnificado[];
  rechazados: RechazoDeActivo[];
}

/** Da la salida a varios activos de una vez y NUNCA lanza: lo que no se pudo
 * cerrar vuelve en `rechazados` con su motivo, y lo demás se cierra igual.
 *
 * Los contratistas, locales y remotos, van todos en una sola llamada al núcleo
 * (`registrarSalidasContratistas`: una transacción, un `PATCH` por cada 50 y un
 * solo aviso de sincronización). Las demás clases siguen cerrándose de una en
 * una, cada una por su camino, hasta que tengan su propio lote. */
export async function cerrarActivos(
  activos: readonly ActivoUnificado[],
): Promise<ResultadoCierreDeActivos> {
  const cerrados = new Set<string>();
  const motivos = new Map<string, string>();

  const locales = new Map<number, ActivoUnificado>();
  const remotos = new Map<string, ActivoUnificado>();
  for (const activo of activos) {
    if (activo.fuente.clase !== "contratista") continue;
    const { fila } = activo.fuente;
    if (fila.origen === "local") locales.set(fila.registro_id, activo);
    else remotos.set(fila.uuid_remoto, activo);
  }

  if (locales.size + remotos.size > 0) {
    try {
      const resultado = await registrarSalidasContratistas([...locales.keys()], [...remotos.keys()]);
      for (const id of resultado.locales.cerrados) {
        const activo = locales.get(id);
        if (activo) cerrados.add(activo.clave);
      }
      for (const uuid of resultado.remotos.cerrados) {
        const activo = remotos.get(uuid);
        if (activo) cerrados.add(activo.clave);
      }
      for (const { id, mensaje } of resultado.locales.rechazados) {
        const activo = locales.get(id);
        if (activo) motivos.set(activo.clave, mensaje);
      }
      for (const { uuid, mensaje } of resultado.remotos.rechazados) {
        const activo = remotos.get(uuid);
        if (activo) motivos.set(activo.clave, mensaje);
      }
    } catch (error) {
      for (const activo of [...locales.values(), ...remotos.values()]) {
        motivos.set(activo.clave, String(error));
      }
    }
  }

  for (const activo of activos) {
    if (activo.fuente.clase === "contratista") continue;
    try {
      await cerrarActivo(activo);
      cerrados.add(activo.clave);
    } catch (error) {
      motivos.set(activo.clave, String(error));
    }
  }

  return {
    cerrados: activos.filter((activo) => cerrados.has(activo.clave)),
    rechazados: activos
      .filter((activo) => !cerrados.has(activo.clave))
      .map((activo) => ({
        activo,
        // Un contratista que el núcleo no nombró ni como cerrado ni como
        // rechazado no debería pasar; si pasa, no se da por cerrado.
        mensaje: motivos.get(activo.clave) ?? "No se pudo registrar la salida",
      })),
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
