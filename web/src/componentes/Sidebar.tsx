import { useRef, useState, type ReactNode } from "react";
import { NavLink } from "react-router-dom";
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, arrayMove, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Seccion } from "../secciones";
import { rutaSeccion } from "../secciones";
import MenuContextualSecciones from "./MenuContextualSecciones";

/** Ventana tras soltar un arrastre en la que el clic que dispara el navegador
 * se ignora: llega enseguida (mismo gesto), un clic de verdad no tan pronto. */
const MS_CLIC_TRAS_ARRASTRE = 250;

interface FilaSeccion {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
}

/** Un solo ítem arrastrable -- `useSortable` maneja tanto la posición
 * durante el arrastre como los listeners de puntero. El `PointerSensor` con
 * `activationConstraint` (ver más abajo) es lo que distingue un click normal
 * de un arrastre: sin mover el mouse más de esa distancia, el click sigue
 * navegando normal. */
function ItemSidebar({
  seccion,
  colapsado,
  onNavegar,
  recienArrastrado,
}: {
  seccion: FilaSeccion;
  colapsado: boolean;
  onNavegar: () => void;
  /** `true` si un arrastre acaba de terminar: el clic que el navegador
   * dispara al soltar no es una navegación. */
  recienArrastrado: () => boolean;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: seccion.id,
  });

  return (
    <NavLink
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      // dnd-kit lo marca como "button"; acá es un enlace de navegación.
      role={undefined}
      // Sin el arrastre nativo del navegador: un enlace se puede arrastrar, y
      // al soltarlo dentro de la página el navegador abría esa dirección
      // (recargaba el panel entero) en vez de dejar que dnd-kit reordene.
      draggable={false}
      to={rutaSeccion(seccion.id)}
      onClick={(evento) => {
        // Al soltar, el puntero queda sobre el ítem movido y el navegador
        // dispara un clic: sin esto, reordenar también llevaba a esa sección.
        if (recienArrastrado()) {
          evento.preventDefault();
          return;
        }
        onNavegar();
      }}
      title={colapsado ? seccion.etiqueta : undefined}
      className={({ isActive }) => `nav-item ${isActive ? "nav-item-activo" : ""}`}
      // Sólo la transformación del arrastre es dinámica.
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.4 : 1,
      }}
      // React Router v7 dispara la View Transitions API nativa del
      // navegador en esta navegación -- react-router (no un
      // `<ViewTransition>` de React) porque acá, a diferencia de
      // desktop, el cambio de sección lo decide la URL.
      viewTransition
    >
      <seccion.Icono size={18} strokeWidth={2} aria-hidden="true" />
      <span className="nav-item-etiqueta">{seccion.etiqueta}</span>
    </NavLink>
  );
}

/**
 * Sidebar izquierdo de `Shell` (`App.tsx`) -- navegación de secciones,
 * reordenable por drag-and-drop y con mostrar/ocultar por click derecho,
 * igual que el de escritorio (`desktop/src/componentes/Sidebar.tsx`), salvo
 * que acá la sección activa la decide la URL (`NavLink`) y no hay pie con la
 * versión (el panel no tiene versión instalada). Arriba, el botón de
 * colapsar/expandir a la altura de la barra de herramientas. La cuenta (tema y
 * "Cerrar sesión") vive en `MenuUsuario`, al fondo del menú (`pie`).
 *
 * Sin estado propio de negocio: colapsado, orden y ocultas viven en `Shell`;
 * acá sólo el popover del menú contextual (visual y efímero).
 */
