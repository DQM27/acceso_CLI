import { useEffect, useMemo, useState } from "react";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import Modal from "../componentes/Modal";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import { listarEmpresasProveedor, registrarIngresoProveedor } from "../api/proveedores";
import type { EmpresaProveedor } from "../api/proveedores";

const MAX_RESULTADOS = 6;
const CIERRE_LISTA_MS = 120;

interface ValoresFormulario {
  cedula: string;
  nombre: string;
  empresa_texto: string;
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
  empresa_texto: z.string().min(1, "Elija una empresa del catálogo"),
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
    watch,
    setValue,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ValoresFormulario>({
    resolver: zodResolver(esquema),
    defaultValues: {
      cedula: "",
      nombre: "",
      empresa_texto: "",
      placa: "",
      gafete_numero: Number.NaN,
    },
  });

  const [empresas, setEmpresas] = useState<EmpresaProveedor[]>([]);
  const [empresaId, setEmpresaId] = useState<number | null>(null);
  useEffect(() => {
    listarEmpresasProveedor()
      .then((datos) => setEmpresas(datos.filter((empresa) => empresa.activo)))
      .catch(() => {});
  }, []);

  const empresaTexto = watch("empresa_texto");
  const [campoEmpresaEnfocado, setCampoEmpresaEnfocado] = useState(false);

  // Si el texto ya no coincide con la empresa elegida (la borró o siguió
  // escribiendo), hay que volver a exigir una elección del catálogo antes
  // de guardar -- mismo criterio que impide mandar un `empresa_id` viejo.
  useEffect(() => {
    setEmpresaId((actual) => {
      if (actual === null) return actual;
      const seleccionada = empresas.find((empresa) => empresa.id === actual);
      return seleccionada && seleccionada.nombre === empresaTexto ? actual : null;
    });
  }, [empresaTexto, empresas]);

  const resultadosEmpresa = useMemo(() => {
    const texto = empresaTexto.trim().toLowerCase();
    if (!texto) return [];
    return empresas
      .filter((empresa) => empresa.nombre.toLowerCase().includes(texto))
      .slice(0, MAX_RESULTADOS);
  }, [empresaTexto, empresas]);

  const listaEmpresaVisible = campoEmpresaEnfocado && resultadosEmpresa.length > 0;
  const { campoRef: campoEmpresaRef, posicion: posicionEmpresa } =
    useListaFlotante(listaEmpresaVisible);

  function elegirEmpresa(empresa: EmpresaProveedor) {
    setValue("empresa_texto", empresa.nombre);
    setEmpresaId(empresa.id);
    setCampoEmpresaEnfocado(false);
  }

  const {
    resaltado: resaltadoEmpresa,
    setResaltado: setResaltadoEmpresa,
    manejarTecla: manejarTeclaEmpresa,
  } = useNavegacionFlechas(resultadosEmpresa, listaEmpresaVisible, elegirEmpresa);

  async function alGuardar(valores: ValoresFormulario) {
    if (!empresaId) {
      setError("empresa_texto", { message: "Elija una empresa del catálogo" });
      return;
    }
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

        <div ref={campoEmpresaRef}>
          <label className="campo">
            Empresa
            <input
              {...register("empresa_texto")}
              autoComplete="off"
              placeholder="Escriba para buscar en el catálogo…"
              onFocus={() => setCampoEmpresaEnfocado(true)}
              onBlur={() => setTimeout(() => setCampoEmpresaEnfocado(false), CIERRE_LISTA_MS)}
              onKeyDown={manejarTeclaEmpresa}
            />
          </label>
          {errors.empresa_texto && (
            <span style={{ color: "var(--error)" }}>{errors.empresa_texto.message}</span>
          )}
        </div>
        {listaEmpresaVisible && posicionEmpresa && (
          <ListaFlotante posicion={posicionEmpresa}>
            {resultadosEmpresa.length === 0 && <SinResultados />}
            {resultadosEmpresa.map((empresa, indice) => (
              <FilaListaFlotante
                key={empresa.id}
                resaltada={indice === resaltadoEmpresa}
                onClick={() => elegirEmpresa(empresa)}
                onMouseEnter={() => setResaltadoEmpresa(indice)}
              >
                <span>{empresa.nombre}</span>
              </FilaListaFlotante>
            ))}
          </ListaFlotante>
        )}

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
