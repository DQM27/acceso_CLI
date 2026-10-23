import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsuarioSesion } from "../api";
import MenuUsuario from "./MenuUsuario";
import type { Tema } from "./useTema";

const sesion: UsuarioSesion = { nombre: "Daniel", correo: "daniel@ejemplo.com" };

function montar(props: { colapsado?: boolean; tema?: Tema; alternar?: () => void; cerrar?: () => void } = {}) {
  return render(
    <MenuUsuario
      sesion={sesion}
      colapsado={props.colapsado ?? false}
      tema={props.tema ?? "light"}
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

  it("al pulsarlo muestra el correo, el tema y cerrar sesión (sin contraseña: el panel no usa)", () => {
    montar();
    fireEvent.click(screen.getByLabelText("Cuenta de Daniel"));
    expect(screen.getByText("daniel@ejemplo.com")).toBeTruthy();
    expect(screen.getByText("Tema: Claro")).toBeTruthy();
    expect(screen.getByText("Cerrar sesión")).toBeTruthy();
    expect(screen.queryByText(/contraseña/i)).toBeNull();
  });

  it("la fila del tema dice cuál es el actual y lo cambia al pulsarla", () => {
    const alternar = vi.fn();
    montar({ tema: "dark", alternar });
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
});
