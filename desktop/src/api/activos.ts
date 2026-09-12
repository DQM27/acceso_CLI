import { listarIngresosActivos, registrarSalida } from "./ingresos";
import type { IngresoActivoResumen, ListaIngresosActivosResumen, MedioIngreso } from "./ingresos";
import {
  cerrarIngresoRemoto,
  listarIngresosRemotos,
  medioIngresoDesdeNube,
  tipoIngresoDesdeNube,
} from "./nube";
import type { IngresoRemoto } from "./nube";
import type { TipoIngreso } from "./contratistas";

/** Fila local (este dispositivo) o remota (abierta por el otro dispositivo
 * del mismo sitio, ver `docs/planes-implementados/plan-persistencia-nube.md` — nunca vive en el
 * historial local, sólo en la caché `ingresos_remotos`). Mismos nombres de
 * campo en los dos casos (los que una remota no tiene van en `null`) para
 * que las columnas de AG Grid (`Activos.tsx`) y la búsqueda de
 * `SalidaModal.tsx` no necesiten saber cuál es cuál — sólo `cerrarFilaActiva`
 * decide internamente cómo cerrar la fila. */
export interface FilaLocal extends IngresoActivoResumen {
  origen: "local";
}

export interface FilaRemota {
  origen: "remoto";
  uuid_remoto: string;
  registro_id: null;
  contratista_id: null;
  cedula: string | null;
  contratista_nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: TipoIngreso | null;
  medio_ingreso: MedioIngreso | null;
  fecha_hora_ingreso: string;
  gafete_numero: number | null;
  usuario_ingreso_nombre: string;
  resultado_registrado: null;
  resultado_acceso: null;
}

export type FilaActiva = FilaLocal | FilaRemota;

export function filaDesdeLocal(item: IngresoActivoResumen): FilaActiva {
  return { ...item, origen: "local" };
}

export function filaDesdeRemoto(remoto: IngresoRemoto): FilaActiva {
  return {
    origen: "remoto",
    uuid_remoto: remoto.uuid,
    registro_id: null,
    contratista_id: null,
    cedula: remoto.contratista_cedula,
    contratista_nombre: remoto.contratista_nombre,
    empresa_nombre: remoto.empresa_nombre,
    tipo_ingreso: tipoIngresoDesdeNube(remoto.tipo_ingreso),
    medio_ingreso: medioIngresoDesdeNube(remoto.medio_ingreso),
    fecha_hora_ingreso: remoto.hora_entrada,
    gafete_numero: remoto.gafete_numero,
    usuario_ingreso_nombre: remoto.usuario_entrada_nombre ?? "—",
    resultado_registrado: null,
    resultado_acceso: null,
  };
}

/** Local + remotas de este sitio, en una sola lista -- misma fuente que usa
 * `Activos.tsx` para la grilla, ahora también consumida por `SalidaModal.tsx`
 * (antes sólo buscaba entre locales: una persona activa abierta por otro
 * dispositivo del mismo sitio no aparecía al buscarla para darle salida,
 * aunque sí se podía cerrar desde la grilla -- inconsistencia real, no sólo
 * de UI). `listarIngresosRemotos` no hace red -- lee la caché local que ya
 * llenó la última sincronización; si la nube nunca se configuró en este
 * dispositivo, simplemente devuelve una lista vacía, no falla. */
export async function listarTodosLosActivos(): Promise<{
  filas: FilaActiva[];
  total: number;
}> {
  const [pagina, remotos]: [ListaIngresosActivosResumen, IngresoRemoto[]] = await Promise.all([
    listarIngresosActivos(),
    listarIngresosRemotos(),
  ]);
  return {
    filas: [...pagina.items.map(filaDesdeLocal), ...remotos.map(filaDesdeRemoto)],
    total: pagina.total + remotos.length,
  };
}

/** Clave estable para listas de React (`key`) y grillas -- `registro_id` es
 * `null` en una remota, así que no alcanza solo. */
export function claveFilaActiva(fila: FilaActiva): string {
  return fila.origen === "local" ? `local-${fila.registro_id}` : `remoto-${fila.uuid_remoto}`;
}

/** Local: cierra en `registro_ingresos` (este dispositivo). Remota: cierra
 * directo contra la nube (`nube::cerrar_ingreso_remoto`) -- nunca toca el
 * historial local, esa fila no es -- ni fue -- de este dispositivo. */
export async function cerrarFilaActiva(fila: FilaActiva): Promise<void> {
  if (fila.origen === "local") {
    await registrarSalida(fila.registro_id);
  } else {
    await cerrarIngresoRemoto(fila.uuid_remoto);
  }
}
