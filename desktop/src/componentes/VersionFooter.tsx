import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

/**
 * Versión instalada, visible en la esquina del sidebar -- para que quien
 * reporta un problema pueda decir qué versión tiene sin ir a buscarla (ver
 * docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, punto "Registro
 * de versión instalada"). Se lee de Tauri (`getVersion()`, refleja
 * `tauri.conf.json` real) y no de `package.json` -- ese campo quedó
 * congelado en "0.0.0" y no es la fuente de verdad de la app empaquetada.
 */
export default function VersionFooter({ colapsado }: { colapsado: boolean }) {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {
        // Sin Tauri detrás (no debería pasar en la app empaquetada) no hay
        // versión que mostrar -- no es un error que el usuario deba ver.
      });
  }, []);

  if (!version) return null;

  return (
    <div className="shell-sidebar-version" title={`Lattis v${version}`}>
      {/* Antes se ocultaba con el sidebar colapsado (¡colapsado &&) -- pero
          la columna angosta (3.75rem, ver .shell-sidebar-colapsada) tiene
          espacio de sobra para "v1.6.4" a este tamaño de fuente (0.7rem),
          no hacía falta esconderlo (hallazgo real del usuario, 2026-09-21). */}
      {`v${version}`}
    </div>
  );
}
