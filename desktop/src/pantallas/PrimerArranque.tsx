import { useState } from "react";
import { configurarDispositivoInicial } from "../api";

/**
 * Única pantalla cuando la base está vacía (`requiereConfiguracionInicial`
 * en `App.tsx`) -- sin login todavía, porque no hay ningún usuario con
 * quien autenticar. Pegar el secreto trae el catálogo remoto completo
 * (contratistas/empresas/gafetes/usuarios) en el mismo paso; los usuarios
 * llegan con el centinela `SIN_PASSWORD_LOCAL` (ver
 * `src/nube/sincronizacion.rs`), así que el primer login de cualquiera de
 * ellos cae solo en "fijar contraseña" -- ya existente, no hay nada nuevo
 * que construir ahí.
 *
 * Reemplaza al mensaje fijo de "usá --reset-root/--cli" -- ese camino
 * sigue existiendo como rescate para un sitio que de verdad quiera operar
 * sin nube nunca, pero ya no es el único.
 */
export default function PrimerArranque({ onListo }: { onListo: () => void }) {
  const [secreto, setSecreto] = useState("");
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function alEnviar(evento: React.FormEvent) {
    evento.preventDefault();
    const valor = secreto.trim();
    if (!valor) return;
    setEnviando(true);
    setError(null);
    try {
      await configurarDispositivoInicial(valor);
      onListo();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center" }}>
      <form
        onSubmit={alEnviar}
        className="tarjeta"
        style={{ display: "flex", flexDirection: "column", gap: "1rem", padding: "1.75rem", width: "100%", maxWidth: 420 }}
      >
        <div>
          <h2 style={{ margin: "0 0 0.35rem" }}>Conectar este dispositivo</h2>
          <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.9rem" }}>
            Todavía no hay ningún usuario en esta base. Pegá el secreto que el panel de
            administración generó para este dispositivo — trae el catálogo (incluidos los
            usuarios) y con eso ya se puede iniciar sesión.
          </p>
        </div>

        <input
          type="password"
          autoFocus
          value={secreto}
          onChange={(evento) => setSecreto(evento.target.value)}
          placeholder="Secreto del dispositivo"
          disabled={enviando}
        />

        {error && (
          <p className="login-error" role="alert" style={{ margin: 0 }}>
            {error}
          </p>
        )}

        <button type="submit" className="boton boton-primario" disabled={enviando || !secreto.trim()}>
          {enviando ? "Conectando…" : "Conectar y sincronizar"}
        </button>

        <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.78rem" }}>
          Sin nube todavía disponible para este sitio? Creá el usuario ROOT desde la consola
          (<code>--tui-clasica</code> o <code>--cli</code>) y volvé a abrir esta ventana.
        </p>
      </form>
    </div>
  );
}
