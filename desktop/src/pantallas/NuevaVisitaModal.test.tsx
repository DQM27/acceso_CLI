import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("../api/invocar", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import NuevaVisitaModal from "./NuevaVisitaModal";

/** Respuestas del núcleo según el comando: el check-in falla con el
 * rechazo que se le pase; las listas vienen vacías. */
function nucleo(rechazo: Record<string, unknown>) {
  invoke.mockImplementation((comando: string) => {
    if (comando === "verificar_check_in_visita") return Promise.reject(rechazo);
    return Promise.resolve([]);
  });
}

beforeEach(() => invoke.mockReset());

describe("NuevaVisitaModal", () => {
  it("sin cita para hoy ofrece registrarla por correo con la cédula ya escrita", async () => {
    nucleo({ mensaje: "No hay ninguna visita agendada para esta cédula", informativo: false, alternativa_por_correo: true });
    render(<NuevaVisitaModal cedulaInicial="155824395105" onRegistrado={() => {}} onCerrar={() => {}} />);

    // Desde "Esperadas" la cédula se verifica sola al abrir.
    expect(await screen.findByText("No hay ninguna visita agendada para esta cédula")).toBeTruthy();
    expect(invoke).toHaveBeenCalledWith("verificar_check_in_visita", { cedula: "155824395105" });

    fireEvent.click(screen.getByRole("button", { name: "Registrar como autorizada por correo" }));
    expect(screen.getByText("Autorizada por correo")).toBeTruthy();
    expect((screen.getByLabelText("Cédula") as HTMLInputElement).value).toBe("155824395105");
  });

  it("una visita para otro día se ve como aviso y también permite el registro por correo", async () => {
    nucleo({ mensaje: "Tiene una visita agendada para el martes 6 de octubre", informativo: true, alternativa_por_correo: true });
    render(<NuevaVisitaModal cedulaInicial="155824395105" onRegistrado={() => {}} onCerrar={() => {}} />);

    const aviso = await screen.findByText("Tiene una visita agendada para el martes 6 de octubre");
    expect(aviso.className).toBe("aviso-informativo");
    expect(screen.getByRole("button", { name: "Registrar como autorizada por correo" })).toBeTruthy();
  });

  it("si la visita fue cancelada o la persona tiene el acceso negado, no ofrece el registro por correo", async () => {
    nucleo({ mensaje: "La visita fue cancelada (anfitrión: Daniel).", informativo: false, alternativa_por_correo: false });
    render(<NuevaVisitaModal cedulaInicial="100" onRegistrado={() => {}} onCerrar={() => {}} />);

    const error = await screen.findByText("La visita fue cancelada (anfitrión: Daniel).");
    expect(error.className).toBe("login-error");
    expect(screen.queryByRole("button", { name: "Registrar como autorizada por correo" })).toBeNull();
  });

  it("el interruptor pasa al formulario por correo y lo exige completo", async () => {
    nucleo({ mensaje: "x", informativo: false, alternativa_por_correo: false });
    render(<NuevaVisitaModal onRegistrado={() => {}} onCerrar={() => {}} />);

    fireEvent.click(screen.getByRole("button", { name: "Autorizada por correo" }));
    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "200" } });
    fireEvent.click(screen.getByRole("button", { name: "Registrar entrada" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toBe("El nombre es obligatorio"));
    expect(invoke).not.toHaveBeenCalledWith("registrar_ingreso_correo", expect.anything());
  });
});

describe("FormularioPorCorreo: medio de ingreso", () => {
  it("caminando no pide placa; vehículo la exige", async () => {
    nucleo({ mensaje: "x", informativo: false, alternativa_por_correo: false });
    render(<NuevaVisitaModal onRegistrado={() => {}} onCerrar={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Autorizada por correo" }));

    expect(screen.queryByLabelText("Placa del vehículo")).toBeNull();
    fireEvent.change(screen.getByLabelText("Cédula"), { target: { value: "200" } });
    fireEvent.change(screen.getByLabelText("Nombre"), { target: { value: "LUIS" } });
    fireEvent.change(screen.getByLabelText("A quién visita y quién lo autorizó"), { target: { value: "RH" } });
    fireEvent.click(screen.getByLabelText("Vehículo"));
    fireEvent.click(screen.getByRole("button", { name: "Registrar entrada" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toBe("Escriba la placa del vehículo"));
  });
});
