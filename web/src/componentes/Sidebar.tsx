import { NavLink } from "react-router-dom";
import type { LucideIcon } from "lucide-react";
import type { Seccion } from "../App";
import { rutaSeccion } from "../App";

/**
 * Sidebar izquierdo de `Shell` (`App.tsx`) — sólo navegación de secciones.
 * Usuario y "Cerrar sesión" viven en `MenuUsuario`, en la barra de estado
 * (ver `Shell`) — no acá. Copiado de `desktop/src/componentes/Sidebar.tsx`,
 * salvo que acá la sección activa la decide la URL (`NavLink`), no un
 * `useState` en `Shell` -- así recargar la página o compartir un link no
 * pierde en qué sección estabas.
 * Sin estado propio más allá de lo puramente visual: colapsado sigue
 * viviendo en `Shell`, acá sólo llega por props.
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
  /** Cajón abierto en pantallas angostas (ver `.shell-sidebar-abierta` en
   * index.css) -- independiente de `colapsado`, que solo aplica en
   * escritorio. */
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
            // React Router v7 dispara la View Transitions API nativa del
            // navegador en esta navegación -- react-router (no un
            // `<ViewTransition>` de React) porque acá, a diferencia de
            // desktop, el cambio de sección lo decide la URL, no un
            // `useState` local que se pueda envolver en `startTransition`.
            viewTransition
          >
            <Icono size={18} strokeWidth={2} aria-hidden="true" />
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
