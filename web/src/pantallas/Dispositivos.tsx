import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import {
  listarDispositivosYSitios,
  moverDispositivo,
  provisionarDispositivo,
  revocarDispositivo,
} from "../api/dispositivos";
import type { Dispositivo, DispositivoProvisionado, TipoDispositivo } from "../api/dispositivos";

const ETIQUETAS_TIPO: Record<TipoDispositivo, string> = {
  pc: "PC",
  mobile: "Celular",
  visor: "Visor web (solo lectura)",
};

interface FilaDispositivo extends Dispositivo {
  sitio_nombre: string;
}

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/**
 * Alta/baja/cambio de sitio de dispositivos -- reemplaza
 * `admin-panel/panel-dispositivos.html` (clave compartida, sin saber quién
 * hizo qué) por esta pantalla dentro del panel nuevo, autenticada con la
 * misma sesión de Google que el resto (ver `api/dispositivos.ts`). El
 * secreto de un dispositivo nuevo se muestra UNA sola vez al crearlo -- no
 * queda guardado en texto plano en ningún lado que se pueda volver a leer,
 * ni siquiera acá. "Cambiar sitio" reasigna `dispositivos.sitio_id` --
 * hueco que no existía antes (docs/plan-panel-administrativo-web.md).
 */
export default function Dispositivos() {
  const [sitios, setSitios] = useState<{ id: string; nombre: string }[]>([]);
  const [dispositivos, setDispositivos] = useState<Dispositivo[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);
  const [provisionado, setProvisionado] = useState<DispositivoProvisionado | null>(null);

  const [sitioNombre, setSitioNombre] = useState("");
  const [sitioDireccion, setSitioDireccion] = useState("");
  const [tipo, setTipo] = useState<TipoDispositivo>("pc");
  const [etiqueta, setEtiqueta] = useState("");

  const [dispositivoAMover, setDispositivoAMover] = useState<FilaDispositivo | null>(null);
  const [sitioNombreMover, setSitioNombreMover] = useState("");
  const [sitioDireccionMover, setSitioDireccionMover] = useState("");
  const [moviendo, setMoviendo] = useState(false);
  const [errorMover, setErrorMover] = useState<string | null>(null);

  const recargar = useCallback(() => {
    setCargando(true);
    return listarDispositivosYSitios()
      .then(({ sitios, dispositivos }) => {
        setSitios(sitios);
        setDispositivos(dispositivos);
      })
      .catch((error) => toast.error(String(error)))
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    recargar();
  }, [recargar]);

  const nombrePorSitio = useMemo(() => {
    const mapa = new Map(sitios.map((s) => [s.id, s.nombre]));
    return (sitioId: string) => mapa.get(sitioId) ?? "?";
  }, [sitios]);

  const filas: FilaDispositivo[] = useMemo(
    () => dispositivos.map((d) => ({ ...d, sitio_nombre: nombrePorSitio(d.sitio_id) })),
    [dispositivos, nombrePorSitio],
  );

  function cerrarModal() {
    setModalAbierto(false);
    setSitioNombre("");
    setSitioDireccion("");
    setTipo("pc");
    setEtiqueta("");
    setErrorForm(null);
    setProvisionado(null);
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    setCreando(true);
    setErrorForm(null);
    try {
      const resultado = await provisionarDispositivo({
        sitio_nombre: sitioNombre.trim(),
        sitio_direccion: sitioDireccion.trim() || undefined,
        tipo,
        etiqueta: etiqueta.trim(),
      });
      setProvisionado(resultado);
      recargar();
    } catch (error) {
      setErrorForm(String(error));
    } finally {
      setCreando(false);
    }
  }

  async function alRevocar(fila: FilaDispositivo) {
    if (!confirm(`¿Revocar "${fila.etiqueta}"? Ese dispositivo va a dejar de poder sincronizar.`)) return;
    try {
      await revocarDispositivo(fila.id);
      toast.success(`${fila.etiqueta} revocado.`);
      recargar();
    } catch (error) {
      toast.error(String(error));
    }
  }

  function abrirMover(fila: FilaDispositivo) {
    setDispositivoAMover(fila);
    setSitioNombreMover(fila.sitio_nombre);
    setSitioDireccionMover("");
    setErrorMover(null);
  }

  function cerrarMover() {
    setDispositivoAMover(null);
    setSitioNombreMover("");
    setSitioDireccionMover("");
    setErrorMover(null);
  }

  async function alConfirmarMover(evento: React.FormEvent) {
    evento.preventDefault();
    if (!dispositivoAMover) return;
    setMoviendo(true);
    setErrorMover(null);
    try {
      const { sitio_nombre } = await moverDispositivo(dispositivoAMover.id, {
        sitio_nombre: sitioNombreMover.trim(),
        sitio_direccion: sitioDireccionMover.trim() || undefined,
      });
      toast.success(`${dispositivoAMover.etiqueta} ahora es de ${sitio_nombre}.`);
      cerrarMover();
      recargar();
    } catch (error) {
      setErrorMover(String(error));
    } finally {
      setMoviendo(false);
    }
  }

  async function copiarSecreto(secret: string) {
    try {
      await navigator.clipboard.writeText(secret);
      toast.success("Secreto copiado.");
    } catch {
      toast.error("No se pudo copiar -- seleccioná el texto a mano.");
    }
  }

  const columnas: ColDef<FilaDispositivo>[] = [
    { field: "etiqueta", headerName: "Etiqueta", flex: 1.6, minWidth: 180, cellStyle: { textAlign: "left" } },
    {
      field: "tipo",
      headerName: "Tipo",
      flex: 1,
      minWidth: 150,
      valueFormatter: ({ value }) => ETIQUETAS_TIPO[value as TipoDispositivo],
    },
    { field: "sitio_nombre", headerName: "Sitio", flex: 1, minWidth: 130 },
    {
      field: "created_at",
      headerName: "Creado",
      flex: 1.2,
      minWidth: 160,
      valueFormatter: ({ value }) => textoFechaHora(value),
    },
    {
      field: "revoked_at",
      headerName: "Estado",
      flex: 0.9,
      minWidth: 110,
      filter: false,
      cellRenderer: ({ value }: { value: string | null }) => (
        <span
          className="chip"
          style={{ ["--chip-color" as string]: value ? "var(--error)" : "var(--exito)" }}
        >
          {value ? "Revocado" : "Activo"}
        </span>
      ),
    },
    {
      colId: "acciones",
      headerName: "",
      flex: 1.4,
      minWidth: 190,
      sortable: false,
      filter: false,
      cellRenderer: ({ data }: { data: FilaDispositivo }) => (
        <div style={{ display: "flex", gap: "0.4rem" }}>
          <button
            type="button"
            className="boton"
            style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
            onClick={() => abrirMover(data)}
          >
            Cambiar sitio
          </button>
          {!data.revoked_at && (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
              onClick={() => alRevocar(data)}
            >
              Revocar
            </button>
          )}
        </div>
      ),
    },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaDispositivo>
            id="dispositivos"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            controles={
              <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                + Nuevo dispositivo
              </button>
            }
          />
        </div>
        {cargando && filas.length === 0 && <p style={{ color: "var(--muted)" }}>Cargando…</p>}
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo dispositivo" onCerrar={cerrarModal}>
          {provisionado ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
              <p style={{ margin: 0 }}>
                Dispositivo creado en <strong>{provisionado.sitio_nombre}</strong>. Pegá este
                secreto en la app del dispositivo — no se va a volver a mostrar acá.
              </p>
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "stretch" }}>
                <input
                  readOnly
                  value={provisionado.secret}
                  onFocus={(evento) => evento.currentTarget.select()}
                  style={{ flex: 1, fontFamily: "monospace", fontSize: "0.8rem" }}
                />
                <button
                  type="button"
                  className="boton"
                  onClick={() => copiarSecreto(provisionado.secret)}
                >
                  Copiar
                </button>
              </div>
              <div style={{ display: "flex", justifyContent: "flex-end" }}>
                <button type="button" className="boton boton-primario" onClick={cerrarModal}>
                  Listo
                </button>
              </div>
            </div>
          ) : (
            <form
              onSubmit={alEnviarFormulario}
              style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
            >
              <label className="campo">
                Sitio
                <input
                  list="sitios-existentes"
                  required
                  autoFocus
                  value={sitioNombre}
                  disabled={creando}
                  placeholder="ej. Brisas"
                  onChange={(evento) => setSitioNombre(evento.target.value)}
                />
                <datalist id="sitios-existentes">
                  {sitios.map((s) => (
                    <option key={s.id} value={s.nombre} />
                  ))}
                </datalist>
              </label>

              <label className="campo">
                Dirección (opcional)
                <input
                  value={sitioDireccion}
                  disabled={creando}
                  placeholder="ej. San Rafael"
                  onChange={(evento) => setSitioDireccion(evento.target.value)}
                />
              </label>

              <label className="campo">
                Tipo de dispositivo
                <select
                  value={tipo}
                  disabled={creando}
                  onChange={(evento) => setTipo(evento.target.value as TipoDispositivo)}
                >
                  <option value="pc">PC</option>
                  <option value="mobile">Celular</option>
                  <option value="visor">Visor web (solo lectura)</option>
                </select>
              </label>

              <label className="campo">
                Etiqueta
                <input
                  required
                  value={etiqueta}
                  disabled={creando}
                  placeholder="ej. Brisas - PC recepción"
                  onChange={(evento) => setEtiqueta(evento.target.value)}
                />
              </label>

              {errorForm && (
                <p className="login-error" role="alert">
                  {errorForm}
                </p>
              )}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button type="button" className="boton" disabled={creando} onClick={cerrarModal}>
                  Cancelar
                </button>
                <button type="submit" className="boton boton-primario" disabled={creando}>
                  {creando ? "Creando…" : "Crear dispositivo"}
                </button>
              </div>
            </form>
          )}
        </Modal>
      )}

      {dispositivoAMover && (
        <Modal titulo={`Cambiar sitio — ${dispositivoAMover.etiqueta}`} onCerrar={cerrarMover}>
          <form
            onSubmit={alConfirmarMover}
            style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
          >
            <p style={{ margin: 0, color: "var(--muted)" }}>
              Está en <strong>{dispositivoAMover.sitio_nombre}</strong>. Elegí el sitio nuevo (o
              escribí uno que no exista todavía para crearlo).
            </p>

            <label className="campo">
              Sitio
              <input
                list="sitios-existentes-mover"
                required
                autoFocus
                value={sitioNombreMover}
                disabled={moviendo}
                onChange={(evento) => setSitioNombreMover(evento.target.value)}
              />
              <datalist id="sitios-existentes-mover">
                {sitios.map((s) => (
                  <option key={s.id} value={s.nombre} />
                ))}
              </datalist>
            </label>

            <label className="campo">
              Dirección (opcional, sólo si el sitio es nuevo)
              <input
                value={sitioDireccionMover}
                disabled={moviendo}
                placeholder="ej. San Rafael"
                onChange={(evento) => setSitioDireccionMover(evento.target.value)}
              />
            </label>

            {errorMover && (
              <p className="login-error" role="alert">
                {errorMover}
              </p>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
              <button type="button" className="boton" disabled={moviendo} onClick={cerrarMover}>
                Cancelar
              </button>
              <button type="submit" className="boton boton-primario" disabled={moviendo}>
                {moviendo ? "Moviendo…" : "Cambiar sitio"}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
