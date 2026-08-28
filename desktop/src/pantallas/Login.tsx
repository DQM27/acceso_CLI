import { useState } from "react";
import type { FormEvent } from "react";
import { login } from "../api";
import type { UsuarioSesion } from "../api";

export default function Login({
  onAutenticado,
}: {
  onAutenticado: (sesion: UsuarioSesion) => void;
}) {
  const [cedula, setCedula] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [verificando, setVerificando] = useState(false);

  async function manejarEnvio(evento: FormEvent) {
    evento.preventDefault();
    setError(null);
    setVerificando(true);
    try {
      const sesion = await login(cedula, password);
      onAutenticado(sesion);
    } catch (error) {
      setError(String(error));
    } finally {
      setVerificando(false);
    }
  }

  return (
    <div className="min-h-full bg-fondo text-texto">
      <div className="login-superficie grid min-h-full place-items-center px-6 py-10">
        <div className="grid w-full max-w-5xl gap-10 lg:grid-cols-[1fr_25rem] lg:items-center">
          <section className="hidden max-w-xl lg:block">
            <div className="mb-10 flex items-center gap-3">
              <div className="marca-sello" aria-hidden="true">
                B
              </div>
              <div>
                <p className="text-sm font-semibold text-texto">Brisas</p>
                <p className="text-xs text-muted">Control de acceso</p>
              </div>
            </div>

            <p className="mb-4 text-sm font-medium uppercase tracking-[0.18em] text-acento">
              Operación segura
            </p>
            <h1 className="max-w-lg text-4xl font-semibold leading-tight text-texto">
              Registro claro para entradas, empresas y usuarios.
            </h1>
            <p className="mt-5 max-w-lg text-base leading-7 text-muted">
              Una consola de trabajo sobria, rápida y pensada para revisar información sin ruido.
            </p>

            <div className="mt-10 grid max-w-lg grid-cols-3 gap-3">
              <div className="login-indicador">
                <span>Modo</span>
                <strong>Privado</strong>
              </div>
              <div className="login-indicador">
                <span>Acceso</span>
                <strong>Interno</strong>
              </div>
              <div className="login-indicador">
                <span>Estado</span>
                <strong>Activo</strong>
              </div>
            </div>
          </section>

          <form
            onSubmit={manejarEnvio}
            className="tarjeta login-card flex w-full flex-col gap-5 p-7 shadow-[var(--sombra-panel)] sm:p-8"
          >
            <div className="flex items-start justify-between gap-4">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted">
                  Control de acceso
                </p>
                <h2 className="mt-2 text-2xl font-semibold text-texto">Iniciar sesión</h2>
              </div>
              <div className="marca-sello marca-sello-compacto" aria-hidden="true">
                B
              </div>
            </div>

            <p className="text-sm leading-6 text-muted">
              Ingresa tus credenciales para continuar con la operación.
            </p>

            <label className="campo">
              Cédula
              <input
                value={cedula}
                onChange={(evento) => setCedula(evento.target.value)}
                autoFocus
                disabled={verificando}
                autoComplete="username"
                placeholder="Número de cédula"
              />
            </label>

            <label className="campo">
              Contraseña
              <input
                type="password"
                value={password}
                onChange={(evento) => setPassword(evento.target.value)}
                disabled={verificando}
                autoComplete="current-password"
                placeholder="Contraseña"
              />
            </label>

            {error && (
              <p className="login-error" role="alert">
                {error}
              </p>
            )}

            <button
              type="submit"
              className="boton boton-primario mt-1 w-full"
              disabled={verificando || !cedula || !password}
            >
              {verificando ? "Verificando..." : "Ingresar"}
            </button>

            <p className="border-t border-borde pt-4 text-xs leading-5 text-muted">
              Sesión protegida para personal autorizado.
            </p>
          </form>
        </div>
      </div>
    </div>
  );
}
