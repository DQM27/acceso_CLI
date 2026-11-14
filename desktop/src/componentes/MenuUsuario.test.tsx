import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsuarioSesion } from "../api";
import MenuUsuario from "./MenuUsuario";
import type { Tema } from "./SelectorTema.logica";

const sesion: UsuarioSesion = { id: 1, cedula: "100000000", nombre: "Daniel", rol: "Root" };

function montar(
  props: {
    colapsado?: boolean;
    tema?: Tema;
    alternar?: () => void;
    cerrar?: () => void;
    personalizar?: (posicion: { x: number; y: number }) => void;
    buscar?: () => void;
  } = {},
) {
  return render(
    <MenuUsuario
      sesion={sesion}
      colapsado={props.colapsado ?? false}
      tema={props.tema ?? "tokyo-day"}
      onAlternarTema={props.alternar ?? (() => {})}
      onCerrarSesion={props.cerrar ?? (() => {})}
      onPersonalizarMenu={props.personalizar ?? (() => {})}
      onBuscarActualizacion={props.buscar ?? (() => {})}
      buscandoActualizacion={false}
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
    expect(screen.getByText("Tema: Claro")).toBeTruthy();
    expect(screen.getByText("Cambiar contraseña")).toBeTruthy();
    expect(screen.getByText("Cerrar sesión")).toBeTruthy();
  });

  it("la fila del tema dice cuál es el actual y lo cambia al pulsarla", () => {
    const alternar = vi.fn();
    montar({ tema: "tokyo-night", alternar });
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.click(screen.getByText("Tema: Oscuro"));
    expect(alternar).toHaveBeenCalledTimes(1);
  });

  it("cerrar sesión llama al callback", () => {
    const cerrar = vi.fn();
    montar({ cerrar });
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.click(screen.getByText("Cerrar sesión"));
    expect(cerrar).toHaveBeenCalledTimes(1);
  });

  it("Escape cierra el menú después de la animación de salida", async () => {
    montar();
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() => expect(screen.queryByText("Cerrar sesión")).toBeNull());
  });

  it("«Personalizar menú» abre el menú de secciones y cierra el de la cuenta", async () => {
    const personalizar = vi.fn();
    montar({ personalizar });
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.click(screen.getByText("Personalizar menú"));
    expect(personalizar).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByText("Cerrar sesión")).toBeNull());
  });

  it("«Buscar actualización» está en el menú de la cuenta", () => {
    const buscar = vi.fn();
    montar({ buscar });
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    fireEvent.click(screen.getByText(/Buscar actualización/));
    expect(buscar).toHaveBeenCalledTimes(1);
  });
});
