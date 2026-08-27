import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { actualizarUsuario, cambiarPasswordUsuario, crearUsuario } from "../api";
import type { RolUsuario, UsuarioResumen } from "../api";

const ROLES = ["Root", "Administrador", "Operador"] as const;

interface ValoresFormulario {
  cedula: string;
  nombre: string;
  password: string;
  rol: RolUsuario;
  activo: boolean;
}

// Distinto esquema según el modo: en alta la contraseña es obligatoria, en
// edición es opcional (vacío = "no cambiarla") pero si se escribe algo,
// tiene que cumplir el mínimo igual que al crear.
function construirEsquema(esCreacion: boolean) {
  return z.object({
    cedula: z
      .string()
      .min(1, "La cédula es obligatoria")
      .regex(/^\d+$/, "La cédula sólo puede tener números"),
    nombre: z
      .string()
      .min(1, "El nombre es obligatorio")
      .regex(/^[\p{L}\s'-]+$/u, "El nombre no puede tener números ni símbolos"),
    password: esCreacion
      ? z.string().min(8, "La contraseña debe tener al menos 8 caracteres")
      : z
          .string()
          .refine(
            (valor) => valor === "" || valor.length >= 8,
            "La contraseña debe tener al menos 8 caracteres",
          ),
    rol: z.enum(ROLES),
    activo: z.boolean(),
  });
}

export default function FormularioUsuario({
  usuario,
  onGuardado,
  onCerrar,
}: {
  /** Si viene, es edición; si no, alta. */
  usuario?: UsuarioResumen;
  onGuardado: () => void;
  onCerrar: () => void;
}) {
  const esCreacion = !usuario;
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(construirEsquema(esCreacion)),
    defaultValues: usuario
      ? {
          cedula: usuario.cedula,
          nombre: usuario.nombre,
          password: "",
          rol: usuario.rol,
          activo: usuario.activo,
        }
      : {
          cedula: "",
          nombre: "",
          password: "",
          rol: "Operador",
          activo: true,
        },
  });

  async function alGuardar(valores: ValoresFormulario) {
    try {
      if (usuario) {
        await actualizarUsuario(usuario.id, {
          cedula: valores.cedula.trim(),
          nombre: valores.nombre.trim(),
          rol: valores.rol,
          activo: valores.activo,
        });
        if (valores.password) {
          await cambiarPasswordUsuario(usuario.id, valores.password);
        }
      } else {
        await crearUsuario({
          cedula: valores.cedula.trim(),
          nombre: valores.nombre.trim(),
          password: valores.password,
          rol: valores.rol,
          activo: valores.activo,
        });
      }
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo={usuario ? "Editar usuario" : "Nuevo usuario"} onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <label className="campo">
          Cédula
          <input
            {...register("cedula", {
              onChange: (evento) => {
                evento.target.value = evento.target.value.replace(/\D/g, "");
              },
            })}
            inputMode="numeric"
            disabled={!!usuario}
          />
          {errors.cedula && <span style={{ color: "var(--error)" }}>{errors.cedula.message}</span>}
        </label>

        <label className="campo">
          Nombre
          <input
            {...register("nombre", {
              onChange: (evento) => {
                evento.target.value = evento.target.value.replace(/[^\p{L}\s'-]/gu, "");
              },
            })}
          />
          {errors.nombre && <span style={{ color: "var(--error)" }}>{errors.nombre.message}</span>}
        </label>

        <label className="campo">
          {esCreacion ? "Contraseña" : "Nueva contraseña (dejar en blanco para no cambiarla)"}
          <input type="password" {...register("password")} />
          {errors.password && (
            <span style={{ color: "var(--error)" }}>{errors.password.message}</span>
          )}
        </label>

        <label className="campo">
          Rol
          <select {...register("rol")}>
            {ROLES.map((rol) => (
              <option key={rol} value={rol}>
                {rol}
              </option>
            ))}
          </select>
        </label>

        <label
          style={{ display: "flex", alignItems: "center", gap: "0.4rem", color: "var(--texto)" }}
        >
          <input type="checkbox" {...register("activo")} />
          Activo
        </label>

        {errors.root && <p style={{ color: "var(--error)" }}>{errors.root.message}</p>}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
