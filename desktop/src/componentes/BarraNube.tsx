import { RefreshCw } from "lucide-react";

/**
 * Botón "Sincronizar" en la barra de estado — mismo lenguaje visual que
 * `MenuUsuario` (`.barra-estado-boton`, texto plano hasta el hover). Visible
 * para cualquier rol activo: sincronizar (`Operacion::UsarNube`) ya es uso
 * diario normal, no exclusivo de ROOT como configurar el secreto del
 * dispositivo (`Operacion::GestionarNube`, eso sigue solo en la pantalla
 * Nube). Antes este botón vivía únicamente ahí, detrás de una pestaña que ni
 * Administrador ni Operador podían abrir.
 *
 * Sin indicador de conexión: dependía del canal Realtime (`nubeRealtime.ts`),
 * apagado por ahora (ver el comentario en `App.tsx`) — mientras tanto no hay
 * señal de conectividad continua que mostrar, sólo el pulso automático y este
 * botón manual.
 */
export default function BarraNube({
  sincronizando,
  onSincronizar,
}: {
  sincronizando: boolean;
  onSincronizar: () => void;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
      <button
        type="button"
        className="barra-estado-boton"
        onClick={onSincronizar}
        disabled={sincronizando}
        style={{ display: "flex", alignItems: "center", gap: "0.3rem" }}
      >
        <RefreshCw
          size={13}
          strokeWidth={2}
          aria-hidden="true"
          className={sincronizando ? "girando" : undefined}
        />
        {sincronizando ? "Sincronizando…" : "Sincronizar"}
      </button>
    </div>
  );
}
