import { useEffect, useState } from "react";
import Login from "./pantallas/Login";
import Activos from "./pantallas/Activos";
import Contratistas from "./pantallas/Contratistas";
import Empresas from "./pantallas/Empresas";
import Usuarios from "./pantallas/Usuarios";
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

type Seccion = "activos" | "contratistas" | "empresas" | "usuarios";

const SECCIONES: { id: Seccion; etiqueta: string }[] = [
  { id: "activos", etiqueta: "Ingresos activos" },
  { id: "contratistas", etiqueta: "Contratistas" },
  { id: "empresas", etiqueta: "Empresas" },
  { id: "usuarios", etiqueta: "Usuarios" },
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

  return (
    <div style={{ display: "flex", height: "100%" }}>
      <nav className="shell-sidebar">
        <div className="shell-marca">
          <div className="marca-sello" aria-hidden="true">
            B
          </div>
          <div>
            <p style={{ margin: 0, fontSize: "0.9rem", fontWeight: 600, color: "var(--texto)" }}>
              Brisas
            </p>
            <p style={{ margin: 0, fontSize: "0.75rem", color: "var(--muted)" }}>
              Control de acceso
            </p>
          </div>
        </div>

        <div className="shell-nav">
          {SECCIONES.map((item) => (
            <button
              key={item.id}
              onClick={() => setSeccion(item.id)}
              className={`nav-item ${seccion === item.id ? "nav-item-activo" : ""}`}
            >
              {item.etiqueta}
            </button>
          ))}
        </div>

        <div style={{ flex: 1 }} />

        <div className="shell-usuario">
          <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
            <div className="shell-avatar">{inicial}</div>
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
          </div>
          <button className="boton" style={{ width: "100%" }} onClick={onCerrarSesion}>
            Cerrar sesión
          </button>
        </div>
      </nav>

      <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
        {seccion === "activos" && <Activos />}
        {seccion === "contratistas" && <Contratistas />}
        {seccion === "empresas" && <Empresas />}
        {seccion === "usuarios" && <Usuarios />}
      </main>
    </div>
  );
}
