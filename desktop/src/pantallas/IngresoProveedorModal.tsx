import { useEffect, useMemo, useState } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import { listarEmpresasProveedor, registrarIngresoProveedor } from "../api/proveedores";
import type { EmpresaProveedor } from "../api/proveedores";

const MAX_RESULTADOS = 6;

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
      gafete_numero: Number.NaN,
    },
  });

  const [empresas, setEmpresas] = useState<EmpresaProveedor[]>([]);
  useEffect(() => {
    listarEmpresasProveedor()
      .then((datos) => setEmpresas(datos.filter((empresa) => empresa.activo)))
      .catch(() => {});
  }, []);

  // Combobox simple -- un `<select>` nativo en vez del buscador con lista
  // flotante que usan Rutas/Encargados: acá el catálogo de empresas
  // proveedoras es chico y ese mecanismo (portal + blur con `setTimeout` +
  // navegación con flechas) resultó frágil en la práctica -- el click sobre
  // un resultado no siempre alcanzaba a registrarse antes de que el blur del
  // campo cerrara la lista. Un `<select>` nativo no tiene esa carrera: el
  // navegador maneja el click/selección solo, sin lógica propia que pueda
  // desincronizarse. `filtro` sólo acota qué opciones aparecen (máximo
  // `MAX_RESULTADOS`), el valor real sigue siendo `empresaId`.
  const [filtroEmpresa, setFiltroEmpresa] = useState("");
  const [empresaId, setEmpresaId] = useState<number | null>(null);
  const [errorEmpresa, setErrorEmpresa] = useState<string | null>(null);

  const resultadosEmpresa = useMemo(() => {
    const texto = filtroEmpresa.trim().toLowerCase();
    const filtradas = texto
      ? empresas.filter((empresa) => empresa.nombre.toLowerCase().includes(texto))
      : empresas;
    return filtradas.slice(0, MAX_RESULTADOS);
  }, [filtroEmpresa, empresas]);

  // Si el filtro cambia y la empresa ya elegida deja de estar en las
  // opciones visibles, el `<select>` la pierde de todos modos (el navegador
  // no puede mostrar seleccionada una `<option>` que ya no existe) -- limpiar
  // el estado acá evita que quede un `empresaId` "fantasma" sin reflejo en
  // pantalla.
  useEffect(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setEmpresaId` directo
    // -- ver el mismo comentario en Activos.tsx.
    Promise.resolve().then(() => {
      setEmpresaId((actual) =>
        actual !== null && resultadosEmpresa.some((empresa) => empresa.id === actual)
          ? actual
          : null,
      );
    });
  }, [resultadosEmpresa]);

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
            value={filtroEmpresa}
            onChange={(evento) => setFiltroEmpresa(evento.target.value)}
            autoComplete="off"
            placeholder="Escriba para filtrar…"
          />
        </label>
        <label className="campo">
          <select
            size={Math.min(MAX_RESULTADOS, Math.max(resultadosEmpresa.length, 1))}
            value={empresaId ?? ""}
            onChange={(evento) => setEmpresaId(Number(evento.target.value))}
          >
            {resultadosEmpresa.length === 0 ? (
              <option value="" disabled>
                Sin resultados
              </option>
            ) : (
              resultadosEmpresa.map((empresa) => (
                <option key={empresa.id} value={empresa.id}>
                  {empresa.nombre}
                </option>
              ))
            )}
          </select>
        </label>
        {errorEmpresa && <span style={{ color: "var(--error)" }}>{errorEmpresa}</span>}

        <div style={{ display: "flex", gap: "0.75rem" }}>
          <label className="campo" style={{ flex: 1 }}>
            Placa (opcional)
            <input {...register("placa")} autoComplete="off" placeholder="Llegó a pie si se deja en blanco" />
          </label>
          <label className="campo" style={{ flex: "0 1 8rem" }}>
            N.° de gafete
            <input type="number" min={1} {...register("gafete_numero", { valueAsNumber: true })} />
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
