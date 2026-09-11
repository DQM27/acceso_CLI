import { useState } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import marca from "../assets/marca.png";
import { esErrorLogin, login } from "../api";
import type { UsuarioSesion } from "../api";
import FijarPasswordInicial from "./FijarPasswordInicial";

export const esquemaLogin = z.object({
  cedula: z.string().min(1, "La cédula es obligatoria"),
  password: z.string().min(1, "La contraseña es obligatoria"),
});

type ValoresLogin = z.infer<typeof esquemaLogin>;

export default function Login({
  onAutenticado,
}: {
  onAutenticado: (sesion: UsuarioSesion) => void;
}) {
  // Usuario global (Administrador/Operador sincronizado, ver
  // `services/error.rs::AutenticacionError::SinPasswordLocal`) que intentó
  // entrar en ESTE dispositivo por primera vez -- no hay nada que verificar
  // todavía, así que en vez de un error se le ofrece fijar una acá mismo.
  // `null` es el estado normal (formulario de login); con la cédula puesta,
  // se reemplaza por `FijarPasswordInicial`.
  const [cedulaSinPassword, setCedulaSinPassword] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresLogin>({
    resolver: zodResolver(esquemaLogin),
    defaultValues: { cedula: "", password: "" },
  });

  async function alEnviar(valores: ValoresLogin) {
    try {
      const sesion = await login(valores.cedula, valores.password);
      onAutenticado(sesion);
    } catch (error) {
      if (esErrorLogin(error) && error.sin_password_local) {
        setCedulaSinPassword(valores.cedula);
        return;
      }
      setError("root", { message: esErrorLogin(error) ? error.mensaje : String(error) });
    }
  }

  if (cedulaSinPassword !== null) {
    return (
      <FijarPasswordInicial
        cedula={cedulaSinPassword}
        onListo={onAutenticado}
        onCancelar={() => setCedulaSinPassword(null)}
      />
    );
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
            <h1 className="text-lg font-semibold text-texto">Control de acceso</h1>
            <p className="text-sm text-muted">Brisas</p>
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <label className="campo">
            Cédula
            <input
              {...register("cedula")}
              autoFocus
              disabled={isSubmitting}
              autoComplete="username"
              placeholder="Número de cédula"
            />
            {errors.cedula && (
              <span className="login-error-campo">{errors.cedula.message}</span>
            )}
          </label>

          <label className="campo">
            Contraseña
            <input
              type="password"
              {...register("password")}
              disabled={isSubmitting}
              autoComplete="current-password"
              placeholder="Contraseña"
            />
            {errors.password && (
              <span className="login-error-campo">{errors.password.message}</span>
            )}
          </label>
        </div>

        {errors.root && (
          <p className="login-error" role="alert">
            {errors.root.message}
          </p>
        )}

        <button type="submit" className="boton boton-primario w-full" disabled={isSubmitting}>
          {isSubmitting ? "Verificando..." : "Ingresar"}
        </button>
      </form>
    </div>
  );
}
