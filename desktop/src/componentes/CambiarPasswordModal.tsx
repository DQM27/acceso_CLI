import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { toast } from "sonner";
import Modal from "./Modal";
import { cambiarPasswordSupabase } from "../api";

export const esquemaCambioPassword = z
  .object({
    passwordActual: z.string().min(1, "La contraseña actual es obligatoria"),
    passwordNueva: z.string().min(8, "Al menos 8 caracteres"),
    confirmar: z.string().min(1, "Confirma la contraseña"),
  })
  .refine((valores) => valores.passwordNueva === valores.confirmar, {
    message: "Las contraseñas no coinciden",
    path: ["confirmar"],
  })
  .refine((valores) => valores.passwordNueva !== valores.passwordActual, {
    message: "La nueva tiene que ser distinta de la actual",
    path: ["passwordNueva"],
  });

type ValoresCambioPassword = z.infer<typeof esquemaCambioPassword>;

/**
 * Cambio rutinario de la propia contraseña, desde el menú del usuario en
 * la barra de estado (pedido del usuario 2026-09-23). Mismo comando que el
 * cambio obligatorio del primer login (`PasoCambioObligatorio` en
 * Login.tsx): `cambiar_password_supabase` revalida la actual contra
 * Supabase y refresca el caché de login offline con la nueva. No usa
 * `cambiarMiPassword` (sólo toca el hash local): con un usuario global la
 * contraseña local y la de Supabase quedarían distintas.
 */
export default function CambiarPasswordModal({ onCerrar }: { onCerrar: () => void }) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresCambioPassword>({
    resolver: zodResolver(esquemaCambioPassword),
    defaultValues: { passwordActual: "", passwordNueva: "", confirmar: "" },
  });

  async function alEnviar(valores: ValoresCambioPassword) {
    try {
      await cambiarPasswordSupabase(valores.passwordActual, valores.passwordNueva);
      toast.success("Contraseña cambiada.");
      onCerrar();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Cambiar contraseña" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alEnviar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}
      >
        <label className="campo">
          Contraseña actual
          <input
            type="password"
            {...register("passwordActual")}
            autoFocus
            disabled={isSubmitting}
            autoComplete="current-password"
          />
          {errors.passwordActual && (
            <span className="login-error-campo">{errors.passwordActual.message}</span>
          )}
        </label>

        <label className="campo">
          Contraseña nueva
          <input
            type="password"
            {...register("passwordNueva")}
            disabled={isSubmitting}
            autoComplete="new-password"
            placeholder="Al menos 8 caracteres"
          />
          {errors.passwordNueva && (
            <span className="login-error-campo">{errors.passwordNueva.message}</span>
          )}
        </label>

        <label className="campo">
          Confirmar contraseña
          <input
            type="password"
            {...register("confirmar")}
            disabled={isSubmitting}
            autoComplete="new-password"
          />
          {errors.confirmar && (
            <span className="login-error-campo">{errors.confirmar.message}</span>
          )}
        </label>

        {errors.root && (
          <p className="login-error" role="alert">
            {errors.root.message}
          </p>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar} disabled={isSubmitting}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Guardando…" : "Cambiar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
