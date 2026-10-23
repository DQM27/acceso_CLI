import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import TablaRica from "./TablaRica";
import type { Fila } from "./calculo";

const contratistas: Fila[] = [
  { nombre: "Ana Mora", empresa: "ALDAMA", estado_praind: "VENCIDA", dias_para_vencer: -40 },
  { nombre: "Beto Solís", empresa: "ALDAMA", estado_praind: "POR_VENCER", dias_para_vencer: 5 },
  { nombre: "Cris Vega", empresa: "BRISAS", estado_praind: "VIGENTE", dias_para_vencer: 200 },
];

const muchas: Fila[] = Array.from({ length: 12 }, (_, i) => ({ nombre: `Persona ${i}`, empresa: i % 2 ? "ALDAMA" : "Sánchez y Cía" }));

describe("TablaRica", () => {
  it("el estado PRAIND y los días para vencer salen como etiqueta con texto (nunca un número negativo)", async () => {
    render(
      <TablaRica
        origen="contratistas"
        widget={{ tipo: "tabla", columnas: ["nombre", "estado_praind", "dias_para_vencer"] }}
        filas={contratistas}
      />,
    );
    expect(await screen.findByText("Vencido hace 40 días")).toBeTruthy();
    const vencida = screen.getByText("Vencida");
    expect(vencida.closest(".lienzo-estado-error")).toBeTruthy();
    expect(screen.getByText("Por vencer").closest(".lienzo-estado-advertencia")).toBeTruthy();
    expect(screen.queryByText(/-40/)).toBeNull();
    expect(screen.getByText("3 filas.")).toBeTruthy();
  });

  it("agrupada: cada grupo con su cantidad y su barra; se abre para ver sus filas y se vuelve a cerrar", async () => {
    const { container } = render(
      <TablaRica
        origen="contratistas"
        widget={{ tipo: "tabla", columnas: ["empresa", "nombre"], agrupar_por: "empresa" }}
        filas={contratistas}
      />,
    );
    const aldama = await screen.findByRole("button", { name: "Ver las filas de ALDAMA" });
    expect(screen.queryByText("Ana Mora")).toBeNull();
    expect(container.querySelectorAll(".lienzo-celda-barra").length).toBeGreaterThan(0);
    expect(screen.getByText("2 grupos.")).toBeTruthy();

    fireEvent.click(aldama);
    expect(await screen.findByText("Ana Mora")).toBeTruthy();
    expect(screen.getByText("Beto Solís")).toBeTruthy();
    expect(screen.queryByText("Cris Vega")).toBeNull();

    fireEvent.click(await screen.findByRole("button", { name: "Cerrar las filas de ALDAMA" }));
    await screen.findByRole("button", { name: "Ver las filas de ALDAMA" });
    // AG Grid quita las filas con una animación corta.
    await waitFor(() => expect(screen.queryByText("Ana Mora")).toBeNull());
  });

  it("con muchas filas aparece la búsqueda: sin tildes, y avisa si nada coincide", async () => {
    render(<TablaRica origen="contratistas" widget={{ tipo: "tabla", columnas: ["nombre", "empresa"] }} filas={muchas} />);
    const buscar = screen.getByRole("searchbox", { name: "Buscar en la tabla" });
    fireEvent.change(buscar, { target: { value: "sanchez" } });
    expect(await screen.findByText("6 filas coinciden.")).toBeTruthy();
    fireEvent.change(buscar, { target: { value: "zzz" } });
    expect(await screen.findByText("Nada coincide con «zzz».")).toBeTruthy();
  });

  it("con pocas filas no hay búsqueda, y sin filas avisa", () => {
    render(<TablaRica origen="contratistas" widget={{ tipo: "tabla", columnas: ["nombre"] }} filas={[]} />);
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(screen.getByText("Sin datos para mostrar.")).toBeTruthy();
  });
});
