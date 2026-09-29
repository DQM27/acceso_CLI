import { invoke as invokeTauri } from "@tauri-apps/api/core";
import type { InvokeArgs, InvokeOptions } from "@tauri-apps/api/core";
import { registrarLlamada, telemetriaActiva } from "../telemetria";

/** `invoke` de Tauri, midiendo cuánto tarda cada comando (IPC incluido)
 * para la telemetría de diagnóstico. Sin telemetría es el mismo `invoke`:
 * todos los módulos de `api/` lo usan en su lugar. Sólo se anota el nombre
 * del comando, nunca sus argumentos ni su resultado. */
export async function invoke<T>(
  comando: string,
  ...resto: [args?: InvokeArgs, opciones?: InvokeOptions]
): Promise<T> {
  // `resto` tal cual: reenvía exactamente los argumentos recibidos.
  if (!telemetriaActiva()) return invokeTauri<T>(comando, ...resto);
  const inicio = performance.now();
  try {
    const resultado = await invokeTauri<T>(comando, ...resto);
    registrarLlamada(comando, performance.now() - inicio, true);
    return resultado;
  } catch (error) {
    registrarLlamada(comando, performance.now() - inicio, false);
    throw error;
  }
}
