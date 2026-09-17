import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Modal from "../componentes/Modal";
import type { Seccion } from "../App";

interface FilaSeccion {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
}

/** Fila arrastrable de una sección -- `useSortable` es lo único de dnd-kit
 * que necesita vivir por fila, el resto (sensores, contexto, colisión) va
 * una sola vez en el modal. */
function FilaArrastrable({
  fila,
  visible,
  bloqueado,
  onCambiarVisibilidad,
}: {
  fila: FilaSeccion;
  visible: boolean;
  /** `true` cuando esta es la última sección visible -- no se puede ocultar
   * sin dejar el sidebar vacío. */
  bloqueado: boolean;
  onCambiarVisibilidad: (visible: boolean) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: fila.id,
  });

  return (
    <div
      ref={setNodeRef}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.5 : 1,
        display: "flex",
        alignItems: "center",
        gap: "0.6rem",
        padding: "0.4rem 0.5rem",
        border: "1px solid var(--borde)",
        borderRadius: "var(--radio-chico)",
        background: "var(--campo-fondo)",
      }}
    >
      <button
        type="button"
        {...attributes}
        {...listeners}
        style={{
          cursor: "grab",
          display: "flex",
          background: "none",
          border: "none",
          padding: 0,
          color: "var(--muted)",
        }}
        aria-label={`Reordenar ${fila.etiqueta}`}
      >
        <GripVertical size={16} aria-hidden="true" />
      </button>
      <fila.Icono size={16} strokeWidth={2} aria-hidden="true" />
      <span style={{ flex: 1 }}>{fila.etiqueta}</span>
      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "0.35rem",
          fontSize: "0.8rem",
          color: "var(--muted)",
        }}
        title={bloqueado ? "No se puede ocultar la última sección visible" : undefined}
      >
        <input
          type="checkbox"
          checked={visible}
          disabled={bloqueado}
          onChange={(evento) => onCambiarVisibilidad(evento.target.checked)}
        />
        Visible
      </label>
    </div>
  );
}

/**
 * Personalización del sidebar (`docs/decisiones-tecnicas.md`, sidebar
 * "inteligente"): mostrar/ocultar secciones y reordenarlas por
 * drag-and-drop, mismo espíritu que la barra de actividad de VS Code. Cada
 * cambio se aplica de inmediato vía las callbacks (`Shell` ya persiste a
 * `localStorage`) -- no hay un botón "Guardar" aparte, "Cerrar" sólo cierra
 * el diálogo.
 */
export default function PersonalizarSidebarModal({
  secciones,
  ocultas,
  onReordenar,
  onCambiarVisibilidad,
  onRestablecer,
  onCerrar,
}: {
  /** Todas las secciones, ya en el orden guardado -- incluye las ocultas,
   * para poder volver a mostrarlas acá. */
  secciones: FilaSeccion[];
  ocultas: Seccion[];
  onReordenar: (orden: Seccion[]) => void;
  onCambiarVisibilidad: (id: Seccion, visible: boolean) => void;
  onRestablecer: () => void;
  onCerrar: () => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
  );
  const visiblesCount = secciones.length - ocultas.length;

  function alSoltar(evento: DragEndEvent) {
    const { active, over } = evento;
    if (!over || active.id === over.id) return;
    const ids = secciones.map((seccion) => seccion.id);
    const desde = ids.indexOf(active.id as Seccion);
    const hasta = ids.indexOf(over.id as Seccion);
    if (desde === -1 || hasta === -1) return;
    onReordenar(arrayMove(ids, desde, hasta));
  }

  return (
    <Modal titulo="Personalizar barra lateral" onCerrar={onCerrar}>
      <div
        style={{ display: "flex", flexDirection: "column", gap: "0.65rem", width: "22rem", maxWidth: "100%" }}
      >
        <p style={{ margin: 0, fontSize: "0.85rem", color: "var(--muted)" }}>
          Arrastre para reordenar. Desmarque para ocultar una sección del menú.
        </p>

        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={alSoltar}>
          <SortableContext
            items={secciones.map((seccion) => seccion.id)}
            strategy={verticalListSortingStrategy}
          >
            <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
              {secciones.map((fila) => {
                const visible = !ocultas.includes(fila.id);
                return (
                  <FilaArrastrable
                    key={fila.id}
                    fila={fila}
                    visible={visible}
                    bloqueado={visible && visiblesCount <= 1}
                    onCambiarVisibilidad={(nuevoVisible) =>
                      onCambiarVisibilidad(fila.id, nuevoVisible)
                    }
                  />
                );
              })}
            </div>
          </SortableContext>
        </DndContext>

        <div style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onRestablecer}>
            Restablecer
          </button>
          <button type="button" className="boton boton-primario" onClick={onCerrar}>
            Cerrar
          </button>
        </div>
      </div>
    </Modal>
  );
}
