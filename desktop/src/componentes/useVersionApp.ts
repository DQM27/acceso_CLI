import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

/** Versión instalada, leída de Tauri (`getVersion()`, refleja
 * `tauri.conf.json` real) y no de `package.json`, que quedó congelado en
 * "0.0.0". Cadena vacía mientras llega o si no hay Tauri detrás. */
export function useVersionApp(): string {
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {
        // Sin Tauri detrás (no debería pasar en la app empaquetada) no hay
        // versión que mostrar -- no es un error que el usuario deba ver.
      });
  }, []);
  return version;
}
