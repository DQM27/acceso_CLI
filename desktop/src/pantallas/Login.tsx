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
    <div
      style={{
        display: "flex",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <form
        onSubmit={manejarEnvio}
        className="tarjeta"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "1.1rem",
          width: "23rem",
          padding: "2.25rem",
        }}
      >
        <div>
          <p
            style={{
              margin: "0 0 0.2rem",
              fontSize: "0.72rem",
              letterSpacing: "0.12em",
              textTransform: "uppercase",
              color: "var(--muted)",
            }}
          >
            Control de Acceso
          </p>
          <h1 style={{ margin: 0, fontSize: "1.5rem", color: "var(--acento)" }}>Brisas</h1>
        </div>

        <label className="campo">
          Cédula
          <input
            value={cedula}
            onChange={(evento) => setCedula(evento.target.value)}
            autoFocus
            disabled={verificando}
          />
        </label>

        <label className="campo">
          Contraseña
          <input
            type="password"
            value={password}
            onChange={(evento) => setPassword(evento.target.value)}
            disabled={verificando}
          />
        </label>

        {error && (
          <p style={{ color: "var(--error)", margin: 0, fontSize: "0.9rem" }}>{error}</p>
        )}

        <button
          type="submit"
          className="boton boton-primario"
          disabled={verificando || !cedula || !password}
        >
          {verificando ? "Verificando…" : "Ingresar"}
        </button>
      </form>
    </div>
  );
}
