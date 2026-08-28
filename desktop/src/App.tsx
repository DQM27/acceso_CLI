import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { Toaster } from "sonner";
import { Building2, History, LogOut, UserCheck, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import marca from "./assets/marca.png";
import Login from "./pantallas/Login";
import Activos from "./pantallas/Activos";
import Contratistas from "./pantallas/Contratistas";
import Empresas from "./pantallas/Empresas";
import Usuarios from "./pantallas/Usuarios";
import Historial from "./pantallas/Historial";
import NuevoIngresoModal from "./pantallas/NuevoIngresoModal";
import SalidaModal from "./pantallas/SalidaModal";
import Consola from "./pantallas/Consola";
import { cerrarSesion, requiereConfiguracionInicial } from "./api";
import type { UsuarioSesion } from "./api";

type Pantalla =
  | { tipo: "cargando" }
  | { tipo: "requiere-configuracion-inicial" }
  | { tipo: "login" }
  | { tipo: "shell"; sesion: UsuarioSesion };

export default function App() {
  const [pantalla, setPantalla] = useState<Pantalla>({ tipo: "cargando" });

  useEffect(() => {
    requiereConfiguracionInicial()
      .then((requiere) =>
        setPantalla(requiere ? { tipo: "requiere-configuracion-inicial" } : { tipo: "login" }),
      )
      .catch((error) => {
        // Deja intentar login igual — si el problema persiste, el propio
        // comando `login` lo va a reportar con su propio mensaje de error.
        console.error(error);
        setPantalla({ tipo: "login" });
      });
  }, []);

  if (pantalla.tipo === "cargando") {
    return null;
  }

  if (pantalla.tipo === "requiere-configuracion-inicial") {
    return (
      <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center" }}>
        <p style={{ maxWidth: "24rem", textAlign: "center", color: "var(--muted)" }}>
          Todavía no existe un usuario ROOT. Creá el usuario ROOT inicial desde la consola
          (<code>--tui-clasica</code> o <code>--comandos</code>) y volvé a abrir esta ventana.
        </p>
      </div>
    );
  }

  if (pantalla.tipo === "login") {
    return <Login onAutenticado={(sesion) => setPantalla({ tipo: "shell", sesion })} />;
  }

  return (
    <Shell
      sesion={pantalla.sesion}
      onCerrarSesion={() => {
        cerrarSesion().finally(() => setPantalla({ tipo: "login" }));
      }}
    />
  );
}

type Seccion = "activos" | "historial" | "contratistas" | "empresas" | "usuarios";

const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "activos", etiqueta: "Ingresos activos", Icono: UserCheck },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "empresas", etiqueta: "Empresas", Icono: Building2 },
  { id: "usuarios", etiqueta: "Usuarios", Icono: UserCog },
];

/**
 * Interfaz central: sidebar izquierdo con las secciones + área de contenido
 * a la derecha. Cada sección nueva (ingresos, activos, historial...) sólo
 * agrega una entrada acá y su propio componente — no toca el resto.
 */
