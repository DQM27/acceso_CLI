import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("../api/invocar", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
const relaunch = vi.fn(() => Promise.resolve());
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: () => relaunch() }));

import ReconectarEquipoModal from "./ReconectarEquipoModal";

beforeEach(() => {
  invoke.mockReset();
  relaunch.mockClear();
});

function escribirCodigo(codigo: string) {
  fireEvent.change(screen.getByLabelText("Código de reconexión"), { target: { value: codigo } });
}

describe("ReconectarEquipoModal", () => {
  it("en la misma unidad avisa a quien la abrió, que sincroniza", async () => {
    invoke.mockResolvedValue({ tipo: "Reconectado", unidad: "Planta Norte" });
    const onReconectado = vi.fn();
    render(<ReconectarEquipoModal onReconectado={onReconectado} onCerrar={() => {}} />);

    escribirCodigo("k7qmr4xt2p");
    fireEvent.click(screen.getByRole("button", { name: "Reconectar" }));

    await waitFor(() => expect(onReconectado).toHaveBeenCalledWith("Planta Norte"));
    expect(invoke).toHaveBeenCalledWith("reconectar_equipo", {
      codigo: "K7QM-R4XT-2P",
      pendientesConfirmados: null,
    });
  });

  it("al cambiar de unidad con pendientes avisa cuántos se pierden y sólo sigue si se confirma", async () => {
    invoke
      .mockResolvedValueOnce({ tipo: "NecesitaConfirmacion", pendientes: 3, unidad: "Bodega Sur" })
      .mockResolvedValueOnce({ tipo: "ReconectadoEnOtraUnidad", unidad: "Bodega Sur" });
    render(<ReconectarEquipoModal onReconectado={() => {}} onCerrar={() => {}} />);

    escribirCodigo("K7QMR4XT2P");
    fireEvent.click(screen.getByRole("button", { name: "Reconectar" }));

    expect(await screen.findByText(/3 registros sin subir/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Reconectar y borrar" }));

    expect(await screen.findByRole("button", { name: "Reiniciar ahora" })).toBeTruthy();
    expect(invoke).toHaveBeenLastCalledWith("reconectar_equipo", {
      codigo: "K7QM-R4XT-2P",
      pendientesConfirmados: 3,
    });
    fireEvent.click(screen.getByRole("button", { name: "Reiniciar ahora" }));
    expect(relaunch).toHaveBeenCalled();
  });

  it("cancelar el aviso de pérdida no reconecta", async () => {
    invoke.mockResolvedValueOnce({ tipo: "NecesitaConfirmacion", pendientes: 1, unidad: "Bodega Sur" });
    const onCerrar = vi.fn();
    render(<ReconectarEquipoModal onReconectado={() => {}} onCerrar={onCerrar} />);

    escribirCodigo("K7QMR4XT2P");
    fireEvent.click(screen.getByRole("button", { name: "Reconectar" }));
    expect(await screen.findByText(/1 registro sin subir/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onCerrar).toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("muestra el mensaje del núcleo si el código no sirve", async () => {
    invoke.mockRejectedValue("El código de reconexión no es válido o ya venció -- pida uno nuevo en el panel");
    render(<ReconectarEquipoModal onReconectado={() => {}} onCerrar={() => {}} />);

    escribirCodigo("K7QMR4XT2P");
    fireEvent.click(screen.getByRole("button", { name: "Reconectar" }));

    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toContain("no es válido o ya venció");
  });

  it("sin el código completo no deja enviar", () => {
    render(<ReconectarEquipoModal onReconectado={() => {}} onCerrar={() => {}} />);
    escribirCodigo("K7QM");
    expect((screen.getByRole("button", { name: "Reconectar" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
