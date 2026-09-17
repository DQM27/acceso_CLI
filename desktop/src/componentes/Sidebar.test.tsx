import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CalendarDays, Users } from "lucide-react";
import Sidebar from "./Sidebar";
import type { Seccion } from "../App";

const secciones: { id: Seccion; etiqueta: string; Icono: typeof CalendarDays }[] = [
  { id: "activos", etiqueta: "Activos", Icono: CalendarDays },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
];

describe("Sidebar", () => {
  it("al hacer click en una sección, llama a onCambiarSeccion con su id", () => {
    const onCambiarSeccion = vi.fn();
    render(
      <Sidebar
        secciones={secciones}
        seccionActual="activos"
        onCambiarSeccion={onCambiarSeccion}
        colapsado={false}
        onToggleColapsado={() => {}}
        onAbrirPersonalizar={() => {}}
      />,
    );
    fireEvent.click(screen.getByText("Contratistas"));
    expect(onCambiarSeccion).toHaveBeenCalledWith("contratistas");
  });

  it("marca como activa sólo la sección actual", () => {
    render(
      <Sidebar
        secciones={secciones}
        seccionActual="contratistas"
        onCambiarSeccion={() => {}}
        colapsado={false}
        onToggleColapsado={() => {}}
        onAbrirPersonalizar={() => {}}
      />,
    );
    expect(screen.getByText("Contratistas").closest("button")?.className).toContain(
      "nav-item-activo",
    );
    expect(screen.getByText("Activos").closest("button")?.className).not.toContain(
      "nav-item-activo",
    );
  });

  it("el botón Personalizar llama a onAbrirPersonalizar", () => {
    const onAbrirPersonalizar = vi.fn();
    render(
      <Sidebar
        secciones={secciones}
        seccionActual="activos"
        onCambiarSeccion={() => {}}
        colapsado={false}
        onToggleColapsado={() => {}}
        onAbrirPersonalizar={onAbrirPersonalizar}
      />,
    );
    fireEvent.click(screen.getByText("Personalizar"));
    expect(onAbrirPersonalizar).toHaveBeenCalled();
  });

  it("colapsado oculta las etiquetas de texto", () => {
    render(
      <Sidebar
        secciones={secciones}
        seccionActual="activos"
        onCambiarSeccion={() => {}}
        colapsado={true}
        onToggleColapsado={() => {}}
        onAbrirPersonalizar={() => {}}
      />,
    );
    expect(screen.queryByText("Activos")).toBeNull();
  });
});
