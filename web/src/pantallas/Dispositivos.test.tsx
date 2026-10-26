import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Dispositivo, EstadoDispositivos } from "../api/dispositivos";
import Dispositivos from "./Dispositivos";

// Retirar, Eliminar y Autorizar reconexión, con la API de dispositivos y la
// verificación por correo reemplazadas: lo que se prueba es QUÉ acción pide
// código por correo y cuál no (pedido del dueño, 2026-10-26: reconectar no).

const mocks = vi.hoisted(() => ({
  autorizarReconexion: vi.fn(),
  crearSitio: vi.fn(),
  eliminarDispositivo: vi.fn(),
  listarDispositivosYSitios: vi.fn(),
  provisionarDispositivo: vi.fn(),
  revocarDispositivo: vi.fn(),
  recargar: vi.fn(),
  datos: null as EstadoDispositivos | null,
}));
vi.mock("../api/dispositivos", () => ({
  autorizarReconexion: mocks.autorizarReconexion,
  crearSitio: mocks.crearSitio,
  eliminarDispositivo: mocks.eliminarDispositivo,
  listarDispositivosYSitios: mocks.listarDispositivosYSitios,
  provisionarDispositivo: mocks.provisionarDispositivo,
  revocarDispositivo: mocks.revocarDispositivo,
}));
vi.mock("../componentes/useLista", () => ({
  useLista: () => ({ datos: mocks.datos, cargando: false, recargar: mocks.recargar }),
}));
vi.mock("../presenciaSitios", () => ({ usePresenciaPorSitio: () => ({}) }));
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }));
// La grilla real (AG Grid) no dibuja celdas en jsdom: este doble dibuja la
// columna de acciones de cada fila, que es lo que importa acá.
vi.mock("../componentes/Tabla", () => ({
  default: ({ filas, columnas }: {
    filas: unknown[];
    columnas: Array<{ colId?: string; cellRenderer?: (p: { data: unknown }) => ReactNode }>;
  }) => {
    const dibujar = columnas.find((c) => c.colId === "acciones")?.cellRenderer;
    return <div>{dibujar && filas.map((fila, i) => <div key={i}>{dibujar({ data: fila })}</div>)}</div>;
  },
}));
// La verificación por correo: si se abre, se nota y se puede completar.
vi.mock("../componentes/ConfirmacionSensible", () => ({
  default: ({ abierto, titulo, onConfirmar }: { abierto: boolean; titulo: string; onConfirmar: () => Promise<void> }) =>
    abierto
      ? (
        <div role="dialog" aria-label="Verificación por correo">
          {titulo}
          <button type="button" onClick={() => void onConfirmar()}>Confirmar con el código</button>
        </div>
      )
      : null,
}));
vi.mock("../componentes/CodigoVinculacionEmitido", () => ({
  default: ({ codigo }: { codigo: string }) => <p>Código: {codigo}</p>,
}));

const SITIO = { id: "s1", nombre: "Brisas", created_at: "2026-09-01T00:00:00Z" };
const OTRO_SITIO = { id: "s2", nombre: "Cartago", created_at: "2026-09-01T00:00:00Z" };

function equipo(cambios: Partial<Dispositivo>): Dispositivo {
  return {
    id: "d1",
    sitio_id: "s1",
    tipo: "mobile",
    etiqueta: "Celular Brisas",
    created_at: "2026-10-02T00:00:00Z",
    revoked_at: null,
    last_seen_at: null,
    oculto_en_panel: false,
    identificador_hardware: null,
    nombre_dispositivo: null,
    plataforma: null,
    version_build: null,
    app_version: null,
    last_ip: null,
    clave_huella: "huella",
    vinculado_en: "2026-10-02T00:00:00Z",
    credencial: "clave",
    ...cambios,
  };
}

