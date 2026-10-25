import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import Ayuda from "./Ayuda";

beforeAll(() => {
  // jsdom no hace scroll.
  Element.prototype.scrollIntoView = vi.fn();
  Element.prototype.scrollTo = vi.fn() as unknown as typeof Element.prototype.scrollTo;
});

describe("Ayuda", () => {
  it("arranca con el índice del escritorio y el foco en el buscador", () => {
    render(<Ayuda senalBuscar={0} />);
    expect(screen.getByRole("heading", { name: "Ayuda" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Registrar una salida" })).toBeTruthy();
    // Lo del panel no se ofrece en el puesto de control.
    expect(screen.queryByRole("button", { name: /Usuarios del puesto de control/ })).toBeNull();
    expect(document.activeElement).toBe(screen.getByLabelText("Buscar en la ayuda"));
  });

  it("busca un mensaje pegado y abre el proceso en su sección", () => {
    render(<Ayuda senalBuscar={0} />);
    fireEvent.change(screen.getByLabelText("Buscar en la ayuda"), {
      target: { value: "Este encargado ya tiene un gafete provisional prestado" },
    });
    const resultado = screen.getAllByRole("button", { name: /Entregar un gafete provisional KOF/ })[0];
    fireEvent.click(resultado);
    expect(screen.getByRole("heading", { level: 1, name: /Entregar un gafete provisional KOF/ })).toBeTruthy();
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  });

  it("Enter abre el primer resultado y Esc limpia la búsqueda", () => {
    render(<Ayuda senalBuscar={0} />);
    const buscador = screen.getByLabelText("Buscar en la ayuda");
    fireEvent.change(buscador, { target: { value: "registrar salida" } });
    fireEvent.keyDown(buscador, { key: "Enter" });
    expect(screen.getByRole("heading", { level: 1 })).toBeTruthy();
    fireEvent.keyDown(buscador, { key: "Escape" });
    expect((buscador as HTMLInputElement).value).toBe("");
  });

  it("sigue los enlaces entre procesos y vuelve con Atrás", () => {
    render(<Ayuda senalBuscar={0} />);
    fireEvent.click(screen.getByRole("button", { name: "Registrar una salida" }));
    expect(screen.queryByRole("button", { name: "Atrás" })).toBeNull();
    fireEvent.click(screen.getByRole("link", { name: "Registrar el ingreso de un contratista" }));
    expect(screen.getByRole("heading", { level: 1, name: /ingreso de un contratista/i })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Atrás" }));
    expect(screen.getByRole("heading", { level: 1, name: "Registrar una salida" })).toBeTruthy();
  });

  it("sin resultados lo dice", () => {
    render(<Ayuda senalBuscar={0} />);
    fireEvent.change(screen.getByLabelText("Buscar en la ayuda"), { target: { value: "tractor helicóptero" } });
    expect(screen.getByText(/No hay nada con esas palabras/)).toBeTruthy();
  });
});
