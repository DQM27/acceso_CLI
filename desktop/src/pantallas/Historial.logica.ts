import { medioIngresoDesdeNube, tipoIngresoDesdeNube } from "../api";
import type { MovimientoHistorialRemoto, MovimientoIngresoResumen } from "../api";

/** "pc"/"mobile" (`dispositivos.tipo`) → sólo el ícono para la columna
 * "Dispositivo" -- ícono+palabra ("💻 PC"/"📱 Celular") quedaba desparejo
 * visualmente (una palabra bastante más larga que la otra). Cualquier otro
 * valor (o `null`) se muestra tal cual / como "—", nunca se inventa un tipo
 * que no vino. */
export function textoDispositivo(tipo: string | null): string {
  if (tipo === "pc") return "💻";
  if (tipo === "mobile") return "📱";
  return tipo ?? "—";
}

/** `historial_sitio` incluye a propósito los movimientos de ESTE mismo
 * dispositivo (respaldo ante una reinstalación que pierda
 * `registro_ingresos` local, ver `src/nube/sincronizacion.rs`,
 * `recibir_historial_del_sitio`) -- sin este filtro, cada movimiento que ya
 * se sincronizó aparece dos veces: una vez como local y otra como remoto
 * (bug real en producción, 2026-09-11). Mismo criterio que ya aplica la UI
 * de Android (`HistorialViewModel.kt`). */
export function remotosSinDuplicarLocales(
  locales: readonly { uuid: string }[],
  remotos: readonly MovimientoHistorialRemoto[],
): MovimientoHistorialRemoto[] {
  const localesUuids = new Set(locales.map((fila) => fila.uuid));
  return remotos.filter((remoto) => !localesUuids.has(remoto.uuid));
}

/** Local (este dispositivo, con la auditoría completa de la decisión de
 * acceso) o remota (generada por otro dispositivo del mismo sitio, leída
 * de la caché `historial_sitio` -- ver `docs/planes-implementados/plan-persistencia-nube.md`).
 * Decisión explícita del usuario: es la misma operación vista desde otro
 * dispositivo, no una versión resumida -- se combinan en una sola grilla
 * con los mismos campos que ya muestra Historial. Excel/PDF recortan por
 * `uuid` (ver `seleccionParaExportar`), que ambas tienen -- una fila remota
 * no tiene `registro_id` local. */
export interface FilaLocal extends MovimientoIngresoResumen {
  origen: "local";
  // Siempre "pc": esta pantalla sólo existe en el build de escritorio, no
  // hace falta leerlo de ningún lado -- a diferencia de una fila remota,
  // que sí puede venir de cualquier tipo de dispositivo del sitio.
  dispositivo_tipo: "pc";
}

export interface FilaRemota {
  origen: "remoto";
  uuid: string;
  registro_id: null;
  contratista_id: null;
  cedula: string | null;
  contratista_nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: MovimientoIngresoResumen["tipo_ingreso"] | null;
  medio_ingreso: MovimientoIngresoResumen["medio_ingreso"] | null;
  fecha_hora_ingreso: string;
  fecha_hora_salida: string | null;
  gafete_numero: number | null;
  placa: string | null;
  usuario_ingreso_nombre: string;
  usuario_salida_nombre: string | null;
  resultado_acceso: null;
  motivo_resultado: null;
  reglas_version: null;
  empresa_activa_snapshot: null;
  // `null` para movimientos remotos sincronizados antes de que
  // `historial_sitio.dispositivo_entrada_tipo` existiera (migración 26).
  dispositivo_tipo: string | null;
}

export type FilaHistorial = FilaLocal | FilaRemota;

export function filaDesdeRemoto(remoto: MovimientoHistorialRemoto): FilaHistorial {
  return {
    origen: "remoto",
    uuid: remoto.uuid,
    registro_id: null,
    contratista_id: null,
    cedula: remoto.cedula,
    contratista_nombre: remoto.contratista_nombre,
    empresa_nombre: remoto.empresa_nombre,
    tipo_ingreso: tipoIngresoDesdeNube(remoto.tipo_ingreso),
    medio_ingreso: medioIngresoDesdeNube(remoto.medio_ingreso),
    fecha_hora_ingreso: remoto.fecha_hora_ingreso,
    fecha_hora_salida: remoto.fecha_hora_salida,
    gafete_numero: remoto.gafete_numero,
    placa: remoto.placa,
    usuario_ingreso_nombre: remoto.usuario_ingreso_nombre ?? "—",
    usuario_salida_nombre: remoto.usuario_salida_nombre,
    resultado_acceso: null,
    motivo_resultado: null,
    reglas_version: null,
    empresa_activa_snapshot: null,
    dispositivo_tipo: remoto.dispositivo_entrada_tipo,
  };
}
