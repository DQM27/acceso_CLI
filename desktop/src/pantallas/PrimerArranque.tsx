import { useState } from "react";
import { vincularDispositivoInicial } from "../api";
import { codigoCompleto, formatearCodigo } from "../componentes/CodigoVinculacion.logica";

/**
 * Única pantalla cuando la base está vacía (`requiereConfiguracionInicial`
 * en `App.tsx`) -- sin login todavía, porque no hay ningún usuario con
 * quien autenticar. Canjear el código de vinculación que el panel generó
 * para este equipo trae el catálogo remoto completo
 * (contratistas/empresas/gafetes/usuarios) en el mismo paso; los usuarios
 * llegan con el centinela `SIN_PASSWORD_LOCAL` (ver
 * `src/nube/sincronizacion.rs`), así que el primer login de cualquiera de
 * ellos cae solo en "fijar contraseña".
 *
 * El código es de un solo uso y vence en minutos: no es una credencial. Al
 * canjearlo, este equipo genera su propia clave y sólo manda la parte
 * pública (ver `control_acceso::nube::firmante`).
 */
export default function PrimerArranque({ onListo }: { onListo: () => void }) {
  const [codigo, setCodigo] = useState("");
  const [enviando, setEnviando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function alEnviar(evento: React.FormEvent) {
    evento.preventDefault();
    if (!codigoCompleto(codigo)) return;
    setEnviando(true);
    setError(null);
    try {
      await vincularDispositivoInicial(codigo);
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
          <h2 style={{ margin: "0 0 0.35rem" }}>Vincular este equipo</h2>
          <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.9rem" }}>
            Escribí el código que muestra el panel de administración para este equipo. Vence en
            pocos minutos y sirve una sola vez; al vincular se trae el catálogo, incluidos los
            usuarios, y con eso ya se puede iniciar sesión.
          </p>
        </div>

        <label className="campo">
          Código de vinculación
          <input
            type="text"
            autoFocus
            value={codigo}
            onChange={(evento) => setCodigo(formatearCodigo(evento.target.value))}
            placeholder="XXXX-XXXX-XX"
            autoComplete="off"
            spellCheck={false}
            disabled={enviando}
            style={{ fontFamily: "var(--fuente-mono, monospace)", letterSpacing: "0.12em", textAlign: "center" }}
          />
        </label>

        {error && (
          <p className="login-error" role="alert" style={{ margin: 0 }}>
            {error}
          </p>
        )}

        <button type="submit" className="boton boton-primario" disabled={enviando || !codigoCompleto(codigo)}>
          {enviando ? "Vinculando…" : "Vincular y sincronizar"}
        </button>
      </form>
    </div>
  );
}
