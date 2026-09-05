import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { save } from "@tauri-apps/plugin-dialog";
import Modal from "../componentes/Modal";
import {
  crearRespaldo,
  esValido,
  etiquetaTipoRespaldo,
  exportarRespaldo,
  listarRespaldos,
  restaurarRespaldo,
  textoValidacion,
  validarRespaldo,
} from "../api";
import type { RespaldoResumen, ResultadoValidacion } from "../api";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

function nombreArchivo(ruta: string): string {
  return ruta.split(/[\\/]/).pop() ?? ruta;
}

function tamanoLegible(bytes: number): string {
  const unidad = 1024;
  if (bytes < unidad) return `${bytes} B`;
  if (bytes < unidad ** 2) return `${(bytes / unidad).toFixed(1)} KB`;
  return `${(bytes / unidad ** 2).toFixed(1)} MB`;
}

function fechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

interface FilaRespaldo {
  resumen: RespaldoResumen;
  validacion: ResultadoValidacion | null;
  validando: boolean;
}

/**
 * Pantalla exclusiva de Root (`Operacion::GestionarRespaldos`, ver
 * `App.tsx`) — paridad con la TUI (`tui/configuracion/state.rs`): crear,
 * listar, validar, exportar y restaurar. Sin tabla AG Grid a propósito
 * (mismo criterio que `HistorialGafeteModal.tsx`): la lista de respaldos es
 * chica, no hace falta virtualización ni búsqueda.
 *
 * `onRestaurado` es lo mismo que `onCerrarSesion` en `App.tsx` — restaurar
 * ya cierra la sesión del lado del núcleo (la base activa cambió de
 * identidad), así que el frontend sólo necesita volver a Login, no un
 * mecanismo aparte.
 */
