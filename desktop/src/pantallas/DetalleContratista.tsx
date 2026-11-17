import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Pencil, QrCode } from "lucide-react";
import Modal from "../componentes/Modal";
import {
  cargarCarnePraindArchivo,
  cargarCarnePraindDesdeQr,
  estadoCarnePraind,
  permisosCarnePraind,
  textoTipoIngreso,
  verCarnePraind,
} from "../api";
import type { CargaCarne, ContratistaResumen, PermisosCarne } from "../api";
import { textoFechaDDMMYYYY } from "../tiempo";
import { colorEstadoAcceso, textoEstadoAcceso } from "./Contratistas.logica";
import { nombresCoinciden, problemaArchivo, tipoDeClave } from "./DetalleContratista.logica";

type VistaCarne =
  | { tipo: "cargando" }
  | { tipo: "sin_carne" }
  | { tipo: "imagen"; url: string }
  | { tipo: "error"; mensaje: string };

/**
 * Detalles de un contratista con su carné PRAIND. El carné se baja recién
 * al abrir esta vista y queda guardado en el equipo: la próxima vez se ve al
 * instante y sin internet. Cargar uno nuevo sólo aparece si la nube lo
 * permite a este equipo (lo decide el panel).
 */
export default function DetalleContratista({
  contratista,
  onEditar,
  onCerrar,
}: {
  contratista: ContratistaResumen;
  onEditar: () => void;
  onCerrar: () => void;
}) {
  const [vista, setVista] = useState<VistaCarne>({ tipo: "cargando" });
  const [ampliado, setAmpliado] = useState(false);
  const [permisos, setPermisos] = useState<PermisosCarne | null>(null);
  const [cargando, setCargando] = useState(false);
  // Sube después de cargar un carné: vuelve a pedir el que quedó.
  const [version, setVersion] = useState(0);
  const urlActual = useRef<string | null>(null);

  useEffect(() => {
    let vigente = true;
    estadoCarnePraind(contratista.id)
      .then(async (estado) => {
        if (!vigente) return;
        if (!estado.clave) {
          setVista({ tipo: "sin_carne" });
          return;
        }
        const bytes = await verCarnePraind(contratista.id);
        if (!vigente) return;
        const url = URL.createObjectURL(new Blob([bytes], { type: tipoDeClave(estado.clave) }));
        if (urlActual.current) URL.revokeObjectURL(urlActual.current);
        urlActual.current = url;
        setVista({ tipo: "imagen", url });
      })
      .catch((error) => {
        if (vigente) setVista({ tipo: "error", mensaje: String(error) });
      });
    return () => {
      vigente = false;
    };
  }, [contratista.id, version]);

  useEffect(() => {
    let vigente = true;
    permisosCarnePraind()
      .then((p) => {
        if (vigente) setPermisos(p);
      })
      .catch(() => {
        // Sin permisos no se ofrece cargar; ver lo guardado no depende de esto.
      });
    return () => {
      vigente = false;
    };
  }, []);

  useEffect(
    () => () => {
      if (urlActual.current) URL.revokeObjectURL(urlActual.current);
    },
    [],
  );

  const estadoAcceso = textoEstadoAcceso(contratista);

  return (
    <Modal titulo="Detalles del contratista" onCerrar={onCerrar} ancho="56rem">
      <div className="detalle-contratista">
        {/* Datos arriba; acciones (o la carga del gafete PRAIND) abajo, en el
            espacio libre de la columna: el modal no crece ni hace scroll. */}
        <div className="detalle-contratista-izquierda">
          <dl className="detalle-contratista-datos">
            <dt>Nombre</dt>
            <dd>{contratista.nombre}</dd>
            <dt>Cédula</dt>
            <dd>{contratista.cedula}</dd>
            <dt>Empresa</dt>
            <dd>{contratista.empresa_nombre}</dd>
            <dt>Tipo de ingreso</dt>
            <dd>{textoTipoIngreso(contratista.tipo_ingreso)}</dd>
            <dt>Vence PRAIND</dt>
            <dd>
              {contratista.fecha_vencimiento_praind ? textoFechaDDMMYYYY(contratista.fecha_vencimiento_praind) : "—"}
            </dd>
            {estadoAcceso && (
              <>
                <dt>Estado</dt>
                <dd style={{ color: colorEstadoAcceso(contratista.estado_acceso) }}>{estadoAcceso}</dd>
              </>
            )}
          </dl>

          {cargando ? (
            <CargaDeCarne
              contratista={contratista}
              onCancelar={() => setCargando(false)}
              onCargado={() => {
                setCargando(false);
                setVista({ tipo: "cargando" });
                setVersion((v) => v + 1);
              }}
            />
          ) : (
            <div className="detalle-contratista-acciones">
              {permisos?.puede_cargar && (
                <button type="button" className="boton boton-icono" onClick={() => setCargando(true)}>
                  <QrCode size={16} aria-hidden="true" />
                  {vista.tipo === "imagen" ? "Reemplazar gafete PRAIND" : "Cargar gafete PRAIND"}
                </button>
              )}
              <button type="button" className="boton boton-primario boton-icono" onClick={onEditar}>
                <Pencil size={16} aria-hidden="true" />
                Editar
              </button>
            </div>
          )}
        </div>

        <div className="detalle-contratista-carne" aria-live="polite">
          {vista.tipo === "cargando" && <p className="texto-tenue">Cargando gafete PRAIND…</p>}
          {vista.tipo === "sin_carne" && (
            <p className="texto-tenue">Este contratista no tiene el gafete PRAIND cargado.</p>
          )}
          {vista.tipo === "error" && <p style={{ color: "var(--error)" }}>{vista.mensaje}</p>}
          {vista.tipo === "imagen" && (
            <button
              type="button"
              className="detalle-contratista-imagen"
              onClick={() => setAmpliado(true)}
              title="Ver en grande"
            >
              <img src={vista.url} alt={`Gafete PRAIND de ${contratista.nombre}`} />
            </button>
          )}
        </div>
      </div>

      {/* En un portal al <body>: adentro del modal de detalles, la animación
          de entrada (transform) hacía que esta ventana se midiera contra ese
          modal y no contra la pantalla, y quedaba chica y con scroll. */}
      {ampliado &&
        vista.tipo === "imagen" &&
        createPortal(
          <Modal titulo={`Gafete PRAIND · ${contratista.nombre}`} onCerrar={() => setAmpliado(false)} ancho="60rem">
            <img
              className="detalle-contratista-ampliado"
              src={vista.url}
              alt={`Gafete PRAIND de ${contratista.nombre}`}
            />
          </Modal>,
          document.body,
        )}
    </Modal>
  );
}

