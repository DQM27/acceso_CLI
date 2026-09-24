import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { History, UserCheck } from "lucide-react";
import SegmentadoOpciones from "./SegmentadoOpciones";

const opciones = [
  { valor: "activos" as const, Icono: UserCheck, titulo: "Activos" },
  { valor: "historial" as const, Icono: History, titulo: "Historial" },
];

describe("SegmentadoOpciones", () => {
  it("marca la opción elegida y ubica el indicador deslizante en ella", () => {
    render(
      <SegmentadoOpciones opciones={opciones} valor="historial" onCambiar={() => {}} etiqueta="Vista" />,
    );

    expect(screen.getByLabelText("Historial").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByLabelText("Activos").getAttribute("aria-pressed")).toBe("false");
    const grupo = screen.getByRole("group", { name: "Vista" });
    expect(grupo.style.getPropertyValue("--indice")).toBe("1");
    expect(grupo.style.getPropertyValue("--cantidad")).toBe("2");
  });

  it("avisa la opción tocada", () => {
    const onCambiar = vi.fn();
    render(
      <SegmentadoOpciones opciones={opciones} valor="activos" onCambiar={onCambiar} etiqueta="Vista" />,
    );

    fireEvent.click(screen.getByLabelText("Historial"));

    expect(onCambiar).toHaveBeenCalledWith("historial");
  });

  it("con texto muestra el nombre de cada opción y se elige tocándolo", () => {
    const onCambiar = vi.fn();
    render(
      <SegmentadoOpciones
        opciones={opciones}
        valor="activos"
        onCambiar={onCambiar}
        etiqueta="Vista"
        conTexto
        anchoCompleto
      />,
    );

    fireEvent.click(screen.getByText("Historial"));

    expect(onCambiar).toHaveBeenCalledWith("historial");
    expect(screen.getByRole("group", { name: "Vista" }).style.width).toBe("100%");
  });
});
