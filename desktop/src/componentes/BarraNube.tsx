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
 * Permite forzar una sincronización además de los avisos de Realtime y del
 * pulso periódico de respaldo.
 */
export default function BarraNube({
  estado,
  sincronizando,
  onSincronizar,
}: {
  estado: string;
  sincronizando: boolean;
  onSincronizar: () => void;
}) {
  const conectado = estado === "SUBSCRIBED";
  const textoConexion = conectado
    ? "Nube conectada"
    : estado === "CONNECTING"
      ? "Conectando a la nube…"
      : "Sin conexión en vivo";
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
      <span
        role="status"
        title="Estado del canal de avisos en vivo; la sincronización también se intenta periódicamente."
        style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
      >
        <span
          aria-hidden="true"
          style={{
            width: "0.5rem",
            height: "0.5rem",
            borderRadius: "50%",
            background: conectado ? "var(--exito)" : "var(--muted)",
          }}
        />
        {textoConexion}
      </span>
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
