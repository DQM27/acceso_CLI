import { Component, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ErrorInfo, ReactNode } from "react";
import { AlertCircle, Loader2, Moon, Sun, X } from "lucide-react";

export function Cargando({ texto = "Cargando…" }: { texto?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 py-12 text-muted" role="status">
      <Loader2 aria-hidden="true" className="girando" />
      <span>{texto}</span>
    </div>
  );
}

const CLASE_AVISO = {
  error: "aviso-error",
  exito: "aviso-exito",
  info: "aviso-info",
} as const;

export function Aviso({
  children,
  tipo = "error",
}: {
  children: ReactNode;
  tipo?: "error" | "exito" | "info";
}) {
  return (
    <div className={`aviso ${CLASE_AVISO[tipo]}`} role={tipo === "error" ? "alert" : "status"}>
      <AlertCircle aria-hidden="true" />
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}

export function SelectorTema() {
  const [tema, setTema] = useState(() => {
    try {
      const guardado = localStorage.getItem("visitas:tema");
      if (guardado === "light" || guardado === "dark") return guardado;
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
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
      localStorage.setItem("visitas:tema", siguiente);
    } catch {
      /* La preferencia no es necesaria para usar la web. */
    }
  };
  return (
    <button
      type="button"
      className="boton boton-fantasma boton-icono"
      onClick={alternar}
      aria-label={`Cambiar a tema ${tema === "light" ? "oscuro" : "claro"}`}
    >
      {tema === "light" ? <Moon aria-hidden="true" /> : <Sun aria-hidden="true" />}
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
        <h2 className="text-base font-semibold" id="titulo-modal">
          {titulo}
        </h2>
        <button
          type="button"
          className="boton boton-fantasma boton-icono"
          aria-label="Cerrar diálogo"
          onClick={onCerrar}
          disabled={ocupado}
        >
          <X aria-hidden="true" />
        </button>
      </div>
      <div className="modal-cuerpo">{children}</div>
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
        <main className="flex min-h-dvh flex-col items-center justify-center gap-4 p-8 text-center">
          <h1 className="text-xl font-semibold">No pudimos abrir esta pantalla</h1>
          <p className="text-muted">Recargue la página para volver a intentarlo.</p>
          <button type="button" className="boton boton-primario" onClick={() => window.location.reload()}>
            Recargar
          </button>
        </main>
      );
    return this.props.children;
  }
}
