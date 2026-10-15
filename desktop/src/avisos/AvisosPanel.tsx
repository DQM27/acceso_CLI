import { Toaster } from "sonner";
import { AlertTriangle, CheckCircle2, Info, Loader2, XCircle } from "lucide-react";
import { MAXIMO_VISIBLES } from "./avisos.logica";
import "./avisos.css";

/**
 * Donde se ven los avisos: va una sola vez, en la raíz de la app. Sólo
 * presenta -- el aspecto vive en `avisos.css` y los tipos, títulos y
 * duraciones en `avisos.logica.ts`. Se muestran arriba a la derecha, como las
 * notificaciones de macOS.
 */
export default function AvisosPanel() {
  return (
    <Toaster
      // Siempre "light": el color lo ponen las variables de la app (claro, oscuro,
      // Tokyo Night), no `sonner`. Con "system" seguía al sistema operativo y,
      // con el sistema en oscuro y la app en claro, pintaba el texto de blanco.
      theme="light"
      position="top-right"
      visibleToasts={MAXIMO_VISIBLES}
      gap={8}
      offset={12}
      icons={{
        success: <CheckCircle2 size={20} aria-hidden />,
        info: <Info size={20} aria-hidden />,
        warning: <AlertTriangle size={20} aria-hidden />,
        error: <XCircle size={20} aria-hidden />,
        loading: <Loader2 size={20} className="aviso-girando" aria-hidden />,
      }}
      toastOptions={{
        unstyled: true,
        classNames: {
          toast: "aviso",
          icon: "aviso-insignia",
          content: "aviso-cuerpo",
          title: "aviso-titulo",
          description: "aviso-mensaje",
          actionButton: "aviso-accion",
          closeButton: "aviso-cerrar",
        },
      }}
    />
  );
}
