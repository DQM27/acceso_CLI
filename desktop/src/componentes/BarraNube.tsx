import { RefreshCw } from "lucide-react";

/**
 * Botón "Sincronizar" + indicador de conexión con la nube, en la barra de
 * estado — mismo lenguaje visual que `MenuUsuario` (`.barra-estado-boton`,
 * texto plano hasta el hover). Visible para cualquier rol activo: sincronizar
 * (`Operacion::UsarNube`) ya es uso diario normal, no exclusivo de ROOT como
 * configurar el secreto del dispositivo (`Operacion::GestionarNube`, eso
 * sigue solo en la pantalla Nube). Antes este botón vivía únicamente ahí,
 * detrás de una pestaña que ni Administrador ni Operador podían abrir.
 *
 * `estado` viene del canal Realtime (`nubeRealtime.ts`, `onEstado`) — `null`
 * hasta el primer aviso del socket (dispositivo sin secreto configurado
 * todavía, o esperando la primera conexión). No es una fuente perfecta
 * ("conectado" refleja el canal de avisos en vivo, no si la última
 * sincronización en sí funcionó) pero es la única señal de conectividad
 * continua que ya existe — sincronizar de más no cuesta nada, es idempotente.
 */
export default function BarraNube({
  estado,
  sincronizando,
  onSincronizar,
}: {
  estado: string | null;
  sincronizando: boolean;
  onSincronizar: () => void;
}) {
  const conectado = estado === "SUBSCRIBED";

  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
      <span
        title={conectado ? "Conectado a la nube" : "Sin conexión con la nube"}
        style={{
          display: "inline-block",
          width: "0.5rem",
          height: "0.5rem",
          borderRadius: "50%",
          background: conectado ? "var(--exito)" : "var(--muted)",
          flexShrink: 0,
        }}
      />
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
