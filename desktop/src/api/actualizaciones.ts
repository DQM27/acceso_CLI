import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type { Update };

// Consulta plugins.updater.endpoints (tauri.conf.json) — hoy apunta a
// releases/latest/download/latest.json de GitHub Releases (ver
// desktop/docs/pendientes.md). `null` = ya está en la última versión, o el
// check falló (sin conexión, por ejemplo) — quien llama decide si avisar.
export function buscarActualizacion(): Promise<Update | null> {
  return check();
}

/** Descarga, instala y reinicia la app en la versión nueva. La firma ya se
 * verificó adentro de `downloadAndInstall()` contra `plugins.updater.pubkey`
 * — si no coincide, tira y esta promesa rechaza sin instalar nada. */
export async function instalarActualizacion(actualizacion: Update): Promise<void> {
  await actualizacion.downloadAndInstall();
  await relaunch();
}
