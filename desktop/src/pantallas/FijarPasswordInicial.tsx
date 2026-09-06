import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import marca from "../assets/marca.png";
import { esErrorLogin, fijarPasswordInicial } from "../api";
import type { UsuarioSesion } from "../api";

const esquema = z
  .object({
    password: z.string().min(8, "Al menos 8 caracteres"),
    confirmar: z.string().min(1, "Confirmá la contraseña"),
  })
  .refine((valores) => valores.password === valores.confirmar, {
    message: "Las contraseñas no coinciden",
    path: ["confirmar"],
  });

type Valores = z.infer<typeof esquema>;

/// Pantalla que reemplaza al login normal cuando `AutenticacionError::SinPasswordLocal`
/// avisa que esta cédula existe (sincronizada de otro dispositivo) pero
/// nunca fijó contraseña acá -- ver el doc-comment de `Login.tsx`. No pide
/// contraseña anterior a propósito: nunca existió una en este dispositivo.
export default function FijarPasswordInicial({
  cedula,
  onListo,
  onCancelar,
}: {
  cedula: string;
  onListo: (sesion: UsuarioSesion) => void;
  onCancelar: () => void;
}) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<Valores>({
    resolver: zodResolver(esquema),
    defaultValues: { password: "", confirmar: "" },
  });

  async function alEnviar(valores: Valores) {
    try {
      const sesion = await fijarPasswordInicial(cedula, valores.password);
      onListo(sesion);
    } catch (error) {
      setError("root", { message: esErrorLogin(error) ? error.mensaje : String(error) });
    }
  }

  return (
    <div className="grid min-h-full place-items-center bg-fondo px-6 py-10 text-texto">
      <form
        onSubmit={handleSubmit(alEnviar)}
        className="tarjeta login-card flex w-full max-w-sm flex-col gap-6 p-8 shadow-(--sombra-panel)"
      >
        <div className="flex flex-col items-center gap-3 text-center">
          <div className="marca-sello" aria-hidden="true">
            <img src={marca} alt="" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-texto">Fijar contraseña</h1>
            <p className="text-sm text-muted">
              Cédula {cedula} · primera vez en este dispositivo
            </p>
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <label className="campo">
            Contraseña nueva
            <input
              type="password"
              {...register("password")}
              autoFocus
              disabled={isSubmitting}
              autoComplete="new-password"
              placeholder="Al menos 8 caracteres"
            />
            {errors.password && (
              <span className="login-error-campo">{errors.password.message}</span>
            )}
          </label>

          <label className="campo">
            Confirmar contraseña
            <input
              type="password"
              {...register("confirmar")}
              disabled={isSubmitting}
              autoComplete="new-password"
              placeholder="Repetila"
            />
            {errors.confirmar && (
              <span className="login-error-campo">{errors.confirmar.message}</span>
            )}
          </label>
        </div>

        {errors.root && (
          <p className="login-error" role="alert">
            {errors.root.message}
          </p>
        )}

        <div className="flex flex-col gap-2">
          <button type="submit" className="boton boton-primario w-full" disabled={isSubmitting}>
            {isSubmitting ? "Guardando..." : "Fijar y entrar"}
          </button>
          <button
            type="button"
            className="boton w-full"
            disabled={isSubmitting}
            onClick={onCancelar}
          >
            Volver
          </button>
        </div>
      </form>
    </div>
  );
}