function montar(dispositivos: Dispositivo[]) {
  mocks.datos = { sitios: [SITIO, OTRO_SITIO], dispositivos, codigos_pendientes: [], eventos: [] };
  const cliente = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={cliente}>
      <Dispositivos sesion={{ correo: "admin@example.com", nombre: "Admin" } as never} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.recargar.mockResolvedValue(undefined);
});

describe("Dispositivos -- autorizar reconexión", () => {
  it("genera el código directo, sin pedir código por correo", async () => {
    mocks.autorizarReconexion.mockResolvedValue({
      codigo: "K7QMR4XT2P",
      expira_en: "2099-01-01T00:00:00Z",
      sitio_nombre: "Brisas",
    });
    montar([equipo({ revoked_at: "2026-10-26T12:10:49Z" })]);

    fireEvent.click(screen.getByRole("button", { name: "Autorizar reconexión" }));
    fireEvent.click(screen.getByRole("button", { name: "Generar código de reconexión" }));

    await waitFor(() => expect(mocks.autorizarReconexion).toHaveBeenCalledTimes(1));
    expect(mocks.autorizarReconexion).toHaveBeenCalledWith({
      dispositivo_id: "d1",
      sitio_id: "s1",
      etiqueta: "Celular Brisas",
    });
    expect(screen.queryByRole("dialog", { name: "Verificación por correo" })).toBeNull();
    expect(await screen.findByText("Código: K7QMR4XT2P")).toBeTruthy();
  });

  it("avisa a la vista cuando el equipo va a otra unidad", () => {
    montar([equipo({ revoked_at: "2026-10-26T12:10:49Z" })]);
    fireEvent.click(screen.getByRole("button", { name: "Autorizar reconexión" }));

    expect(screen.queryByText(/borra los datos de su unidad anterior/)).toBeNull();
    fireEvent.change(screen.getByLabelText("Unidad operativa"), { target: { value: "s2" } });
    expect(screen.getByRole("alert").textContent).toMatch(/borra los datos de su unidad anterior/);
  });

  it("un doble clic no emite dos códigos", async () => {
    let terminar: (valor: unknown) => void = () => {};
    mocks.autorizarReconexion.mockReturnValue(new Promise((resolver) => (terminar = resolver)));
    montar([equipo({ revoked_at: "2026-10-26T12:10:49Z" })]);

    fireEvent.click(screen.getByRole("button", { name: "Autorizar reconexión" }));
    const generar = screen.getByRole("button", { name: "Generar código de reconexión" });
    fireEvent.click(generar);
    fireEvent.click(generar);
    terminar({ codigo: "K7QMR4XT2P", expira_en: "2099-01-01T00:00:00Z", sitio_nombre: "Brisas" });

    await waitFor(() => expect(screen.queryByText("Código: K7QMR4XT2P")).toBeTruthy());
    expect(mocks.autorizarReconexion).toHaveBeenCalledTimes(1);
  });
});

describe("Dispositivos -- retirar y eliminar siguen pidiendo código por correo", () => {
  it("retirar no llama a la API hasta confirmar con el código", async () => {
    mocks.revocarDispositivo.mockResolvedValue({});
    montar([equipo({})]);

    fireEvent.click(screen.getByRole("button", { name: "Retirar" }));
    expect(screen.getByRole("dialog", { name: "Verificación por correo" })).toBeTruthy();
    expect(mocks.revocarDispositivo).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Confirmar con el código" }));
    await waitFor(() => expect(mocks.revocarDispositivo).toHaveBeenCalledWith("d1"));
  });

  it("eliminar no llama a la API hasta confirmar con el código", async () => {
    mocks.eliminarDispositivo.mockResolvedValue({ ok: true });
    montar([equipo({})]);

    fireEvent.click(screen.getByRole("button", { name: "Eliminar" }));
    expect(screen.getByRole("dialog", { name: "Verificación por correo" })).toBeTruthy();
    expect(mocks.eliminarDispositivo).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Confirmar con el código" }));
    await waitFor(() => expect(mocks.eliminarDispositivo).toHaveBeenCalledWith("d1"));
  });
});
