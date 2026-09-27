import { useCallback, useEffect, useMemo, useState } from "react";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import Tabla from "../componentes/Tabla";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioContratista from "./FormularioContratista";
import { actualizarContratista, buscarContratistas, listarEmpresasSeleccionables } from "../api";
import type { ContratistaResumen, Empresa, RolUsuario } from "../api";
import { columnasPara } from "./Contratistas.logica";

export default function Contratistas({ actorRol }: { actorRol: RolUsuario }) {
  const [empresas, setEmpresas] = useState<Empresa[]>([]);
  const [busqueda, setBusqueda] = useState("");
  const [filas, setFilas] = useState<ContratistaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | ContratistaResumen | null>(
    null,
  );

  // useMemo -- mismo motivo que en Historial.tsx: si `columnas` se recrea en
  // cada render, AG Grid reaplica el orden/ancho literales encima del layout
  // que la persona ya acomodó (persistido en localStorage vía `Tabla`).
  const columnas = useMemo(() => columnasPara(actorRol), [actorRol]);

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} resultado(s)`);

  useEffect(() => {
    // Sólo activas -- una empresa desactivada no es una opción válida en el
    // desplegable del formulario. `FormularioContratista` agrega aparte la
    // empresa actual del contratista en edición si ya está desactivada, para
    // no perderla de vista ni reasignarla en silencio.
    listarEmpresasSeleccionables()
      .then(setEmpresas)
      .catch((error) => toast.error(String(error)));
  }, []);

  const recargar = useCallback((estaVigente: () => boolean = () => true) => {
    setCargando(true);
    return buscarContratistas()
      .then((pagina) => {
        if (!estaVigente()) return;
        setFilas(pagina.items);
      })
      .finally(() => {
        if (estaVigente()) setCargando(false);
      });
  }, []);

  useCargaAlCambiar(recargar, true);

  async function manejarEdicion(fila: ContratistaResumen) {
    try {
      await actualizarContratista(fila.id, {
        cedula: fila.cedula,
        nombre: fila.nombre,
        empresa_id: fila.empresa_id,
        tipo_ingreso: fila.tipo_ingreso,
        fecha_vencimiento_praind: fila.fecha_vencimiento_praind,
        es_personal_ruta: fila.es_personal_ruta,
        tiene_acceso: fila.tiene_acceso,
      });
    } catch (error) {
      // La grilla ya muestra el valor nuevo (edición optimista de AG Grid) —
      // si el guardado falla, hay que volver a pedir los datos reales para
      // que la celda no quede mintiendo.
      toast.error(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<ContratistaResumen>
            cargando={cargando}
            id="contratistas"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            controles={
              <>
                <button
                  type="button"
                  className="boton boton-icono"
                  title="Nuevo contratista"
                  aria-label="Nuevo contratista"
                  onClick={() => setFormularioAbierto("crear")}
                >
                  <Plus size={16} aria-hidden="true" />
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula o nombre…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
          />
        </div>
      </div>

      {formularioAbierto && (
        <FormularioContratista
          contratista={formularioAbierto === "crear" ? undefined : formularioAbierto}
          empresas={empresas}
          onCerrar={() => setFormularioAbierto(null)}
          onGuardado={() => {
            setFormularioAbierto(null);
            recargar();
          }}
        />
      )}
    </div>
  );
}
