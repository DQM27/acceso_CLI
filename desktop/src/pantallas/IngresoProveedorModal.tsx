import { useEffect, useMemo, useState } from "react";
import type { ChangeEvent } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { listarEmpresasProveedor, registrarIngresoProveedor } from "../api/proveedores";
import type { EmpresaProveedor } from "../api/proveedores";

interface ValoresFormulario {
  cedula: string;
  nombre: string;
  placa: string;
  gafete_numero: number;
}

/** Sin OCR ni catálogo de personas -- mismo criterio que el resto de esta
 * pantalla: escritorio es respaldo operativo, el flujo completo (con lector
 * de cédula) vive en mobile
 * (docs/features-futuras/plan-control-proveedores.md). */
const esquema = z.object({
  cedula: z.string().min(1, "La cédula es obligatoria"),
  nombre: z.string().min(1, "El nombre es obligatorio"),
  placa: z.string(),
  gafete_numero: z
    .number()
    .refine((n) => Number.isInteger(n) && n > 0, "El número de gafete es obligatorio"),
});

export default function IngresoProveedorModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: {
      cedula: "",
      nombre: "",
      placa: "",
      // Antes NaN (con `type="number"` el DOM lo mostraba vacío solo).
      // Con `type="text"` (ver abajo) un valor NaN se refleja literal como
      // el string "NaN" en el campo -- string vacío evita ese problema, y
      // `setValueAs` (abajo) lo convierte de vuelta a NaN si el campo queda
      // vacío al validar.
      gafete_numero: "" as unknown as number,
    },
  });

  // `type="number"` deja teclear "e"/"-"/"+" (notación científica) aunque
  // el campo sea un entero positivo -- mismo criterio que
  // FormularioGafete.tsx: texto + filtrado en onChange, en vez de confiar
  // en el input nativo (docs/pendientes.md, "Auditar máscaras de entrada").
  const registroGafeteNumero = register("gafete_numero", {
    setValueAs: (valor: string) => (valor === "" ? Number.NaN : Number(valor)),
  });
  const alCambiarGafeteNumero = (evento: ChangeEvent<HTMLInputElement>) => {
    evento.target.value = evento.target.value.replace(/\D/g, "");
    registroGafeteNumero.onChange(evento);
  };

  const [empresas, setEmpresas] = useState<EmpresaProveedor[]>([]);
  useEffect(() => {
    // El núcleo filtra las inactivas (`soloActivos: true`) -- no queda del
    // lado de la pantalla decidir eso.
    listarEmpresasProveedor(true)
      .then(setEmpresas)
      .catch(() => {});
  }, []);

  // Combobox nativo -- `<input list>` + `<datalist>` en vez del buscador con
  // lista flotante que usan Rutas/Encargados (portal + blur con `setTimeout`
  // + navegación con flechas): ese mecanismo resultó frágil en la práctica
  // (el click sobre un resultado no siempre alcanzaba a registrarse antes de
  // que el blur cerrara la lista) y, a diferencia de un `<select size={N}>`,
  // el desplegable del navegador es un overlay -- no reserva espacio ni
  // cambia el layout del modal aunque el catálogo tenga muchas empresas. El
  // filtrado y el límite de opciones visibles los resuelve el propio
  // navegador; acá sólo hace falta convertir el texto elegido de vuelta a un
  // id real, sin catálogo la persona pudo haber tecleado cualquier cosa que
  // no calce con ninguna empresa.
  const [empresaTexto, setEmpresaTexto] = useState("");
  const [errorEmpresa, setErrorEmpresa] = useState<string | null>(null);

  const empresaId = useMemo(() => {
    const texto = empresaTexto.trim().toLowerCase();
    if (!texto) return null;
    return empresas.find((empresa) => empresa.nombre.toLowerCase() === texto)?.id ?? null;
  }, [empresaTexto, empresas]);

  async function alGuardar(valores: ValoresFormulario) {
    if (!empresaId) {
      setErrorEmpresa("Elija una empresa del catálogo");
      return;
    }
    setErrorEmpresa(null);
    try {
      await registrarIngresoProveedor({
        cedula: valores.cedula.trim(),
        nombre: valores.nombre.trim(),
        empresa_id: empresaId,
        placa: valores.placa.trim() || null,
        gafete_numero: valores.gafete_numero,
      });
      onRegistrado();
    } catch (error) {
      setError("root", { message: String(error) });
    }
  }

  return (
    <Modal titulo="Nuevo ingreso de proveedor" onCerrar={onCerrar}>
      <form
        onSubmit={handleSubmit(alGuardar)}
        style={{ display: "flex", flexDirection: "column", gap: "0.75rem", width: "26rem", maxWidth: "100%" }}
      >
        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            Cédula
            <input {...register("cedula")} autoFocus autoComplete="off" />
            {errors.cedula && <span style={{ color: "var(--error)" }}>{errors.cedula.message}</span>}
          </label>
          <label className="campo" style={{ flex: 1.4 }}>
            Nombre
            <input {...register("nombre")} autoComplete="off" />
            {errors.nombre && <span style={{ color: "var(--error)" }}>{errors.nombre.message}</span>}
          </label>
        </div>

        <label className="campo">
          Empresa
          <input
            list="empresas-proveedor-datalist"
            value={empresaTexto}
            onChange={(evento) => setEmpresaTexto(evento.target.value)}
            autoComplete="off"
            placeholder="Escriba para buscar en el catálogo…"
          />
          <datalist id="empresas-proveedor-datalist">
            {empresas.map((empresa) => (
              <option key={empresa.id} value={empresa.nombre} />
            ))}
          </datalist>
        </label>
        {errorEmpresa && <span style={{ color: "var(--error)" }}>{errorEmpresa}</span>}

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            Placa (opcional)
            <input {...register("placa")} autoComplete="off" placeholder="Caminando si se deja en blanco" />
          </label>
          <label className="campo" style={{ flex: "0 1 8rem" }}>
            N.° de gafete
            <input
              {...registroGafeteNumero}
              onChange={alCambiarGafeteNumero}
              inputMode="numeric"
            />
            {errors.gafete_numero && (
              <span style={{ color: "var(--error)" }}>{errors.gafete_numero.message}</span>
            )}
          </label>
        </div>

        {errors.root && <p style={{ color: "var(--error)" }}>{errors.root.message}</p>}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCerrar}>
            Cancelar
          </button>
          <button type="submit" className="boton boton-primario" disabled={isSubmitting}>
            {isSubmitting ? "Registrando…" : "Registrar ingreso"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
