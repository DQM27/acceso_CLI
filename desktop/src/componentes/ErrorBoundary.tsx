import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

/**
 * Sin esto, un error de render sin capturar en cualquier pantalla (un dato
 * inesperado, un `undefined` no previsto) tumba TODO el árbol de React —
 * React 19 lo desmonta entero, pantalla en blanco, sin mensaje, sin forma de
 * recuperarse salvo reiniciar la app a mano. Los error boundaries siguen
 * siendo componentes de clase en React — no hay equivalente con hooks
 * todavía (`getDerivedStateFromError`/`componentDidCatch` no existen como
 * hook). Envuelve `<App />` entero en `main.tsx`: un error en una pantalla
 * no debe dejar sin login/sidebar a alguien que ya estaba adentro.
 */
export default class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Error sin capturar en la GUI:", error, info.componentStack);
  }

  render() {
    if (!this.state.error) {
      return this.props.children;
    }

    return (
      <div
        style={{
          display: "flex",
          height: "100%",
          alignItems: "center",
          justifyContent: "center",
          padding: "2rem",
        }}
      >
        <div
          className="tarjeta"
          style={{
            maxWidth: "28rem",
            padding: "1.5rem",
            display: "flex",
            flexDirection: "column",
            gap: "0.75rem",
          }}
        >
          <h2 style={{ margin: 0, fontSize: "1.1rem", color: "var(--error)" }}>
            Ocurrió un error inesperado
          </h2>
          <p style={{ margin: 0, color: "var(--muted)" }}>
            La ventana no puede seguir en este estado. Los datos ya guardados no se
            pierden — reiniciá la app para continuar.
          </p>
          <p
            style={{
              margin: 0,
              padding: "0.6rem 0.75rem",
              background: "var(--campo-fondo)",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              color: "var(--muted)",
              fontSize: "0.8rem",
              fontFamily: "ui-monospace, Consolas, monospace",
              overflowWrap: "break-word",
            }}
          >
            {this.state.error.message}
          </p>
          <button
            type="button"
            className="boton boton-primario"
            onClick={() => window.location.reload()}
          >
            Reiniciar
          </button>
        </div>
      </div>
    );
  }
}
