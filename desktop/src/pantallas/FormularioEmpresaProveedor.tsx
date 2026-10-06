import type { ChangeEvent } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { crearEmpresaProveedor } from "../api/proveedores";
import { esquema } from "./FormularioEmpresaProveedor.logica";
import { escribirNombreEnCampo } from "../nombres";

interface ValoresFormulario {
  nombre: string;
}

export default function FormularioEmpresaProveedor({
  onGuardado,
  onCerrar,
}: {
  onGuardado: () => void;
  onCerrar: () => void;
}) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: { nombre: "" },
  });

  // En mayúscula mientras se escribe (regla del núcleo para todo nombre).
  const registroNombre = register("nombre");
  function alCambiarNombre(evento: ChangeEvent<HTMLInputElement>) {
    escribirNombreEnCampo(evento.target);
    return registroNombre.onChange(evento);
  }

  async function alGuardar(valores: ValoresFormulario) {
    try {
      // El núcleo lo guarda en mayúscula.
      await crearEmpresaProveedor(valores.nombre.trim());
      onGuardado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Nueva empresa proveedora" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <label className="campo">
          Nombre
          <input {...registroNombre} onChange={alCambiarNombre} autoFocus />
          {errors.nombre && <span style={{ color: "var(--error)" }}>{errors.nombre.message}</span>}
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
