import { NavLink } from "react-router-dom";
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

      <div
        style={{ flex: 1 }}
        title="Doble click para colapsar/expandir"
        onDoubleClick={onToggleColapsado}
      />
    </nav>
  );
}
