import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { QrCode, Trash2, Upload } from "lucide-react";
import Modal from "../componentes/Modal";
import {
  cargarCarneArchivo,
  cargarCarneDesdeQr,
  claveCarne,
  quitarCarne,
  urlImagenCarne,
} from "../api/praind";
import type { CargaCarne } from "../api/praind";
import type { ContratistaConEstado } from "../api/contratistas";
import { mensajeError } from "../mensajeError";
import { nombresCoinciden, problemaArchivo } from "./CarnePraind.logica";

type Vista =
  | { tipo: "cargando" }
  | { tipo: "sin_carne" }
  | { tipo: "imagen"; url: string }
  | { tipo: "error"; mensaje: string };

/**
 * Carné PRAIND de un contratista: verlo, cargarlo (enlace del QR o imagen) y
 * quitarlo. Los equipos lo ven con la misma ruta; al cargar uno nuevo les
 * llega con la sincronización del catálogo.
 */
export default function CarnePraind({
  contratista,
  onCerrar,
}: {
  contratista: ContratistaConEstado;
  onCerrar: () => void;
}) {
  const [vista, setVista] = useState<Vista>({ tipo: "cargando" });
  const [version, setVersion] = useState(0);
  const [cargando, setCargando] = useState(false);
  const [confirmarQuitar, setConfirmarQuitar] = useState(false);
  const [quitando, setQuitando] = useState(false);
  const urlActual = useRef<string | null>(null);

  useEffect(() => {
    let vigente = true;
    claveCarne(contratista.id)
      .then(async (clave) => {
        if (!vigente) return;
        if (!clave) {
          setVista({ tipo: "sin_carne" });
          return;
        }
        const url = await urlImagenCarne(clave);
        if (!vigente) {
          URL.revokeObjectURL(url);
          return;
        }
        if (urlActual.current) URL.revokeObjectURL(urlActual.current);
        urlActual.current = url;
        setVista({ tipo: "imagen", url });
      })
      .catch((error) => {
        if (vigente) setVista({ tipo: "error", mensaje: mensajeError(error) });
      });
    return () => {
      vigente = false;
    };
  }, [contratista.id, version]);

  useEffect(
    () => () => {
      if (urlActual.current) URL.revokeObjectURL(urlActual.current);
    },
    [],
  );

  function recargarCarne() {
    setVista({ tipo: "cargando" });
    setVersion((v) => v + 1);
  }

  async function quitar() {
    setQuitando(true);
    try {
      await quitarCarne(contratista.id);
      toast.success(`Se quitó el gafete PRAIND de ${contratista.nombre}.`);
      setConfirmarQuitar(false);
      recargarCarne();
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      setQuitando(false);
    }
  }

  return (
    <Modal titulo={`Gafete PRAIND · ${contratista.nombre}`} onCerrar={onCerrar}>
      <div className="flex flex-col gap-4">
        <div
          className="flex min-h-48 items-center justify-center rounded-(--radio-chico) border border-(--borde) bg-(--panel-suave) p-2 text-center"
          aria-live="polite"
        >
          {vista.tipo === "cargando" && <p className="text-(--muted)">Cargando gafete PRAIND…</p>}
          {vista.tipo === "sin_carne" && (
            <p className="text-(--muted)">Este contratista no tiene el gafete PRAIND cargado.</p>
          )}
          {vista.tipo === "error" && <p className="text-(--error)">{vista.mensaje}</p>}
          {vista.tipo === "imagen" && (
            <a href={vista.url} target="_blank" rel="noreferrer" title="Abrir en grande">
              <img
                src={vista.url}
                alt={`Gafete PRAIND de ${contratista.nombre}`}
                className="block max-h-[60vh] max-w-full rounded-(--radio-chico)"
              />
            </a>
          )}
        </div>

        {cargando ? (
          <CargaDeCarne
            contratista={contratista}
            onCancelar={() => setCargando(false)}
            onCargado={() => {
              setCargando(false);
              toast.success(`Gafete PRAIND de ${contratista.nombre} guardado.`);
              recargarCarne();
            }}
          />
        ) : confirmarQuitar ? (
          <div className="flex flex-wrap items-center justify-end gap-2">
            <span className="mr-auto">¿Quitar el gafete PRAIND de {contratista.nombre}? Los equipos dejan de verlo.</span>
            <button type="button" className="boton" onClick={() => setConfirmarQuitar(false)} disabled={quitando}>
              Cancelar
            </button>
            <button type="button" className="boton boton-peligro" onClick={quitar} disabled={quitando}>
              {quitando ? "Quitando…" : "Sí, quitar"}
            </button>
          </div>
        ) : (
          <div className="flex flex-wrap justify-end gap-2">
            {vista.tipo === "imagen" && (
              <button type="button" className="boton boton-icono" onClick={() => setConfirmarQuitar(true)}>
                <Trash2 size={16} aria-hidden="true" />
                Quitar
              </button>
            )}
            <button type="button" className="boton boton-primario boton-icono" onClick={() => setCargando(true)}>
              <QrCode size={16} aria-hidden="true" />
              {vista.tipo === "imagen" ? "Reemplazar gafete PRAIND" : "Cargar gafete PRAIND"}
            </button>
          </div>
        )}
      </div>
    </Modal>
  );
}

function CargaDeCarne({
  contratista,
  onCancelar,
  onCargado,
}: {
  contratista: ContratistaConEstado;
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
      setError(mensajeError(e));
    } finally {
      setOcupado(false);
    }
  }

  const coincide = previa ? nombresCoinciden(previa.nombre_en_origen, contratista.nombre) : false;

  return (
    <div className="flex flex-col gap-3 border-t border-(--borde) pt-4">
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
      {error && <p className="text-(--error)">{error}</p>}

      <input
        ref={archivo}
        type="file"
        accept="image/png,image/jpeg"
        hidden
        onChange={(e) => {
          const elegido = e.target.files?.[0];
          e.target.value = "";
          if (!elegido) return;
          void correr(async () => {
            const problema = problemaArchivo(elegido);
            if (problema) throw new Error(problema);
            await cargarCarneArchivo(contratista.id, elegido);
            onCargado();
          });
        }}
      />

      <div className="flex flex-wrap justify-between gap-2">
        <button type="button" className="boton boton-icono" onClick={() => archivo.current?.click()} disabled={ocupado}>
          <Upload size={16} aria-hidden="true" />
          Elegir imagen…
        </button>
        <div className="flex gap-2">
          <button type="button" className="boton" onClick={onCancelar} disabled={ocupado}>
            Cancelar
          </button>
          {previa ? (
            <button
              type="button"
              className="boton boton-primario"
              disabled={ocupado}
              onClick={() =>
                void correr(async () => {
                  await cargarCarneDesdeQr(contratista.id, enlace, false);
                  onCargado();
                })
              }
            >
              {ocupado ? "Guardando…" : coincide ? "Guardar gafete PRAIND" : "Guardar igual"}
            </button>
          ) : (
            <button
              type="button"
              className="boton boton-primario"
              disabled={ocupado || !enlace.trim()}
              onClick={() =>
                void correr(async () => {
                  setPrevia(await cargarCarneDesdeQr(contratista.id, enlace, true));
                })
              }
            >
              {ocupado ? "Buscando…" : "Buscar gafete PRAIND"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
