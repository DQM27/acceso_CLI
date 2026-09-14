import { NavLink } from "react-router-dom";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Seccion } from "../App";
import { rutaSeccion } from "../App";

/**
 * Sidebar izquierdo del portal — mismo componente que
 * `desktop/src/componentes/Sidebar.tsx` y `web/src/componentes/Sidebar.tsx`
 * (sólo navegación de secciones; usuario y "Cerrar sesión" viven en la barra
 * de estado, ver `App.tsx`). La sección activa la decide la URL (`NavLink`).
 */
export default function Sidebar({
  secciones,
  onNavegar,
  colapsado,
  onToggleColapsado,
  abiertoEnMovil,
}: {
  secciones: { id: Seccion; etiqueta: string; Icono: LucideIcon }[];
  /** Se dispara al elegir una sección -- hoy sólo usado para cerrar el cajón
   * en mobile; cuál queda activa la decide el propio `NavLink`. */
  onNavegar: () => void;
  colapsado: boolean;
  onToggleColapsado: () => void;
  /** Cajón abierto en pantallas angostas -- independiente de `colapsado`,
   * que sólo aplica en escritorio. */
  abiertoEnMovil: boolean;
}) {
  return (
    <nav
      className={`shell-sidebar ${colapsado ? "shell-sidebar-colapsada" : ""} ${abiertoEnMovil ? "shell-sidebar-abierta" : ""}`}
    >
      <div className="shell-nav">
        {secciones.map(({ id, etiqueta, Icono }) => (
          <NavLink
            key={id}
            to={rutaSeccion(id)}
            onClick={onNavegar}
            title={colapsado ? etiqueta : undefined}
            className={({ isActive }) => `nav-item ${isActive ? "nav-item-activo" : ""}`}
          >
            <Icono aria-hidden="true" />
            <span className="nav-item-etiqueta">{etiqueta}</span>
          </NavLink>
        ))}
      </div>

      <div style={{ flex: 1 }} />

      {/* Antes era un <div> vacío sin rol de botón, sólo alcanzable con
          doble-click de mouse -- indescubrible y no operable por teclado.
          Ahora es un botón real, de un solo click, con aria-expanded. */}
      <button
        type="button"
        className="nav-item nav-item-colapsar"
        onClick={onToggleColapsado}
        aria-expanded={!colapsado}
        aria-label={colapsado ? "Expandir panel" : "Colapsar panel"}
        title={colapsado ? "Expandir panel" : undefined}
      >
        {colapsado ? (
          <PanelLeftOpen aria-hidden="true" />
        ) : (
          <PanelLeftClose aria-hidden="true" />
        )}
        <span className="nav-item-etiqueta">Colapsar panel</span>
      </button>
    </nav>
  );
}
