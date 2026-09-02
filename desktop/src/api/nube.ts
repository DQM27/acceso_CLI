import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/nube.rs. Exclusivo de ROOT (ver App.tsx, rolesPermitidos)
// -- el secreto identifica al dispositivo entero ante el receptor.

export interface ResumenSincronizacion {
  enviados: number;
  fallidos: number;
  remotos_abiertos: number;
  sitio_id: string;
  dispositivo_id: string;
  tipo: string;
}

/** Ingreso abierto por el otro dispositivo del mismo sitio -- no vive en el
 * historial local (ver `database::schema`, tabla `ingresos_remotos`, y el
 * comentario en `nube::sincronizacion::IngresoRemoto`). */
export interface IngresoRemoto {
  uuid: string;
  contratista_nombre: string;
  /** ISO 8601 (UTC). */
  hora_entrada: string;
  usuario_entrada_nombre: string | null;
}

export function guardarSecretoDispositivo(secreto: string): Promise<void> {
  return invoke("guardar_secreto_dispositivo", { secreto });
}

export function secretoDispositivoGuardado(): Promise<boolean> {
  return invoke("secreto_dispositivo_guardado");
}

export function sincronizarConNube(): Promise<ResumenSincronizacion> {
  return invoke("sincronizar_con_nube");
}

export function listarIngresosRemotos(): Promise<IngresoRemoto[]> {
  return invoke("listar_ingresos_remotos");
}

export function cerrarIngresoRemoto(uuid: string): Promise<void> {
  return invoke("cerrar_ingreso_remoto", { uuid });
}

/** Filas de la cola que ya agotaron los reintentos automáticos y quedaron
 * `fallido` de forma permanente -- necesitan que alguien las mire. */
export function fallosPermanentesNube(): Promise<number> {
  return invoke("fallos_permanentes_nube");
}
