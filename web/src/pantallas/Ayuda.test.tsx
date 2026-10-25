import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import Ayuda from "./Ayuda";

beforeAll(() => {
  // jsdom no hace scroll.
  Element.prototype.scrollIntoView = vi.fn();
  Element.prototype.scrollTo = vi.fn() as unknown as typeof Element.prototype.scrollTo;
});

describe("Ayuda (panel)", () => {
  it("arranca con el índice del panel y deja cambiar de app", () => {
    render(<Ayuda />);
    expect(screen.getByRole("button", { name: "Panel web" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: /Usuarios del puesto de control/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Móvil" }));
    expect(screen.getByRole("button", { name: /Vincular el teléfono/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Usuarios del puesto de control/ })).toBeNull();
  });

  it("busca en las tres apps y dice de cuál es cada resultado", () => {
    render(<Ayuda />);
    fireEvent.change(screen.getByLabelText("Buscar en la ayuda"), { target: { value: "vincular" } });
    const apps = screen.getAllByText(/^(Panel web|Escritorio|Móvil)( · |$)/).map((n) => n.textContent?.split(" · ")[0]);
    expect(new Set(apps)).toEqual(new Set(["Escritorio", "Móvil", "Panel web"]));
  });

  it("al abrir un tema de otra app, el índice pasa a esa app; Atrás vuelve", () => {
    render(<Ayuda />);
    fireEvent.click(screen.getByRole("button", { name: /Dispositivos/ }));
    expect(screen.getByRole("heading", { level: 1, name: /Dispositivos/ })).toBeTruthy();
    const enlaceAOtraApp = screen
      .getAllByRole("link")
      .find((enlace) => enlace.getAttribute("href")?.startsWith("#escritorio/"));
    if (!enlaceAOtraApp) throw new Error("Dispositivos ya no enlaza al escritorio");
    fireEvent.click(enlaceAOtraApp);
    expect(screen.getByRole("button", { name: "Escritorio" }).getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "Atrás" }));
    expect(screen.getByRole("heading", { level: 1, name: /Dispositivos/ })).toBeTruthy();
  });
});