/** Cargar el carné con el enlace del QR (se compara el nombre antes de
 * guardar) o eligiendo la imagen. */
function CargaDeCarne({
  contratista,
  onCancelar,
  onCargado,
}: {
  contratista: ContratistaResumen;
  onCancelar: () => void;
  onCargado: () => void;
}) {
  const [enlace, setEnlace] = useState("");
  const [previa, setPrevia] = useState<CargaCarne | null>(null);
  const [ocupado, setOcupado] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const archivo = useRef<HTMLInputElement>(null);

  async function correr(tarea: () => Promise<void>) {
    setOcupado(true);
    setError(null);
    try {
      await tarea();
    } catch (e) {
      setError(String(e));
    } finally {
      setOcupado(false);
    }
  }

  const buscar = () =>
    correr(async () => {
      setPrevia(await cargarCarnePraindDesdeQr(contratista.id, enlace, true));
    });

  const guardar = () =>
    correr(async () => {
      await cargarCarnePraindDesdeQr(contratista.id, enlace, false);
      onCargado();
    });

  const subir = (elegido: File) =>
    correr(async () => {
      const problema = problemaArchivo(elegido);
      if (problema) throw problema;
      await cargarCarnePraindArchivo(contratista.id, new Uint8Array(await elegido.arrayBuffer()));
      onCargado();
    });

  const coincide = previa ? nombresCoinciden(previa.nombre_en_origen, contratista.nombre) : false;

  return (
    <div className="detalle-contratista-carga">
      <label className="campo">
        Enlace del QR del gafete PRAIND
        <input
          value={enlace}
          onChange={(e) => {
            setEnlace(e.target.value);
            setPrevia(null);
          }}
          placeholder="https://prd.pw/…"
          disabled={ocupado}
          autoFocus
        />
      </label>

      {previa && (
        <p style={{ color: coincide ? "var(--exito)" : "var(--advertencia)" }}>
          {coincide
            ? `El gafete PRAIND es de ${previa.nombre_en_origen}.`
            : `El gafete PRAIND es de «${previa.nombre_en_origen ?? "nombre desconocido"}», que no coincide con ${contratista.nombre}. Revisá que sea el gafete PRAIND correcto.`}
        </p>
      )}
      {error && <p style={{ color: "var(--error)" }}>{error}</p>}

      <input
        ref={archivo}
        type="file"
        accept="image/png,image/jpeg"
        hidden
        onChange={(e) => {
          const elegido = e.target.files?.[0];
          e.target.value = "";
          if (elegido) subir(elegido);
        }}
      />

      <div className="detalle-contratista-carga-botones">
        <button type="button" className="boton" onClick={() => archivo.current?.click()} disabled={ocupado}>
          Elegir imagen…
        </button>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button type="button" className="boton" onClick={onCancelar} disabled={ocupado}>
            Cancelar
          </button>
          {previa ? (
            <button type="button" className="boton boton-primario" onClick={guardar} disabled={ocupado}>
              {ocupado ? "Guardando…" : coincide ? "Guardar PRAIND" : "Guardar igual"}
            </button>
          ) : (
            <button
              type="button"
              className="boton boton-primario"
              onClick={buscar}
              disabled={ocupado || !enlace.trim()}
            >
              {ocupado ? "Buscando…" : "Buscar PRAIND"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
