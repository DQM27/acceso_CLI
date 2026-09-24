import { useState } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, arrayMove, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Seccion } from "../App";
import MenuContextualSecciones from "./MenuContextualSecciones";
import VersionFooter from "./VersionFooter";

interface FilaSeccion {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
}

/** Un solo ítem arrastrable -- `useSortable` maneja tanto la posición
 * durante el arrastre como los listeners de puntero. El `PointerSensor` con
 * `activationConstraint` (ver más abajo) es lo que distingue un click normal
 * de un arrastre: sin mover el mouse más de esa distancia, el `onClick`
 * sigue disparando normal. */
function ItemSidebar({
  seccion,
  activa,
  colapsado,
  onClick,
}: {
  seccion: FilaSeccion;
  activa: boolean;
  colapsado: boolean;
  onClick: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: seccion.id,
  });

  return (
    <button
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      onClick={onClick}
      title={colapsado ? seccion.etiqueta : undefined}
      className={`nav-item ${activa ? "nav-item-activo" : ""}`}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.4 : 1,
      }}
    >
      <seccion.Icono size={18} strokeWidth={2} aria-hidden="true" />
      {!colapsado && seccion.etiqueta}
    </button>
  );
}

/**
 * Sidebar izquierdo de `Shell` (`App.tsx`) — navegación de secciones,
 * reordenable por drag-and-drop y con mostrar/ocultar por click derecho —
 * mismo comportamiento "orgánico" que la barra de actividad de VS Code
 * (arrastrar el ícono mismo, sin un modal aparte; click derecho para el
 * checklist de qué se ve). Usuario y "Cerrar sesión" viven en `MenuUsuario`,
 * en la barra de estado (ver `Shell`) — no acá. Sin estado propio de
 * negocio: `seccion`/`colapsado`/orden/ocultas siguen viviendo en `Shell`,
 * acá sólo el popover del menú contextual (puramente visual, efímero).
 */
export default function Sidebar({
  secciones,
  ocultas,
  seccionActual,
  onCambiarSeccion,
  colapsado,
  onToggleColapsado,
  onReordenar,
  onCambiarVisibilidad,
  onRestablecer,
}: {
  /** Todas las secciones, en el orden guardado -- incluye las ocultas
   * (Sidebar filtra internamente qué renderiza; el menú contextual necesita
   * la lista completa para poder volver a mostrar una oculta). */
  secciones: FilaSeccion[];
  ocultas: Seccion[];
  seccionActual: Seccion;
  onCambiarSeccion: (id: Seccion) => void;
  colapsado: boolean;
  onToggleColapsado: () => void;
  onReordenar: (orden: Seccion[]) => void;
  onCambiarVisibilidad: (id: Seccion, visible: boolean) => void;
  onRestablecer: () => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
  );
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);

  const visibles = secciones.filter((seccion) => !ocultas.includes(seccion.id));

  function alSoltar(evento: DragEndEvent) {
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
    // Columna lateral: arriba el botón de colapsar/expandir, a la altura
    // de la barra de herramientas; abajo la tarjeta del menú, que arranca a
    // la altura de la grilla (pedido del usuario 2026-09-23). Antes el
    // menú se colapsaba con doble clic en su espacio vacío, algo que nadie
    // descubría solo.
    <div className={`shell-columna-lateral ${colapsado ? "shell-columna-lateral-colapsada" : ""}`}>
      <div className="shell-cabecera-lateral">
        {/* Con el menú abierto, el nombre de la app ocupa el espacio que
            sobraba al lado del botón (elegido por el usuario 2026-09-23).
            Texto y no `marca.png`: la imagen tiene fondo blanco y a este
            alto, en tema oscuro, quedaba un cuadro blanco con letras
            diminutas. */}
        {!colapsado && <span className="shell-marca">Lattis</span>}
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
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={alSoltar}>
        <SortableContext
          items={visibles.map((seccion) => seccion.id)}
          strategy={verticalListSortingStrategy}
        >
          <div className="shell-nav">
            {visibles.map((seccion) => (
              <ItemSidebar
                key={seccion.id}
                seccion={seccion}
                activa={seccionActual === seccion.id}
                colapsado={colapsado}
                onClick={() => onCambiarSeccion(seccion.id)}
              />
            ))}
          </div>
        </SortableContext>
      </DndContext>

      <div style={{ flex: 1 }} title="Click derecho para mostrar/ocultar secciones" />

      <VersionFooter />

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