function Shell({
  sesion,
  onCerrarSesion,
}: {
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
}) {
  const [seccion, setSeccion] = useState<Seccion>("activos");
  const inicial = sesion.nombre.trim().charAt(0).toUpperCase() || "?";
  const [colapsado, setColapsado] = useState(false);

  const [modalNuevoIngreso, setModalNuevoIngreso] = useState(false);
  const [modalSalida, setModalSalida] = useState(false);
  // Sube en cada registro/salida exitosa — Activos lo usa para refrescar su
  // grilla aunque haya salido desde otra pantalla.
  const [refrescarActivos, setRefrescarActivos] = useState(0);

  // Ctrl+Shift+N/S (no Ctrl+N/S solos — esas convenciones quedan libres
  // para un "nuevo"/"salida" más genéricos más adelante) desde cualquier
  // pantalla: ambos modales son autosuficientes (buscan y registran sin
  // depender de qué sección esté abierta), así que no tiene sentido
  // atarlos a un botón dentro de Activos únicamente. Deshabilitados por
  // defecto mientras se escribe en un campo de texto (comportamiento por
  // defecto de la librería).
  useHotkeys("ctrl+shift+n", () => setModalNuevoIngreso(true), { preventDefault: true });
  useHotkeys("ctrl+shift+s", () => setModalSalida(true), { preventDefault: true });

  return (
    <div style={{ display: "flex", height: "100%" }}>
      <nav className={`shell-sidebar ${colapsado ? "shell-sidebar-colapsada" : ""}`}>
        <div
          className="shell-marca"
          title="Doble click para colapsar/expandir"
          onDoubleClick={() => setColapsado((c) => !c)}
        >
          <div className="marca-sello">
            <img src={marca} alt="" />
          </div>
          {!colapsado && (
            <div style={{ minWidth: 0 }}>
              <p style={{ margin: 0, fontSize: "0.9rem", fontWeight: 600, color: "var(--texto)" }}>
                Brisas
              </p>
              <p style={{ margin: 0, fontSize: "0.75rem", color: "var(--muted)" }}>
                Control de acceso
              </p>
            </div>
          )}
        </div>

        <div className="shell-nav">
          {SECCIONES.map(({ id, etiqueta, Icono }) => (
            <button
              key={id}
              onClick={() => setSeccion(id)}
              title={colapsado ? etiqueta : undefined}
              className={`nav-item ${seccion === id ? "nav-item-activo" : ""}`}
            >
              <Icono size={18} strokeWidth={2} aria-hidden="true" />
              {!colapsado && etiqueta}
            </button>
          ))}
        </div>

        <div
          style={{ flex: 1 }}
          title="Doble click para colapsar/expandir"
          onDoubleClick={() => setColapsado((c) => !c)}
        />

        <div className="shell-usuario">
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.6rem",
              justifyContent: colapsado ? "center" : "flex-start",
            }}
          >
            <div className="shell-avatar" title={colapsado ? sesion.nombre : undefined}>
              {inicial}
            </div>
            {!colapsado && (
              <div style={{ minWidth: 0 }}>
                <p
                  style={{
                    margin: 0,
                    fontSize: "0.85rem",
                    fontWeight: 600,
                    color: "var(--texto)",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  {sesion.nombre}
                </p>
                <span className="chip">{sesion.rol}</span>
              </div>
            )}
          </div>
          <button
            className="boton boton-icono boton-salir"
            title={colapsado ? "Cerrar sesión" : undefined}
            onClick={onCerrarSesion}
          >
            <LogOut size={17} strokeWidth={2} aria-hidden="true" />
            {!colapsado && "Cerrar sesión"}
          </button>
        </div>
      </nav>

      <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
        {seccion === "activos" && (
          <Activos
            refrescarSenal={refrescarActivos}
            onAbrirNuevoIngreso={() => setModalNuevoIngreso(true)}
            onAbrirSalida={() => setModalSalida(true)}
          />
        )}
        {seccion === "historial" && <Historial />}
        {seccion === "contratistas" && <Contratistas />}
        {seccion === "empresas" && <Empresas />}
        {seccion === "usuarios" && <Usuarios />}
      </main>

      {modalNuevoIngreso && (
        <NuevoIngresoModal
          onRegistrado={() => setRefrescarActivos((n) => n + 1)}
          onCerrar={() => setModalNuevoIngreso(false)}
        />
      )}

      {modalSalida && (
        <SalidaModal
          onRegistrado={() => setRefrescarActivos((n) => n + 1)}
          onCerrar={() => setModalSalida(false)}
        />
      )}

      <Consola />
      {/* theme="system": mismo criterio que el resto de la app (paleta
          clara/oscura sigue `prefers-color-scheme`, sin toggle manual
          todavía) — estilizado con las variables propias en index.css, no
          los colores por defecto de sonner. */}
      <Toaster theme="system" position="bottom-right" richColors={false} />
    </div>
  );
}
