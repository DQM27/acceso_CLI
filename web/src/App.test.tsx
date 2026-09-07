import { useEffect } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import { fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { User } from "@supabase/supabase-js";
import App from "./App";

// Mockea las 4 pantallas por componentes chicos que sólo avisan cuándo se
// MONTAN -- lo que se prueba acá es que Shell (App.tsx) no las desmonta al
// cambiar de sección (el bug real: antes <Routes>/<Route> las destruía y
// las volvía a crear cada vez, perdiendo datos/estado). No importa el
// contenido real de cada pantalla, ya tienen sus propios tests.
const montajes = vi.hoisted(() => ({
  historial: vi.fn(),
  contratistas: vi.fn(),
  usuarios: vi.fn(),
  dispositivos: vi.fn(),
}));

function Sonda({ etiqueta, alMontar }: { etiqueta: string; alMontar: () => void }) {
  useEffect(() => {
    alMontar();
  }, [alMontar]);
  return <div>{etiqueta}</div>;
}

vi.mock("./pantallas/Historial", () => ({
  default: () => <Sonda etiqueta="Contenido de Historial" alMontar={montajes.historial} />,
}));
vi.mock("./pantallas/Contratistas", () => ({
  default: () => <Sonda etiqueta="Contenido de Contratistas" alMontar={montajes.contratistas} />,
}));
vi.mock("./pantallas/Usuarios", () => ({
  default: () => <Sonda etiqueta="Contenido de Usuarios" alMontar={montajes.usuarios} />,
}));
vi.mock("./pantallas/Dispositivos", () => ({
  default: () => <Sonda etiqueta="Contenido de Dispositivos" alMontar={montajes.dispositivos} />,
}));

const mocks = vi.hoisted(() => ({
  getSession: vi.fn(),
  onAuthStateChange: vi.fn(),
  signOut: vi.fn(),
  signInWithOAuth: vi.fn(),
  maybeSingle: vi.fn(),
}));

vi.mock("./lib/supabase", () => ({
  supabase: {
    auth: {
      getSession: mocks.getSession,
      onAuthStateChange: mocks.onAuthStateChange,
      signOut: mocks.signOut,
      signInWithOAuth: mocks.signInWithOAuth,
    },
    from: () => ({ select: () => ({ eq: () => ({ maybeSingle: mocks.maybeSingle }) }) }),
  },
}));

function usuario(): User {
  return { email: "admin@example.com", user_metadata: { full_name: "Admin" } } as unknown as User;
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.getSession.mockResolvedValue({ data: { session: { user: usuario() } } });
  mocks.maybeSingle.mockResolvedValue({ data: { correo: "admin@example.com" }, error: null });
  mocks.onAuthStateChange.mockReturnValue({ data: { subscription: { unsubscribe: vi.fn() } } });
  window.history.pushState({}, "", "/");
  // jsdom no implementa matchMedia -- lo usa <Toaster theme="system"> (de
  // sonner) para saber si el sistema está en modo oscuro. Sin este stub,
  // App entero tira una excepción no atrapada apenas se monta.
  window.matchMedia ??= vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })) as unknown as typeof window.matchMedia;
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("App -- navegación entre secciones", () => {
  it("no vuelve a montar una sección ya visitada al volver a ella", async () => {
    render(<App />);

    await screen.findByText("Contenido de Historial");
    expect(montajes.historial).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("link", { name: /Contratistas/i }));
    await screen.findByText("Contenido de Contratistas");
    expect(montajes.contratistas).toHaveBeenCalledTimes(1);
    // Historial sigue en el DOM (oculto), no debería haber sido desmontado.
    expect(screen.getByText("Contenido de Historial")).toBeDefined();

    fireEvent.click(screen.getByRole("link", { name: /Historial/i }));
    await waitFor(() => expect(screen.getByText("Contenido de Historial").closest("div")).toBeDefined());

    // El punto real del test: volver a Historial NO lo montó de nuevo.
    expect(montajes.historial).toHaveBeenCalledTimes(1);
    expect(montajes.contratistas).toHaveBeenCalledTimes(1);
  });

  it("sólo la sección activa queda visible -- las demás quedan display:none", async () => {
    render(<App />);
    await screen.findByText("Contenido de Historial");

    fireEvent.click(screen.getByRole("link", { name: /Usuarios/i }));
    await screen.findByText("Contenido de Usuarios");

    const contenedorHistorial = screen.getByText("Contenido de Historial").parentElement;
    const contenedorUsuarios = screen.getByText("Contenido de Usuarios").parentElement;
    expect(contenedorHistorial?.style.display).toBe("none");
    expect(contenedorUsuarios?.style.display).toBe("flex");
  });
});
