import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import InterruptorCelda from "../componentes/InterruptorCelda";
import AvisoTruncado from "../componentes/AvisoTruncado";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { actualizarActivoUsuario, crearUsuario, listarSitios, listarUsuarios } from "../api/usuarios";
import type { Usuario } from "../api/usuarios";
import { listarDispositivosYSitios } from "../api/dispositivos";
import { usePresenciaPorSitio } from "../presenciaSitios";
import { sanearSoloDigitos, sanearSoloLetras } from "../validacion";
import { mensajeError } from "../mensajeError";

/**
 * Vista + baja + alta de operadores/administradores globales (ver
 * docs/plan-panel-administrativo-web.md, punto 4). El toggle "Activo" ES
 * la baja (y la reactivación) -- global, no por sitio, ver `api/usuarios.ts`.
 * El alta no pide contraseña -- el usuario nuevo entra con el centinela
 * `SIN_PASSWORD_LOCAL`, el primer dispositivo donde esa cédula inicia
 * sesión es el que la fija de verdad (mismo mecanismo que ya existía para
 * operadores creados localmente, ver `FormularioUsuario.tsx` de desktop --
 * mismos campos cédula/nombre/rol, sin contraseña porque acá no hace
 * falta). ROOT puede aparecer en la grilla (viaja por la nube desde
 * 2026-09-06) pero no se da de alta desde acá a propósito -- eso sigue
 * siendo CLI/TUI (`crear_root_inicial`).
 *
 * Sin selector de sitio a propósito, igual que el formulario de escritorio
 * no lo tiene -- hoy existe un solo sitio ("Brisas"); se resuelve solo al
 * abrir el modal. Si algún día hay más de uno, ahí sí hace falta sumar el
 * selector (y decidir qué sitio le corresponde a cada alta).
 */
interface FilaUsuario extends Usuario {
  conectado: boolean;
  dispositivo_etiqueta: string | null;
}

