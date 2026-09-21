import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

// Espejo de comandos/gafetes_provisionales.rs, comandos/nube.rs y de
// GafetesProvisionalesViewModel.kt (mobile) -- mismo módulo, mismo criterio
// de "un solo campo bloqueante (encargado), sin wizard de pasos".

export interface PrestamoGafeteProvisionalActivoResumen {
  id: number;
  encargado_nombre: string;
  encargado_codigo_empleado: string;
  gafete_numero: number;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_entrega: string;
  usuario_entrega_nombre: string;
}

/** Espejo de `nube::PrestamoGafeteProvisionalRemoto` -- un préstamo abierto
 * por el OTRO dispositivo del mismo sitio, listo para mostrarse y, si hace
 * falta, cerrarse (registrar devolución) desde acá. */
export interface PrestamoGafeteProvisionalRemoto {
  uuid: string;
  encargado_nombre: string;
  encargado_codigo_empleado: string;
  gafete_numero: number;
  hora_entrega: string;
  usuario_entrega_nombre: string;
}

export function entregarGafeteProvisional(
  encargadoId: number,
  gafeteNumero: number,
): Promise<number> {
  return invoke<number>("entregar_gafete_provisional", {
    encargadoId,
    gafeteNumero,
  }).then((id) => {
    solicitarSincronizacionNube();
    return id;
  });
}

export async function registrarDevolucionGafeteProvisional(id: number): Promise<void> {
  await invoke("registrar_devolucion_gafete_provisional", { id });
  solicitarSincronizacionNube();
}

export function listarGafetesProvisionalesActivos(): Promise<
  PrestamoGafeteProvisionalActivoResumen[]
> {
  return invoke("listar_gafetes_provisionales_activos");
}

export function listarPrestamosGafeteProvisionalRemotos(): Promise<
  PrestamoGafeteProvisionalRemoto[]
> {
  return invoke("listar_prestamos_gafete_provisional_remotos");
}

export async function cerrarPrestamoGafeteProvisionalRemoto(uuid: string): Promise<void> {
  await invoke("cerrar_prestamo_gafete_provisional_remoto", { uuid });
  solicitarSincronizacionNube();
}

// ---- Local + remoto fusionados -- mismo criterio que api/proveedores.ts:
// un préstamo entregado por el OTRO dispositivo del mismo sitio nunca vive
// en `prestamos_gafete_provisional` de este, sólo en la caché
// `prestamos_gafete_provisional_remotos` -- se fusionan acá para que la
// pantalla muestre y pueda devolver ambos sin distinguir de dónde vino
// cada fila. ----

export interface FilaGafeteProvisionalLocal extends PrestamoGafeteProvisionalActivoResumen {
  origen: "local";
}

export interface FilaGafeteProvisionalRemota {
  origen: "remoto";
  uuid_remoto: string;
  id: null;
  encargado_nombre: string;
  encargado_codigo_empleado: string;
  gafete_numero: number;
  fecha_hora_entrega: string;
  usuario_entrega_nombre: string;
}

export type FilaGafeteProvisionalActiva = FilaGafeteProvisionalLocal | FilaGafeteProvisionalRemota;

export function filaGafeteProvisionalDesdeLocal(
  item: PrestamoGafeteProvisionalActivoResumen,
): FilaGafeteProvisionalActiva {
  return { ...item, origen: "local" };
}

export function filaGafeteProvisionalDesdeRemoto(
  remoto: PrestamoGafeteProvisionalRemoto,
): FilaGafeteProvisionalActiva {
  return {
    origen: "remoto",
    uuid_remoto: remoto.uuid,
    id: null,
    encargado_nombre: remoto.encargado_nombre,
    encargado_codigo_empleado: remoto.encargado_codigo_empleado,
    gafete_numero: remoto.gafete_numero,
    fecha_hora_entrega: remoto.hora_entrega,
    usuario_entrega_nombre: remoto.usuario_entrega_nombre,
  };
}

/** Clave estable para listas de React (`key`) -- `id` es `null` en una
 * remota, así que no alcanza solo. */
export function claveFilaGafeteProvisionalActiva(fila: FilaGafeteProvisionalActiva): string {
  return fila.origen === "local" ? `local-${fila.id}` : `remoto-${fila.uuid_remoto}`;
}

export async function listarTodosLosGafetesProvisionalesActivos(): Promise<
  FilaGafeteProvisionalActiva[]
> {
  const [locales, remotos] = await Promise.all([
    listarGafetesProvisionalesActivos(),
    listarPrestamosGafeteProvisionalRemotos(),
  ]);
  return [
    ...locales.map(filaGafeteProvisionalDesdeLocal),
    ...remotos.map(filaGafeteProvisionalDesdeRemoto),
  ];
}

/** Local: cierra en `prestamos_gafete_provisional` (este dispositivo).
 * Remota: cierra directo contra la nube (`cerrarPrestamoGafeteProvisionalRemoto`) --
 * nunca toca el historial local, ese préstamo no es -- ni fue -- de este
 * dispositivo. */
export async function cerrarFilaGafeteProvisionalActiva(
  fila: FilaGafeteProvisionalActiva,
): Promise<void> {
  if (fila.origen === "local") {
    await registrarDevolucionGafeteProvisional(fila.id);
  } else {
    await cerrarPrestamoGafeteProvisionalRemoto(fila.uuid_remoto);
  }
}
