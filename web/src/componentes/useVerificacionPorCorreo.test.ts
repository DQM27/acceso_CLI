import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useVerificacionPorCorreo } from "./useVerificacionPorCorreo";

const mocks = vi.hoisted(() => ({
  signInWithOtp: vi.fn(),
  verifyOtp: vi.fn(),
}));
vi.mock("../lib/supabase", () => ({
  supabase: { auth: { signInWithOtp: mocks.signInWithOtp, verifyOtp: mocks.verifyOtp } },
}));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("useVerificacionPorCorreo -- confirmarCodigo", () => {
  it("un error de red (sin status, AuthError antes de respuesta) no dice 'código inválido'", async () => {
    // Documentado en AuthError de @supabase/auth-js: status/code quedan
    // undefined cuando el error pasa ANTES de recibir respuesta del
    // servidor (sin conexión, timeout, DNS) -- a diferencia de un código
    // realmente rechazado por el servidor, que sí trae un status HTTP.
    mocks.verifyOtp.mockResolvedValue({
      error: { message: "Failed to fetch", status: undefined, code: undefined },
    });

    const { result } = renderHook(() => useVerificacionPorCorreo("admin@example.com"));

    await act(async () => {
      await expect(result.current.confirmarCodigo("123456")).rejects.toThrow();
    });

    expect(result.current.error).not.toBeNull();
    expect(result.current.error).toContain("conexión");
    expect(result.current.error).not.toContain("inválido");
  });

  it("un código realmente inválido/vencido (status HTTP real) sí dice 'código inválido'", async () => {
    mocks.verifyOtp.mockResolvedValue({
      error: { message: "Token has expired or is invalid", status: 403, code: "otp_expired" },
    });

    const { result } = renderHook(() => useVerificacionPorCorreo("admin@example.com"));

    await act(async () => {
      await expect(result.current.confirmarCodigo("000000")).rejects.toThrow();
    });

    expect(result.current.error).not.toBeNull();
    expect(result.current.error).toContain("inválido");
  });

  it("sin error, no marca ningún error y no lanza", async () => {
    mocks.verifyOtp.mockResolvedValue({ error: null });

    const { result } = renderHook(() => useVerificacionPorCorreo("admin@example.com"));

    await act(() => result.current.confirmarCodigo("123456"));

    expect(result.current.error).toBeNull();
  });
});
