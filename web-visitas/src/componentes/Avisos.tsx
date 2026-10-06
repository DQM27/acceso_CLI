import { CircleCheck, X } from "lucide-react";
import { cerrarAviso, useAvisos } from "../avisos";

/** Zona de avisos arriba al centro. Siempre montada (vacía cuando no hay
 * nada) para que los lectores de pantalla anuncien lo que aparezca. */
export function Avisos() {
  const avisos = useAvisos();
  return (
    <div className="fixed inset-x-0 top-3 z-50 flex justify-center px-4 pointer-events-none" role="status" aria-live="polite">
      {avisos.map((aviso) => (
        <div key={aviso.id} className="aviso-flotante pointer-events-auto">
          <CircleCheck size={18} className="shrink-0 text-exito" aria-hidden="true" />
          <span className="min-w-0 flex-1">{aviso.texto}</span>
          {aviso.accion && (
            <button
              type="button"
              className="boton boton-compacto"
              onClick={() => {
                cerrarAviso(aviso.id);
                aviso.accion?.alPulsar();
              }}
            >
              {aviso.accion.texto}
            </button>
          )}
          <button
            type="button"
            className="boton boton-discreto boton-compacto"
            aria-label="Cerrar aviso"
            onClick={() => cerrarAviso(aviso.id)}
          >
            <X size={16} aria-hidden="true" />
          </button>
        </div>
      ))}
    </div>
  );
}