export default function Respaldos({ onRestaurado }: { onRestaurado: () => void }) {
  const [filas, setFilas] = useState<FilaRespaldo[]>([]);
  const [cargando, setCargando] = useState(true);
  const [creando, setCreando] = useState(false);
  const [confirmando, setConfirmando] = useState<RespaldoResumen | null>(null);
  const [restaurando, setRestaurando] = useState(false);

  function cargar() {
    setCargando(true);
    listarRespaldos()
      .then((items) =>
        setFilas(items.map((resumen) => ({ resumen, validacion: null, validando: false }))),
      )
      .catch((error) => toast.error(String(error)))
      .finally(() => setCargando(false));
  }

  useEffect(cargar, []);

  async function crear() {
    setCreando(true);
    try {
      const resumen = await crearRespaldo();
      toast.success(`Respaldo creado — ${nombreArchivo(resumen.ruta)}`);
      setFilas((actuales) => [{ resumen, validacion: null, validando: false }, ...actuales]);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setCreando(false);
    }
  }

  async function validar(ruta: string) {
    setFilas((actuales) =>
      actuales.map((fila) => (fila.resumen.ruta === ruta ? { ...fila, validando: true } : fila)),
    );
    try {
      const resultado = await validarRespaldo(ruta);
      setFilas((actuales) =>
        actuales.map((fila) =>
          fila.resumen.ruta === ruta ? { ...fila, validacion: resultado, validando: false } : fila,
        ),
      );
    } catch (error) {
      toast.error(String(error));
      setFilas((actuales) =>
        actuales.map((fila) => (fila.resumen.ruta === ruta ? { ...fila, validando: false } : fila)),
      );
    }
  }

  async function exportar(ruta: string) {
    const destino = await save({
      title: "Exportar respaldo",
      defaultPath: nombreArchivo(ruta),
      filters: [{ name: "Base de datos", extensions: ["db"] }],
    });
    if (!destino) return;
    toast.promise(exportarRespaldo(ruta, destino), {
      loading: "Exportando…",
      success: `Exportado a ${destino}`,
      error: (error) => String(error),
    });
  }

  async function confirmarRestauracion() {
    if (!confirmando) return;
    setRestaurando(true);
    try {
      await restaurarRespaldo(confirmando.ruta);
      // Éxito: el núcleo ya cerró la sesión del lado de Tauri — no hay
      // nada más que refrescar en esta pantalla, la app entera vuelve a
      // Login.
      onRestaurado();
    } catch (error) {
      // La sesión también se cerró del lado del núcleo aunque haya
      // fallado (ver `GuiState::restaurar_respaldo`) — la base pudo haber
      // quedado en un estado distinto al de antes del intento, así que un
      // login nuevo es lo más seguro de todos modos.
      toast.error(String(error));
      onRestaurado();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        className="pantalla-cuerpo"
        style={{ minHeight: 0, flex: 1, display: "flex", flexDirection: "column", gap: "0.75rem" }}
      >
        <div>
          <button type="button" className="boton boton-primario" onClick={crear} disabled={creando}>
            {creando ? "Creando…" : "Crear respaldo"}
          </button>
        </div>

        {cargando && <p style={{ color: "var(--muted)" }}>Cargando…</p>}
        {!cargando && filas.length === 0 && (
          <p style={{ color: "var(--muted)" }}>Todavía no hay respaldos.</p>
        )}
        {!cargando && filas.length > 0 && (
          <div style={{ overflow: "auto", flex: 1 }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
              <thead>
                <tr>
                  <Encabezado>Fecha</Encabezado>
                  <Encabezado>Tipo</Encabezado>
                  <Encabezado>Tamaño</Encabezado>
                  <Encabezado>Validación</Encabezado>
                  <Encabezado>Acciones</Encabezado>
                </tr>
              </thead>
              <tbody>
                {filas.map(({ resumen, validacion, validando }) => (
                  <tr key={resumen.ruta}>
                    <Celda>{fechaHora(resumen.creado_en)}</Celda>
                    <Celda>{etiquetaTipoRespaldo(resumen.tipo)}</Celda>
                    <Celda>{tamanoLegible(resumen.tamano_bytes)}</Celda>
                    <Celda>
                      {validando ? (
                        "Validando…"
                      ) : validacion ? (
                        <span
                          style={{ color: esValido(validacion) ? "var(--exito)" : "var(--error)" }}
                        >
                          {textoValidacion(validacion)}
                        </span>
                      ) : (
                        "—"
                      )}
                    </Celda>
                    <Celda>
                      <div style={{ display: "flex", gap: "0.4rem" }}>
                        <button
                          type="button"
                          className="boton"
                          onClick={() => validar(resumen.ruta)}
                          disabled={validando}
                        >
                          Validar
                        </button>
                        <button type="button" className="boton" onClick={() => exportar(resumen.ruta)}>
                          Exportar
                        </button>
                        <button
                          type="button"
                          className="boton"
                          onClick={() => setConfirmando(resumen)}
                        >
                          Restaurar
                        </button>
                      </div>
                    </Celda>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {confirmando && (
        <Modal
          titulo="Confirmar restauración"
          onCerrar={() => !restaurando && setConfirmando(null)}
        >
          <p style={{ marginTop: 0 }}>
            ¿Restaurar el respaldo del {fechaHora(confirmando.creado_en)} (
            {etiquetaTipoRespaldo(confirmando.tipo)})?
          </p>
          <p className="login-error" style={{ marginBottom: "1rem" }}>
            Esto reemplaza TODA la base de datos activa por este respaldo. Se crea automáticamente
            un respaldo de seguridad de la base actual antes de reemplazarla, pero la acción no se
            puede deshacer desde acá. La sesión se cierra al terminar — habrá que volver a iniciar
            sesión.
          </p>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              onClick={() => setConfirmando(null)}
              disabled={restaurando}
            >
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-peligro"
              onClick={confirmarRestauracion}
              disabled={restaurando}
            >
              {restaurando ? "Restaurando…" : "Restaurar"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}

function Encabezado({ children }: { children: ReactNode }) {
  return (
    <th
      style={{
        textAlign: "left",
        padding: "0.4rem 0.6rem",
        borderBottom: "1px solid var(--borde)",
        color: "var(--muted)",
        fontWeight: 500,
      }}
    >
      {children}
    </th>
  );
}

function Celda({ children }: { children: ReactNode }) {
  return (
    <td style={{ padding: "0.4rem 0.6rem", borderBottom: "1px solid var(--borde)" }}>{children}</td>
  );
}
