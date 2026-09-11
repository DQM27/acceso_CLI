import { useState } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import marca from "../assets/marca.png";
import { cambiarPasswordSupabase, esErrorLogin, login } from "../api";
import type { UsuarioSesion } from "../api";

export const esquemaLogin = z.object({
  cedula: z.string().min(1, "La cédula es obligatoria"),
  password: z.string().min(1, "La contraseña es obligatoria"),
});

type ValoresLogin = z.infer<typeof esquemaLogin>;

export const esquemaCambioObligatorio = z
  .object({
    passwordNueva: z.string().min(8, "Al menos 8 caracteres"),
    confirmar: z.string().min(1, "Confirmá la contraseña"),
  })
  .refine((valores) => valores.passwordNueva === valores.confirmar, {
    message: "Las contraseñas no coinciden",
    path: ["confirmar"],
  });

type ValoresCambioObligatorio = z.infer<typeof esquemaCambioObligatorio>;

/// Cuando `login` marca `debe_cambiar_password` -- usuario global recién
/// creado en el panel, todavía con la temporal de un solo uso (ver
/// docs/plan-autenticacion-supabase-auth.md) -- queda pendiente este paso
/// antes de dejarlo operar. La sesión YA está activa del lado de Rust
/// (`login` la dejó iniciada); esto sólo cambia la contraseña, no vuelve a
/// autenticar. `passwordActual` es la temporal que recién tipeó -- hace
/// falta para que `cambiar_password_supabase` revalide del lado del
/// backend antes de aceptar la nueva.
function PasoCambioObligatorio({
  sesion,
  passwordActual,
  onListo,
}: {
  sesion: UsuarioSesion;
  passwordActual: string;
  onListo: (sesion: UsuarioSesion) => void;
}) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresCambioObligatorio>({
    resolver: zodResolver(esquemaCambioObligatorio),
    defaultValues: { passwordNueva: "", confirmar: "" },
  });

  async function alEnviar(valores: ValoresCambioObligatorio) {
    try {
      await cambiarPasswordSupabase(passwordActual, valores.passwordNueva);
      onListo(sesion);
    } catch (error) {
      setError("root", { message: String(error) });
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
              {sesion.nombre} · primera vez con esta contraseña temporal
            </p>
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <label className="campo">
            Contraseña nueva
            <input
              type="password"
              {...register("passwordNueva")}
              autoFocus
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

        <button type="submit" className="boton boton-primario w-full" disabled={isSubmitting}>
          {isSubmitting ? "Guardando..." : "Fijar y entrar"}
        </button>
      </form>
    </div>
  );
}

/// Una sola pantalla siempre, en cualquier sitio -- ya no existe el camino
/// de "reclamar" una cuenta con solo la cédula
/// (`services/password.rs::SIN_PASSWORD_LOCAL`, cerrado en
/// docs/plan-autenticacion-supabase-auth.md). `login` resuelve del lado
/// del backend si esta cédula es local (ROOT del arranque inicial) o
/// global (Supabase Auth) -- acá no hace falta saber cuál de las dos fue.
export default function Login({
  onAutenticado,
}: {
  onAutenticado: (sesion: UsuarioSesion) => void;
}) {
  const [cambioObligatorio, setCambioObligatorio] = useState<{
    sesion: UsuarioSesion;
    passwordActual: string;
  } | null>(null);

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
      const resultado = await login(valores.cedula, valores.password);
      if (resultado.debe_cambiar_password) {
        setCambioObligatorio({ sesion: resultado.sesion, passwordActual: valores.password });
        return;
      }
      onAutenticado(resultado.sesion);
    } catch (error) {
      setError("root", { message: esErrorLogin(error) ? error.mensaje : String(error) });
    }
  }

  if (cambioObligatorio !== null) {
    return (
      <PasoCambioObligatorio
        sesion={cambioObligatorio.sesion}
        passwordActual={cambioObligatorio.passwordActual}
        onListo={onAutenticado}
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
