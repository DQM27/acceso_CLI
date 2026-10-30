import { useState } from "react";
import type { ChangeEvent } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import {
  crearEmpresaProveedor,
  empresasProveedorParecidas,
} from "../api/proveedores";
import type { EmpresaProveedor } from "../api/proveedores";
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

  // Antes de crear, las que se parecen ("FUDDY FACTORY" → "FOODIE FACTORY"):
  // si hay, se muestran y hace falta confirmar que es otra empresa. El
  // nombre confirmado se recuerda para no volver a preguntar.
  const [parecidas, setParecidas] = useState<EmpresaProveedor[]>([]);
  const [confirmadoPara, setConfirmadoPara] = useState<string | null>(null);

  async function alGuardar(valores: ValoresFormulario) {
    const nombre = valores.nombre.trim();
    try {
      if (confirmadoPara !== nombre) {
        const encontradas = await empresasProveedorParecidas(nombre);
        if (encontradas.length > 0) {
          setParecidas(encontradas);
          setConfirmadoPara(nombre);
          return;
        }
      }
      // El núcleo lo guarda en mayúscula.
      await crearEmpresaProveedor(nombre);
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
          {errors.nombre && (
            <span style={{ color: "var(--error)" }}>
              {errors.nombre.message}
            </span>
          )}
        </label>

        {errors.root && (
          <p style={{ color: "var(--error)" }}>{errors.root.message}</p>
        )}

        {parecidas.length > 0 && (
          <div role="alert" className="aviso-informativo">
            <p style={{ margin: 0 }}>
              Ya existen empresas con un nombre parecido:
            </p>
            <ul style={{ margin: "0.25rem 0" }}>
              {parecidas.map((empresa) => (
                <li key={empresa.id}>{empresa.nombre}</li>
              ))}
            </ul>
            <p style={{ margin: 0 }}>
              Si es alguna de estas, no hace falta crearla. Si es otra empresa,
              guarde de nuevo.
            </p>
          </div>
        )}

        <div
          style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}
        >
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button
            type="submit"
            className="boton boton-primario"
            disabled={isSubmitting}
          >
            {isSubmitting
              ? "Guardando…"
              : parecidas.length > 0
                ? "Es otra empresa, crearla"
                : "Guardar"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
