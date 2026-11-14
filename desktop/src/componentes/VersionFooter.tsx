import { useVersionApp } from "./useVersionApp";

/**
 * Versión instalada, visible en la esquina del sidebar -- para que quien
 * reporta un problema pueda decir qué versión tiene sin ir a buscarla (ver
 * docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, punto "Registro
 * de versión instalada"). Se lee de Tauri (`getVersion()`, refleja
 * `tauri.conf.json` real) y no de `package.json` -- ese campo quedó
 * congelado en "0.0.0" y no es la fuente de verdad de la app empaquetada.
 *
 * Sólo texto: antes era también el botón de "buscar actualización", pero
 * nada lo indicaba. Ahora esa acción está en el menú de la cuenta
 * (`MenuUsuario`, auditoría 2026-11-14, U11).
 */
export default function VersionFooter() {
  const version = useVersionApp();
  if (!version) return null;
  return <span className="shell-sidebar-version">v{version}</span>;
}
