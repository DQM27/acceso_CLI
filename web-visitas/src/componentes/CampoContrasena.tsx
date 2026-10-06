import { useId, useState } from "react";
import { Eye, EyeOff } from "lucide-react";

/** Contraseña con botón para mostrarla: menos errores de tipeo sin obligar
 * a copiarla en otro lado (NIST SP 800-63B-4). Pegar siempre se permite. */
export function CampoContrasena({
  etiqueta,
  valor,
  onCambiar,
  autoComplete,
  error,
  ayuda,
}: {
  etiqueta: string;
  valor: string;
  onCambiar: (valor: string) => void;
  autoComplete: "current-password" | "new-password";
  error?: string | null;
  ayuda?: string;
}) {
  const [visible, setVisible] = useState(false);
  const id = useId();
  const descripcion = error ? `${id}-error` : ayuda ? `${id}-ayuda` : undefined;
  return (
    <div className="campo" data-error={!!error}>
      <label htmlFor={id}>{etiqueta}</label>
      <div className="relative">
        <input
          id={id}
          type={visible ? "text" : "password"}
          value={valor}
          required
          autoComplete={autoComplete}
          autoCapitalize="none"
          autoCorrect="off"
          spellCheck={false}
          aria-invalid={!!error}
          aria-describedby={descripcion}
          className="pr-11"
          onChange={(evento) => onCambiar(evento.target.value)}
        />
        <button
          type="button"
          className="absolute inset-y-0 right-0 flex w-11 items-center justify-center text-muted"
          aria-label={visible ? "Ocultar contraseña" : "Mostrar contraseña"}
          aria-pressed={visible}
          onClick={() => setVisible((antes) => !antes)}
        >
          {visible ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}
        </button>
      </div>
      {error ? (
        <span id={`${id}-error`} className="campo-error">
          {error}
        </span>
      ) : (
        ayuda && (
          <span id={`${id}-ayuda`} className="text-[12px]">
            {ayuda}
          </span>
        )
      )}
    </div>
  );
}
