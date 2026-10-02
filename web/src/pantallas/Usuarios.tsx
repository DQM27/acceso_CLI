import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import InterruptorCelda from "../componentes/InterruptorCelda";
import AvisoTruncado from "../componentes/AvisoTruncado";
import { useLista } from "../componentes/useLista";
import {
  actualizarActivoUsuario,
  crearUsuario,
  listarUsuarios,
  resetearPasswordUsuario,
} from "../api/usuarios";
import type { Usuario } from "../api/usuarios";
import { listarDispositivosYSitios } from "../api/dispositivos";
import { usePresenciaPorSitio } from "../presenciaSitios";
import { sanearSoloDigitos, sanearSoloLetras } from "../validacion";
import { mensajeError } from "../mensajeError";

/**
 * Vista + baja + alta de operadores/administradores globales (ver
 * docs/planes-implementados/plan-panel-administrativo-web.md, punto 4). El toggle "Activo" ES
 * la baja (y la reactivación) -- global, no por sitio, ver `api/usuarios.ts`.
 * El alta genera una contraseña temporal de un solo uso (Supabase Auth,
 * ver docs/planes-implementados/plan-autenticacion-supabase-auth.md) -- se muestra una sola vez
 * acá para copiar/mandar a la persona; la app la obliga a cambiarla en su
 * primer login, en cualquier sitio. "Resetear contraseña" hace lo mismo
 * para alguien que ya existe (olvidó la suya). ROOT puede aparecer en la
 * grilla (viaja por la nube desde 2026-09-06) pero no se da de alta desde
 * acá a propósito -- eso sigue siendo CLI/TUI (`crear_root_inicial`).
 *
 * Sin unidad operativa: un usuario es global, entra en cualquier unidad.
 */
interface FilaUsuario extends Usuario {
  conectado: boolean;
  dispositivo_etiqueta: string | null;
}

