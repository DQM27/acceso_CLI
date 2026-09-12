import { Component, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ErrorInfo, ReactNode } from "react";
import { AlertCircle, LoaderCircle, Moon, Sun, X } from "lucide-react";

export function Cargando({ texto = "Cargando…" }: { texto?: string }) {
  return (
    <div className="cargando" role="status">
      <LoaderCircle className="giro" aria-hidden="true" />
      <span>{texto}</span>
    </div>
  );
}

export function Aviso({
  children,
  tipo = "error",
}: {
  children: ReactNode;
  tipo?: "error" | "exito" | "info";
}) {
  return (
    <div
      className={`aviso aviso-${tipo}`}
      role={tipo === "error" ? "alert" : "status"}
    >
      <AlertCircle aria-hidden="true" />
      <div>{children}</div>
    </div>
  );
}

export function SelectorTema() {
  const [tema, setTema] = useState(() => {
    try {
      const guardado = localStorage.getItem("brisas:tema");
      if (guardado === "light" || guardado === "dark") return guardado;
      return window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";
    } catch {
      return "light";
    }
  });
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = tema;
  }, [tema]);
  const alternar = () => {
    const siguiente = tema === "light" ? "dark" : "light";
    setTema(siguiente);
    try {
      localStorage.setItem("brisas:tema", siguiente);
    } catch {
      /* La preferencia no es necesaria para usar la web. */
    }
  };
  return (
    <button
      className="boton boton-discreto solo-icono"
      onClick={alternar}
      aria-label={`Cambiar a tema ${tema === "light" ? "oscuro" : "claro"}`}
    >
      {tema === "light" ? (
        <Moon aria-hidden="true" />
      ) : (
        <Sun aria-hidden="true" />
      )}
    </button>
  );
}

export function Modal({
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
      className="modal"
      aria-labelledby="titulo-modal"
      onCancel={(e) => {
        e.preventDefault();
        if (!ocupado) onCerrar();
      }}
    >
      <div className="modal-encabezado">
        <h2 id="titulo-modal">{titulo}</h2>
        <button
          className="boton boton-discreto solo-icono"
          aria-label="Cerrar diálogo"
          onClick={onCerrar}
          disabled={ocupado}
        >
          <X aria-hidden="true" />
        </button>
      </div>
      {children}
    </dialog>
  );
}

export class LimiteErrores extends Component<
  { children: ReactNode },
  { fallo: boolean }
> {
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
        <main className="error-fatal">
          <h1>No pudimos abrir esta pantalla</h1>
          <p>Recargá la página para volver a intentarlo.</p>
          <button
            className="boton boton-primario"
            onClick={() => window.location.reload()}
          >
            Recargar
          </button>
        </main>
      );
    return this.props.children;
  }
}
