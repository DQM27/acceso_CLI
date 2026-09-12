import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { User } from "@supabase/supabase-js";
import { AuthProvider, useAuth } from "./AuthContexto";

const mocks = vi.hoisted(() => ({
  getSession: vi.fn(),
  onAuthStateChange: vi.fn(),
  signOut: vi.fn(),
  signInWithOAuth: vi.fn(),
  maybeSingle: vi.fn(),
}));

vi.mock("../lib/supabase", () => ({
  supabase: {
    auth: {
      getSession: mocks.getSession,
      onAuthStateChange: mocks.onAuthStateChange,
      signOut: mocks.signOut,
      signInWithOAuth: mocks.signInWithOAuth,
    },
    from: () => ({
      select: () => ({
        eq: () => ({
          maybeSingle: mocks.maybeSingle,
        }),
      }),
    }),
  },
}));

function Sonda() {
  const { sesion, cargando, error } = useAuth();
  return (
    <div>
      <span data-testid="cargando">{String(cargando)}</span>
      <span data-testid="sesion">{sesion ? sesion.correo : "null"}</span>
      <span data-testid="error">{error ?? "null"}</span>
    </div>
  );
}

function usuario(email: string): User {
  return { email, user_metadata: { full_name: "Alguien" } } as unknown as User;
}

let escuchador: ((evento: string, session: { user: User } | null) => void) | undefined;

beforeEach(() => {
  vi.clearAllMocks();
  escuchador = undefined;
  mocks.onAuthStateChange.mockImplementation((callback: typeof escuchador) => {
    escuchador = callback;
    return { data: { subscription: { unsubscribe: vi.fn() } } };
  });
  mocks.signOut.mockResolvedValue({ error: null });
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("AuthContexto", () => {
  it("mantiene la sesión activa si un re-chequeo en segundo plano falla por red -- no desloguea por un error de conexión", async () => {
    mocks.getSession.mockResolvedValue({ data: { session: { user: usuario("admin@example.com") } } });
    mocks.maybeSingle.mockResolvedValueOnce({ data: { correo: "admin@example.com" }, error: null });

    render(
      <AuthProvider>
        <Sonda />
      </AuthProvider>,
    );

    await screen.findByText("admin@example.com");
    expect(mocks.signOut).not.toHaveBeenCalled();

    // Re-chequeo en segundo plano (ej. la pestaña recupera el foco, según
    // el propio comentario de AuthContexto esto pasa seguido) que esta vez
    // falla por red, no porque el admin haya dejado de estarlo.
    mocks.maybeSingle.mockResolvedValueOnce({ data: null, error: { message: "network error" } });
    await act(async () => {
      escuchador?.("TOKEN_REFRESHED", { user: usuario("admin@example.com") });
    });

    // Sigue logueado -- un error de red no debe sacarlo de una sesión que
    // ya tenía andando.
    expect(screen.getByTestId("sesion").textContent).toBe("admin@example.com");
    expect(mocks.signOut).not.toHaveBeenCalled();
    expect(screen.getByTestId("error").textContent).toContain("conexión");
  });

  it("desloguea de verdad cuando la cuenta no está en administradores_panel (consulta sin error, sin fila)", async () => {
    mocks.getSession.mockResolvedValue({ data: { session: { user: usuario("intruso@example.com") } } });
    mocks.maybeSingle.mockResolvedValue({ data: null, error: null });

    render(
      <AuthProvider>
        <Sonda />
      </AuthProvider>,
    );

    await screen.findByText(/no está autorizada/);
    expect(mocks.signOut).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("sesion").textContent).toBe("null");
  });

  it("sin sesión de Google, no consulta administradores_panel ni desloguea", async () => {
    mocks.getSession.mockResolvedValue({ data: { session: null } });

    render(
      <AuthProvider>
        <Sonda />
      </AuthProvider>,
    );

    await screen.findByText("false", { selector: "[data-testid=cargando]" });
    expect(mocks.maybeSingle).not.toHaveBeenCalled();
    expect(mocks.signOut).not.toHaveBeenCalled();
    expect(screen.getByTestId("sesion").textContent).toBe("null");
  });

  it("no restaura una cuenta vieja si su consulta de autorización responde después que la de una cuenta más nueva", async () => {
    mocks.getSession.mockResolvedValue({ data: { session: null } });

    render(
      <AuthProvider>
        <Sonda />
      </AuthProvider>,
    );
    await screen.findByText("false", { selector: "[data-testid=cargando]" });

    let resolverVieja!: (valor: unknown) => void;
    mocks.maybeSingle.mockReturnValueOnce(
      new Promise((resolver) => {
        resolverVieja = resolver;
      }),
    );
    await act(async () => {
      escuchador?.("SIGNED_IN", { user: usuario("vieja@example.com") });
    });

    mocks.maybeSingle.mockResolvedValueOnce({
      data: { correo: "nueva@example.com" },
      error: null,
    });
    await act(async () => {
      escuchador?.("SIGNED_IN", { user: usuario("nueva@example.com") });
    });
    await screen.findByText("nueva@example.com");

    // La consulta de la cuenta vieja llega tarde -- no debe pisar la sesión
    // ya vigente de la cuenta nueva (hallazgo P2).
    await act(async () => {
      resolverVieja({ data: { correo: "vieja@example.com" }, error: null });
    });

    expect(screen.getByTestId("sesion").textContent).toBe("nueva@example.com");
  });

  it("no deja la app en blanco para siempre si getSession() rechaza", async () => {
    mocks.getSession.mockRejectedValue(new Error("fallo interno de supabase-js"));

    render(
      <AuthProvider>
        <Sonda />
      </AuthProvider>,
    );

    // Sin el fix, `cargando` se queda en "true" para siempre (la rejection
    // no tiene ningún manejo) y `Contenido` nunca renderiza nada.
    await screen.findByText("false", { selector: "[data-testid=cargando]" });
    expect(screen.getByTestId("sesion").textContent).toBe("null");
    expect(screen.getByTestId("error").textContent).toContain("verificación");
  });
});
