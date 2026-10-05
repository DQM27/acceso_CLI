import { Component, useEffect, useRef } from "react";
import type { ErrorInfo, ReactNode } from "react";
import { LoaderCircle, Moon, Sun } from "lucide-react";
import { useTema } from "../tema";

export function Cargando({ texto = "Cargando…" }: { texto?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 py-10 text-muted" role="status">
      <LoaderCircle size={18} className="animate-spin" aria-hidden="true" />
      <span>{texto}</span>
    </div>
  );
}

export function Aviso({ children, tipo = "error" }: { children: ReactNode; tipo?: "error" | "info" }) {
  return (
    <div className={tipo === "error" ? "aviso aviso-error" : "aviso"} role={tipo === "error" ? "alert" : "status"}>
      {children}
    </div>
  );
}

export function BotonTema() {
  const { tema, alternar } = useTema();
  return (
    <button
      type="button"
      className="boton boton-discreto"
      onClick={alternar}
      aria-label={`Cambiar a tema ${tema === "light" ? "oscuro" : "claro"}`}
    >
      {tema === "light" ? <Moon aria-hidden="true" /> : <Sun aria-hidden="true" />}
    </button>
  );
}

/** Diálogo nativo (`<dialog>`): foco atrapado, Escape y velo los da el
 * navegador. Al cerrarse devuelve el foco a donde estaba. */
export function Dialogo({
  titulo,
  children,
  onCerrar,
  ocupado = false,
}: {
  titulo: string;
  children: ReactNode;
  onCerrar: () => void;
  ocupado?: boolean;
}) {
  const referencia = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const anterior = document.activeElement as HTMLElement | null;
    const dialogo = referencia.current;
    if (!dialogo) return;
    dialogo.showModal();
    return () => {
      dialogo.close();
      anterior?.focus();
    };
  }, []);
  return (
    <dialog
      ref={referencia}
      className="dialogo"
      aria-labelledby="titulo-dialogo"
      onCancel={(evento) => {
        evento.preventDefault();
        if (!ocupado) onCerrar();
      }}
    >
      <div className="flex flex-col gap-4 p-5">
        <h2 id="titulo-dialogo" className="m-0 text-base font-semibold">
          {titulo}
        </h2>
        {children}
      </div>
    </dialog>
  );
}

export class LimiteErrores extends Component<{ children: ReactNode }, { fallo: boolean }> {
  state = { fallo: false };
  static getDerivedStateFromError() {
    return { fallo: true };
  }
  componentDidCatch(_error: Error, _info: ErrorInfo) {
    /* No enviar datos personales ni estado de sesión a registros. */
  }
  render() {
    if (this.state.fallo)
      return (
        <main className="mx-auto flex max-w-md flex-col items-start gap-3 p-6">
          <h1 className="m-0 text-xl">No se pudo abrir esta pantalla</h1>
          <p className="m-0 text-muted">Recargue la página para volver a intentarlo.</p>
          <button type="button" className="boton boton-primario" onClick={() => window.location.reload()}>
            Recargar
          </button>
        </main>
      );
    return this.props.children;
  }
}