export default function Sidebar({
  secciones,
  ocultas,
  onNavegar,
  colapsado,
  onToggleColapsado,
  onReordenar,
  onCambiarVisibilidad,
  onRestablecer,
  abiertoEnMovil,
  pie,
}: {
  /** Todas las secciones, en el orden guardado -- incluye las ocultas
   * (Sidebar filtra internamente qué renderiza; el menú contextual necesita
   * la lista completa para poder volver a mostrar una oculta). */
  secciones: FilaSeccion[];
  ocultas: Seccion[];
  /** Se dispara al elegir una sección -- hoy sólo usado para cerrar el cajón
   * en mobile; cuál queda activa la decide el propio `NavLink`. */
  onNavegar: () => void;
  colapsado: boolean;
  onToggleColapsado: () => void;
  onReordenar: (orden: Seccion[]) => void;
  onCambiarVisibilidad: (id: Seccion, visible: boolean) => void;
  onRestablecer: () => void;
  /** Cajón abierto en pantallas angostas (ver `.shell-columna-abierta` en
   * index.css) -- independiente de `colapsado`, que solo aplica en
   * escritorio. */
  abiertoEnMovil: boolean;
  /** Lo que va al fondo del menú (la cuenta de quien tiene la sesión, como en
   * escritorio). El Sidebar sólo navega: no sabe qué es. */
  pie?: ReactNode;
}) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  // Cuándo terminó el último arrastre (ver `recienArrastrado` en `ItemSidebar`).
  const finArrastre = useRef(0);
  const recienArrastrado = () => performance.now() - finArrastre.current < MS_CLIC_TRAS_ARRASTRE;

  const visibles = secciones.filter((seccion) => !ocultas.includes(seccion.id));

  function alSoltar(evento: DragEndEvent) {
    finArrastre.current = performance.now();
    const { active, over } = evento;
    if (!over || active.id === over.id) return;
    // Índices contra la lista COMPLETA (no sólo visibles) -- así una
    // sección oculta conserva su posición relativa si se vuelve a mostrar
    // después, en vez de saltar siempre al final.
    const ids = secciones.map((seccion) => seccion.id);
    const desde = ids.indexOf(active.id as Seccion);
    const hasta = ids.indexOf(over.id as Seccion);
    if (desde === -1 || hasta === -1) return;
    onReordenar(arrayMove(ids, desde, hasta));
  }

  return (
    <div
      className={`shell-columna-lateral ${colapsado ? "shell-columna-lateral-colapsada" : ""} ${abiertoEnMovil ? "shell-columna-abierta" : ""}`}
    >
      <div className="shell-cabecera-lateral">
        {/* Con el menú abierto, el nombre de la app ocupa el espacio que
            sobraba al lado del botón. Texto y no imagen: a este alto, en
            tema oscuro, un logo con fondo claro queda como un cuadro blanco.
            Letra por letra: `.shell-marca` las reparte a lo ancho, así el
            nombre llega hasta el botón sea cual sea el ancho del menú. */}
        {!colapsado && (
          <span className="shell-marca" role="img" aria-label="Lattis">
            {"LATTIS".split("").map((letra, indice) => (
              <span key={indice} aria-hidden="true">
                {letra}
              </span>
            ))}
          </span>
        )}
        <button
          type="button"
          className="boton boton-icono"
          title={colapsado ? "Expandir menú" : "Colapsar menú"}
          aria-label={colapsado ? "Expandir menú" : "Colapsar menú"}
          aria-expanded={!colapsado}
          onClick={onToggleColapsado}
        >
          {colapsado ? (
            <PanelLeftOpen size={16} aria-hidden="true" />
          ) : (
            <PanelLeftClose size={16} aria-hidden="true" />
          )}
        </button>
      </div>

      <nav
        className={`shell-sidebar ${colapsado ? "shell-sidebar-colapsada" : ""}`}
        onContextMenu={(evento) => {
          evento.preventDefault();
          setMenu({ x: evento.clientX, y: evento.clientY });
        }}
      >
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={alSoltar}
          onDragCancel={() => {
            finArrastre.current = performance.now();
          }}
        >
          <SortableContext items={visibles.map((seccion) => seccion.id)} strategy={verticalListSortingStrategy}>
            <div className="shell-nav">
              {visibles.map((seccion) => (
                <ItemSidebar
                  key={seccion.id}
                  seccion={seccion}
                  colapsado={colapsado}
                  onNavegar={onNavegar}
                  recienArrastrado={recienArrastrado}
                />
              ))}
            </div>
          </SortableContext>
        </DndContext>

        <div className="flex-1" title="Click derecho para mostrar/ocultar secciones" />

        {pie}

        {menu && (
          <MenuContextualSecciones
            posicion={menu}
            secciones={secciones}
            ocultas={ocultas}
            onCambiarVisibilidad={onCambiarVisibilidad}
            onRestablecer={onRestablecer}
            onCerrar={() => setMenu(null)}
          />
        )}
      </nav>
    </div>
  );
}
