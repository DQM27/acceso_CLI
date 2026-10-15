import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsuarioSesion } from "../api";
import MenuUsuario from "./MenuUsuario";
import type { Tema } from "./SelectorTema.logica";

const sesion: UsuarioSesion = { id: 1, cedula: "100000000", nombre: "Daniel", rol: "Root" };

function montar(
  props: { colapsado?: boolean; tema?: Tema; alternar?: () => void; cerrar?: () => void } = {},
) {
  return render(
    <MenuUsuario
      sesion={sesion}
      colapsado={props.colapsado ?? false}
      tema={props.tema ?? "tokyo-day"}
      onAlternarTema={props.alternar ?? (() => {})}
      onCerrarSesion={props.cerrar ?? (() => {})}
    />,
  );
}

describe("MenuUsuario", () => {
  it("muestra la inicial y el nombre con el menú abierto", () => {
    montar();
    expect(screen.getByText("D")).toBeTruthy();
    expect(screen.getByText("Daniel")).toBeTruthy();
  });

  it("colapsado muestra sólo la inicial, sin el nombre", () => {
    montar({ colapsado: true });
    expect(screen.getByText("D")).toBeTruthy();
    expect(screen.queryByText("Daniel")).toBeNull();
  });

  it("al pulsarlo ofrece tema, cambiar contraseña y cerrar sesión", () => {
    montar();
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    expect(screen.getByText("Tema: Tokyo Night Day")).toBeTruthy();
    expect(screen.getByText("Cambiar contraseña")).toBeTruthy();
    expect(screen.getByText("Cerrar sesión")).toBeTruthy();
  });

  it("la fila del tema dice cuál es el actual y lo cambia al pulsarla", () => {
    const alternar = vi.fn();
    montar({ tema: "tokyo-night", alternar });
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.click(screen.getByText("Tema: Tokyo Night"));
    expect(alternar).toHaveBeenCalledTimes(1);
  });

  it("cerrar sesión llama al callback", () => {
    const cerrar = vi.fn();
    montar({ cerrar });
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.click(screen.getByText("Cerrar sesión"));
    expect(cerrar).toHaveBeenCalledTimes(1);
  });

  it("Escape cierra el menú", () => {
    montar();
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByText("Cerrar sesión")).toBeNull();
  });
});
