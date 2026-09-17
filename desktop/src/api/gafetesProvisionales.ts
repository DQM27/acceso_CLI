import { invoke } from "@tauri-apps/api/core";
import { solicitarSincronizacionNube } from "../eventosNube";

// Espejo de comandos/gafetes_provisionales.rs y de
// GafetesProvisionalesViewModel.kt (mobile) -- mismo módulo, mismo criterio
// de "un solo campo bloqueante (encargado), sin wizard de pasos".

export interface PrestamoGafeteProvisionalActivoResumen {
  id: number;
  encargado_nombre: string;
  encargado_codigo_empleado: string;
  gafete_numero: number;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_entrega: string;
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
