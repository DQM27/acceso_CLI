import { useState } from "react";
import marca from "../assets/marca.png";
import { useAuth } from "../contexto/AuthContexto";

/**
 * Sin cédula/contraseña a propósito -- este panel es un espacio de actores
 * distinto de Root/Administrador/Operador de cada sitio (ver
 * docs/plan-panel-administrativo-web.md). "Iniciar sesión con Google" es
 * toda la interacción; la autorización real pasa en `AuthContexto` después
 * de que Google confirma quién es la persona.
 */
export default function Login() {
  const { iniciarSesionConGoogle, error } = useAuth();
  const [enviando, setEnviando] = useState(false);

  async function alHacerClic() {
    setEnviando(true);
    try {
      await iniciarSesionConGoogle();
    } finally {
      // Si el redirect a Google no llegó a dispararse (bloqueado, error de
      // red), no dejar el botón pegado en "Conectando...".
      setEnviando(false);
    }
  }

  return (
    <div className="grid min-h-full place-items-center bg-fondo px-6 py-10 text-texto">
      <div className="tarjeta login-card flex w-full max-w-sm flex-col gap-6 p-8 shadow-(--sombra-panel)">
        <div className="flex flex-col items-center gap-3 text-center">
          <div className="marca-sello" aria-hidden="true">
            <img src={marca} alt="" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-texto">Panel administrativo</h1>
            <p className="text-sm text-muted">Brisas</p>
          </div>
        </div>

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        <button
          type="button"
          className="boton boton-primario w-full"
          onClick={alHacerClic}
          disabled={enviando}
        >
          {enviando ? "Conectando…" : "Iniciar sesión con Google"}
        </button>
      </div>
    </div>
  );
}