export default function Usuarios() {
  const [busqueda, setBusqueda] = useState("");
  const [filas, setFilas] = useState<Usuario[]>([]);
  const [truncado, setTruncado] = useState(false);
  const [cargando, setCargando] = useState(true);

  const [modalAbierto, setModalAbierto] = useState(false);
  const [sitioId, setSitioId] = useState<string | null>(null);
  const [cedula, setCedula] = useState("");
  const [nombre, setNombre] = useState("");
  const [rol, setRol] = useState<"ADMINISTRADOR" | "OPERADOR">("OPERADOR");
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);
  // Guarda de vigencia para `abrirModal` -- ver ese comentario. Mismo
  // patrón que `vigente` en AuthContexto/useAutoRefresh, pero como
  // contador (no booleano) porque acá puede haber más de una apertura en
  // vuelo, y sólo la última importa.
  const aperturaModalRef = useRef(0);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarUsuarios()
      .then(({ filas, truncado }) => {
        setFilas(filas);
        setTruncado(truncado);
      })
      .catch((error) => {
        if (!silencioso) toast.error(mensajeError(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, []);

  useEffect(() => {
    recargar();
  }, [recargar]);

  // Cambia rara vez (altas/bajas puntuales) -- mismo intervalo que usan
  // desktop/mobile para su propio sync periódico. "usuarios" para el aviso
  // en vivo (ver migración avisa_cambio_nube_en_usuarios) -- sin esto, una
  // baja/reactivación hecha desde otra sesión del panel o un dispositivo no
  // se veía acá hasta el próximo poll de 2 minutos (mismo gap que tenía
  // Contratistas.tsx antes de sumarle "contratistas,empresas").
  useAutoRefresh(() => recargar({ silencioso: true }), 120_000, "usuarios");

  // Presencia en tiempo real (docs/plan-sesion-unica-dispositivos.md,
  // "Panel de presencia en tiempo real"): mismo mecanismo que
  // Dispositivos.tsx, pero acá lo que importa es el `usuario_cedula` que
  // viaja en el mismo `track()` -- quién tiene sesión abierta ahora y en
  // qué dispositivo. `etiquetaPorDispositivo` sólo sirve para mostrar el
  // nombre del dispositivo en vez de su UUID.
  const [sitios, setSitios] = useState<{ id: string }[]>([]);
  const [etiquetaPorDispositivo, setEtiquetaPorDispositivo] = useState<Record<string, string>>({});
  useEffect(() => {
    listarDispositivosYSitios()
      .then(({ sitios, dispositivos }) => {
        setSitios(sitios);
        setEtiquetaPorDispositivo(
          Object.fromEntries(dispositivos.map((d) => [d.id, d.etiqueta])),
        );
      })
      .catch(() => {
        // Sólo degrada la presencia a "sin nombre de dispositivo" -- la
        // lista de usuarios en sí ya cargó por su cuenta.
      });
  }, []);

  const sitioIds = useMemo(() => sitios.map((s) => s.id), [sitios]);
  const presenciaPorSitio = usePresenciaPorSitio(sitioIds);
  const conectadoPorCedula = useMemo(() => {
    const todos: Record<string, { dispositivoId?: string }> = {};
    for (const estado of Object.values(presenciaPorSitio)) {
      for (const presencias of Object.values(estado)) {
        for (const presencia of presencias as {
          usuario_cedula?: string;
          dispositivo_id?: string;
        }[]) {
          if (presencia.usuario_cedula) {
            todos[presencia.usuario_cedula] = { dispositivoId: presencia.dispositivo_id };
          }
        }
      }
    }
    return todos;
  }, [presenciaPorSitio]);

  async function manejarEdicion(fila: Usuario) {
    try {
      await actualizarActivoUsuario(fila.id, fila.activo);
      toast.success(
        fila.activo ? `${fila.nombre} reactivado.` : `${fila.nombre} dado de baja.`,
      );
    } catch (error) {
      // La grilla ya muestra el valor nuevo (edición optimista de AG Grid) --
      // si el guardado falla, hay que volver a pedir los datos reales para
      // que la celda no quede mintiendo.
      toast.error(mensajeError(error));
      recargar();
    }
  }

  function abrirModal() {
    setModalAbierto(true);
    setErrorForm(null);
    // Si el modal se cierra y se vuelve a abrir antes de que resuelva esta
    // llamada, la respuesta de la apertura VIEJA no debe pisar el
    // `sitioId` que ya eligió la apertura NUEVA -- de ahí el número de
    // apertura: sólo aplica el resultado si sigue siendo la última.
    const apertura = ++aperturaModalRef.current;
    listarSitios()
      .then((lista) => {
        if (aperturaModalRef.current !== apertura) return;
        setSitioId(lista[0]?.id ?? null);
      })
      .catch((error) => {
        if (aperturaModalRef.current !== apertura) return;
        toast.error(mensajeError(error));
      });
  }

  function cerrarModal() {
    setModalAbierto(false);
    setCedula("");
    setNombre("");
    setRol("OPERADOR");
    setErrorForm(null);
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    if (!sitioId) {
      setErrorForm("No hay ninguna unidad operativa configurada todavía.");
      return;
    }
    setCreando(true);
    setErrorForm(null);
    try {
      await crearUsuario({ sitio_id: sitioId, cedula: cedula.trim(), nombre: nombre.trim(), rol });
      toast.success(`${nombre.trim()} creado -- fija su contraseña al iniciar sesión por primera vez.`);
      cerrarModal();
      recargar();
    } catch (error) {
      setErrorForm(mensajeError(error));
    } finally {
      setCreando(false);
    }
  }

  const filasConPresencia: FilaUsuario[] = useMemo(
    () =>
      filas.map((fila) => {
        const presencia = conectadoPorCedula[fila.cedula];
        return {
          ...fila,
          conectado: presencia !== undefined,
          dispositivo_etiqueta:
            presencia?.dispositivoId != null
              ? (etiquetaPorDispositivo[presencia.dispositivoId] ?? "—")
              : null,
        };
      }),
    [filas, conectadoPorCedula, etiquetaPorDispositivo],
  );

  const columnas: ColDef<FilaUsuario>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 1, minWidth: 130, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      { field: "rol", headerName: "Rol", flex: 1, minWidth: 130 },
      {
        field: "conectado",
        headerName: "Conexión",
        flex: 0.9,
        minWidth: 130,
        filter: false,
        cellRenderer: ({ data }: { data: FilaUsuario }) => {
          const [texto, color] = data.conectado
            ? ["Conectado", "var(--exito)"]
            : ["Desconectado", "var(--muted)"];
          return (
            <span className="chip" style={{ ["--chip-color" as string]: color }}>
              {texto}
            </span>
          );
        },
      },
      {
        field: "dispositivo_etiqueta",
        headerName: "Desde",
        flex: 1.2,
        minWidth: 150,
        filter: false,
        valueFormatter: ({ value }) => value ?? "—",
      },
      {
        field: "activo",
        headerName: "Activo",
        flex: 0.9,
        minWidth: 100,
        cellRenderer: InterruptorCelda,
        cellRendererParams: { critico: true },
        filter: false,
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        {truncado && (
          <AvisoTruncado
            mensaje={`Hay más de ${filas.length.toLocaleString("es-CR")} usuarios -- se muestran solo los primeros (la búsqueda de acá arriba sólo filtra entre esos, no trae más).`}
          />
        )}
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaUsuario>
            id="usuarios"
            columnas={columnas}
            filas={filasConPresencia}
            busqueda={busqueda}
            filtrosPorColumna
            onCeldaEditada={manejarEdicion}
            controles={
              <>
                <button type="button" className="boton" onClick={abrirModal}>
                  + Nuevo
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula o nombre…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo usuario" onCerrar={cerrarModal}>
          <form onSubmit={alEnviarFormulario} style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            <label className="campo">
              Cédula
              <input
                required
                autoFocus
                inputMode="numeric"
                value={cedula}
                disabled={creando}
                onChange={(evento) => setCedula(sanearSoloDigitos(evento.target.value))}
              />
            </label>

            <label className="campo">
              Nombre
              <input
                required
                value={nombre}
                disabled={creando}
                onChange={(evento) => setNombre(sanearSoloLetras(evento.target.value))}
              />
            </label>

            <label className="campo">
              Rol
              <select
                value={rol}
                disabled={creando}
                onChange={(evento) => setRol(evento.target.value as "ADMINISTRADOR" | "OPERADOR")}
              >
                <option value="OPERADOR">Operador</option>
                <option value="ADMINISTRADOR">Administrador</option>
              </select>
            </label>

            <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.8rem" }}>
              Sin contraseña -- la persona la fija sola al iniciar sesión por primera vez en
              cualquier dispositivo.
            </p>

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
                {creando ? "Creando…" : "Crear usuario"}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