export default function Usuarios() {
  const [busqueda, setBusqueda] = useState("");

  const [modalAbierto, setModalAbierto] = useState(false);
  const [cedula, setCedula] = useState("");
  const [nombre, setNombre] = useState("");
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);
  // Se muestra una sola vez, apenas vuelve del Edge Function -- ver el
  // doc-comment de arriba. `null` = no hay nada para mostrar.
  const [credencialGenerada, setCredencialGenerada] = useState<{
    cedula: string;
    password_temporal: string;
  } | null>(null);
  const [reseteando, setReseteando] = useState<string | null>(null);

  // Cambia rara vez (altas/bajas puntuales) -- mismo intervalo que usan
  // desktop/mobile para su propio sync periódico. "usuarios" para el aviso
  // en vivo (ver migración avisa_cambio_nube_en_usuarios) -- sin esto, una
  // baja/reactivación hecha desde otra sesión del panel o un dispositivo no
  // se veía acá hasta el próximo poll de 2 minutos (mismo gap que tenía
  // Contratistas.tsx antes de sumarle "contratistas,empresas").
  const { datos, cargando, recargar } = useLista(["usuarios"], listarUsuarios, {
    intervaloMs: 120_000,
    tablas: "usuarios",
  });
  const filas = datos?.filas ?? [];
  const truncado = datos?.truncado ?? false;

  // Presencia en tiempo real (docs/features-futuras/plan-sesion-unica-dispositivos.md,
  // "Panel de presencia en tiempo real"): mismo mecanismo que
  // Dispositivos.tsx, pero acá lo que importa es el `usuario_cedula` que
  // viaja en el mismo `track()` -- quién tiene sesión abierta ahora y en
  // qué dispositivo. `etiquetaPorDispositivo` sólo sirve para mostrar el
  // nombre del dispositivo en vez de su UUID.
  // Misma clave que Dispositivos.tsx: las dos pantallas comparten una sola
  // petición. Si falla, sólo degrada la presencia a "sin nombre de
  // dispositivo" -- la lista de usuarios en sí ya cargó por su cuenta.
  const { data: dispositivosYSitios } = useQuery({
    queryKey: ["dispositivos-y-sitios"],
    queryFn: listarDispositivosYSitios,
  });
  const sitios = useMemo(() => dispositivosYSitios?.sitios ?? [], [dispositivosYSitios]);
  const etiquetaPorDispositivo = useMemo(
    () => Object.fromEntries((dispositivosYSitios?.dispositivos ?? []).map((d) => [d.id, d.etiqueta])),
    [dispositivosYSitios],
  );

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
      void recargar();
    }
  }

  function abrirModal() {
    setModalAbierto(true);
    setErrorForm(null);
  }

  function cerrarModal() {
    setModalAbierto(false);
    setCedula("");
    setNombre("");
    setErrorForm(null);
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    setCreando(true);
    setErrorForm(null);
    try {
      const creado = await crearUsuario({
        cedula: cedula.trim(),
        nombre: nombre.trim(),
        // El rol ya no distingue nada dentro de la app (aplanado de
        // autorización, ver docs/decisiones-tecnicas.md 2026-09-11) -- se
        // manda fijo para no pedirle a quien crea el usuario una decisión
        // que no tiene ningún efecto real. ROOT sigue sin darse de alta
        // desde acá a propósito (ver doc-comment del componente).
        rol: "OPERADOR",
      });
      cerrarModal();
      void recargar();
      setCredencialGenerada({ cedula: creado.cedula, password_temporal: creado.password_temporal });
    } catch (error) {
      setErrorForm(mensajeError(error));
    } finally {
      setCreando(false);
    }
  }

  async function manejarResetPassword(fila: Usuario) {
    setReseteando(fila.id);
    try {
      const { password_temporal } = await resetearPasswordUsuario(fila.id);
      setCredencialGenerada({ cedula: fila.cedula, password_temporal });
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      setReseteando(null);
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
      {
        colId: "resetPassword",
        headerName: "",
        flex: 0.9,
        minWidth: 130,
        filter: false,
        sortable: false,
        cellRenderer: ({ data }: { data: FilaUsuario }) => (
          <button
            type="button"
            className="boton boton-celda-angosto"
            disabled={reseteando === data.id}
            onClick={() => manejarResetPassword(data)}
          >
            {reseteando === data.id ? "Reseteando…" : "Resetear contraseña"}
          </button>
        ),
      },
    ],
    [reseteando],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        {truncado && (
          <AvisoTruncado
            mensaje={`Hay más de ${filas.length.toLocaleString("es-CR")} usuarios -- se muestran solo los primeros (la búsqueda de aquí arriba sólo filtra entre esos, no trae más).`}
          />
        )}
        <div className="min-h-0 flex-1">
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
                <div className="campo flex-[0_1_16rem]">
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
          <form onSubmit={alEnviarFormulario} className="flex flex-col gap-3">
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

            <p className="m-0 text-[0.8rem] text-muted">
              Se genera una contraseña temporal de un solo uso -- se muestra aquí apenas se
              cree, para copiar y enviarle a la persona. La va a tener que cambiar en su
              primer inicio de sesión.
            </p>

            {errorForm && (
              <p className="login-error" role="alert">
                {errorForm}
              </p>
            )}

            <div className="flex justify-end gap-2">
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

      {credencialGenerada && (
        <Modal
          titulo="Contraseña temporal generada"
          onCerrar={() => setCredencialGenerada(null)}
        >
          <div className="flex flex-col gap-3">
            <p className="m-0 text-[0.85rem] text-muted">
              Cédula <strong>{credencialGenerada.cedula}</strong> -- copie esto y envíeselo a
              la persona (WhatsApp, en persona, lo que sea). No se vuelve a mostrar después de
              cerrar esta ventana.
            </p>
            <div className="flex items-center gap-2 rounded-[0.4rem] border border-borde px-[0.8rem] py-[0.6rem] font-mono text-[1.1rem] tracking-wider">
              <span className="flex-1">{credencialGenerada.password_temporal}</span>
              <button
                type="button"
                className="boton"
                onClick={() => {
                  navigator.clipboard.writeText(credencialGenerada.password_temporal);
                  toast.success("Copiada.");
                }}
              >
                Copiar
              </button>
            </div>
            <div className="flex justify-end">
              <button
                type="button"
                className="boton boton-primario"
                onClick={() => setCredencialGenerada(null)}
              >
                Ya la copié
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
