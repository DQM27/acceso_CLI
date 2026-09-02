import { invoke } from "@tauri-apps/api/core";

// Espejo de comandos/nube.rs. Exclusivo de ROOT (ver App.tsx, rolesPermitidos)
// -- el secreto identifica al dispositivo entero ante el receptor.

export interface ResumenSincronizacion {
  enviados: number;
  fallidos: number;
  sitio_id: string;
  dispositivo_id: string;
  tipo: string;
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
