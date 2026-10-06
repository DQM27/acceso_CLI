import { useState } from "react";
import { toast } from "sonner";
import Modal from "../componentes/Modal";
import { editarUsuario } from "../api/usuarios";
import type { Usuario } from "../api/usuarios";
import { reglas } from "../reglas";
import { sanearSoloLetras } from "../validacion";
import { mensajeError } from "../mensajeError";

/** Edición de nombre y rol. La cédula se muestra pero no se cambia: es con
 * la que la persona inicia sesión (y su cuenta de Supabase Auth). A un ROOT
 * no se le cambia el rol desde acá. El rol hoy no restringe nada dentro de la
 * app (aplanado de roles), queda como etiqueta. */
export default function FormularioEditarUsuario({
  usuario,
  onGuardado,
  onCerrar,
}: {
  usuario: Usuario;
  onGuardado: () => void;
  onCerrar: () => void;
}) {
  const [nombre, setNombre] = useState(usuario.nombre);
  const [rol, setRol] = useState(usuario.rol);
  const [guardando, setGuardando] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const esRoot = usuario.rol === "ROOT";

  async function alEnviar(evento: React.FormEvent) {
    evento.preventDefault();
    setGuardando(true);
    setError(null);
    try {
      const limpio = nombre.trim();
      await editarUsuario(usuario.id, {
        nombre: limpio,
        rol: esRoot ? undefined : (rol as "ADMINISTRADOR" | "OPERADOR"),
      });
      toast.success(`${limpio} actualizado.`);
      onGuardado();
    } catch (fallo) {
      setError(mensajeError(fallo));
    } finally {
      setGuardando(false);
    }
  }

  return (
    <Modal titulo="Editar usuario" onCerrar={onCerrar}>
      <form onSubmit={alEnviar} className="flex flex-col gap-3">
        <label className="campo">
          Cédula
          <input value={usuario.cedula} disabled readOnly />
        </label>
        <p className="m-0 text-[0.8rem] text-muted">
          La cédula no se cambia: es con la que la persona inicia sesión.
        </p>

        <label className="campo">
          Nombre
          <input
            required
            autoFocus
            value={nombre}
            disabled={guardando}
            onChange={(evento) => setNombre(reglas.nombreMientrasSeEscribe(sanearSoloLetras(evento.target.value)))}
          />
        </label>

        <label className="campo">
          Rol
          {esRoot ? (
            <input value="ROOT" disabled readOnly />
          ) : (
            <select
              value={rol}
              disabled={guardando}
              onChange={(evento) => setRol(evento.target.value as Usuario["rol"])}
            >
              <option value="OPERADOR">Operador</option>
              <option value="ADMINISTRADOR">Administrador</option>
            </select>
          )}
        </label>

        {error && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2">
          <button type="button" className="boton" disabled={guardando} onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={guardando}>
            {guardando ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
