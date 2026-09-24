import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CalendarDays, Users } from "lucide-react";
import Sidebar from "./Sidebar";
import type { Seccion } from "../App";

const secciones: { id: Seccion; etiqueta: string; Icono: typeof CalendarDays }[] = [
  { id: "activos", etiqueta: "Activos", Icono: CalendarDays },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
];

function renderSidebar(props: Partial<React.ComponentProps<typeof Sidebar>> = {}) {
  return render(
    <Sidebar
      secciones={secciones}
      ocultas={[]}
      seccionActual="activos"
      onCambiarSeccion={() => {}}
      colapsado={false}
      onToggleColapsado={() => {}}
      onReordenar={() => {}}
      onCambiarVisibilidad={() => {}}
      onRestablecer={() => {}}
      {...props}
    />,
  );
}

describe("Sidebar", () => {
  it("el botón de arriba colapsa o expande el menú según el estado", () => {
    const onToggleColapsado = vi.fn();
    const { rerender } = renderSidebar({ onToggleColapsado });

    fireEvent.click(screen.getByLabelText("Colapsar menú"));
    expect(onToggleColapsado).toHaveBeenCalledTimes(1);

    rerender(
      <Sidebar
        secciones={secciones}
        ocultas={[]}
        seccionActual="activos"
        onCambiarSeccion={() => {}}
        colapsado
        onToggleColapsado={onToggleColapsado}
        onReordenar={() => {}}
        onCambiarVisibilidad={() => {}}
        onRestablecer={() => {}}
      />,
    );
    fireEvent.click(screen.getByLabelText("Expandir menú"));
    expect(onToggleColapsado).toHaveBeenCalledTimes(2);
  });

  it("al hacer click en una sección, llama a onCambiarSeccion con su id", () => {
    const onCambiarSeccion = vi.fn();
    renderSidebar({ onCambiarSeccion });
    fireEvent.click(screen.getByText("Contratistas"));
    expect(onCambiarSeccion).toHaveBeenCalledWith("contratistas");
  });

  it("marca como activa sólo la sección actual", () => {
    renderSidebar({ seccionActual: "contratistas" });
    expect(screen.getByText("Contratistas").closest("button")?.className).toContain(
      "nav-item-activo",
    );
    expect(screen.getByText("Activos").closest("button")?.className).not.toContain(
      "nav-item-activo",
    );
  });

  it("colapsado oculta las etiquetas de texto", () => {
    renderSidebar({ colapsado: true });
    expect(screen.queryByText("Activos")).toBeNull();
  });

  it("una sección oculta no se renderiza en la lista", () => {
    renderSidebar({ ocultas: ["contratistas"] });
    expect(screen.queryByText("Contratistas")).toBeNull();
    expect(screen.getByText("Activos")).toBeTruthy();
  });

  it("click derecho abre el menú contextual con todas las secciones", () => {
    renderSidebar({ ocultas: ["contratistas"] });
    fireEvent.contextMenu(screen.getByText("Activos"));
    // "Activos" aparece dos veces (el ítem del sidebar + la fila del menú);
    // "Contratistas" sólo en el menú -- el sidebar no la renderiza porque
    // está oculta, a diferencia del menú, que sí muestra TODAS las
    // secciones para poder volver a mostrarla.
    expect(screen.getAllByText("Activos")).toHaveLength(2);
    expect(screen.getByText("Contratistas")).toBeTruthy();
  });

  it("elegir una sección en el menú contextual llama a onCambiarVisibilidad", () => {
    const onCambiarVisibilidad = vi.fn();
    renderSidebar({ ocultas: ["contratistas"], onCambiarVisibilidad });
    fireEvent.contextMenu(screen.getByText("Activos"));
    fireEvent.click(screen.getByText("Contratistas"));
    expect(onCambiarVisibilidad).toHaveBeenCalledWith("contratistas", true);
  });
});
