import { useEffect, useMemo, useRef, useState } from "react";
import type { RefObject } from "react";
import { UserPlus } from "lucide-react";
import Modal from "../componentes/Modal";
import {
  FilaListaFlotante,
  ListaFlotante,
  SinResultados,
  useListaFlotante,
  useNavegacionFlechas,
} from "../componentes/ListaFlotante";
import {
  listarEmpresasProveedorSeleccionables,
  listarHistorialIngresosProveedorSitio,
  listarTodosLosProveedoresActivos,
  registrarIngresoProveedor,
} from "../api/proveedores";
import type { EmpresaProveedor, HistorialIngresoProveedorRemoto } from "../api/proveedores";
import { coincideBusqueda, plegar, textoGafete, validarNumeroGafete } from "../busqueda";

const MAX_CONOCIDOS = 4;
const MAX_EMPRESAS = 5;

/** Un proveedor que ya ingresó alguna vez al sitio, con los datos de su
 * ingreso más reciente. */
export interface ProveedorConocido {
  cedula: string;
  nombre: string;
  empresa_nombre: string | null;
}

/** Un proveedor por cédula, con el nombre y la empresa de su ingreso más
 * reciente, ordenados del más reciente al más viejo. */
export function proveedoresConocidos(
  historial: Pick<HistorialIngresoProveedorRemoto, "cedula" | "nombre" | "empresa_nombre" | "fecha_hora_ingreso">[],
): ProveedorConocido[] {
  const ordenado = [...historial].sort((a, b) =>
    b.fecha_hora_ingreso.localeCompare(a.fecha_hora_ingreso),
  );
  const vistos = new Set<string>();
  const conocidos: ProveedorConocido[] = [];
  for (const ingreso of ordenado) {
    const cedula = ingreso.cedula.trim();
    if (!cedula || vistos.has(cedula)) continue;
    vistos.add(cedula);
    conocidos.push({ cedula, nombre: ingreso.nombre, empresa_nombre: ingreso.empresa_nombre });
  }
  return conocidos;
}

/** "Nuevo proveedor" con lo que se escribió en el buscador: si parece una
 * cédula (sólo dígitos, espacios o guiones) va a la cédula; si no, al
 * nombre. */
export function datosNuevoDesdeBusqueda(texto: string): { cedula: string; nombre: string } {
  const recortado = texto.trim();
  return /^[\d\s-]+$/.test(recortado)
    ? { cedula: recortado, nombre: "" }
    : { cedula: "", nombre: recortado.toUpperCase() };
}

type Opcion = { tipo: "conocido"; proveedor: ProveedorConocido } | { tipo: "nuevo" };

type Seleccion =
  | { tipo: "ninguna" }
  | { tipo: "conocido"; proveedor: ProveedorConocido }
  | { tipo: "nuevo" };

/**
 * Mismo flujo que `NuevoIngresoModal` y `EntregarGafeteProvisionalModal`
 * (pedido del usuario 2026-09-24): buscador arriba, ficha debajo al elegir.
 * A diferencia de contratistas, los proveedores no tienen catálogo propio
 * (registro efímero, docs/features-futuras/plan-control-proveedores.md), así
 * que el buscador usa el historial del sitio: quien ya ingresó antes se
 * elige y trae su nombre y su última empresa, y sólo falta el gafete. La
 * última fila de la lista es siempre "Nuevo proveedor", que abre la ficha
 * con cédula y nombre para escribir.
 *
 * La empresa también se elige con buscador flotante (antes un `<datalist>`
 * nativo), sobre el catálogo de empresas activas.
 */
export default function IngresoProveedorModal({
  onRegistrado,
  onCerrar,
}: {
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [conocidos, setConocidos] = useState<ProveedorConocido[]>([]);
  const [gafetesAdentro, setGafetesAdentro] = useState<Map<string, number>>(new Map());
  const [empresas, setEmpresas] = useState<EmpresaProveedor[]>([]);

  const [filtro, setFiltro] = useState("");
  const [seleccion, setSeleccion] = useState<Seleccion>({ tipo: "ninguna" });
  const [cedula, setCedula] = useState("");
  const [nombre, setNombre] = useState("");
  const [empresaTexto, setEmpresaTexto] = useState("");
  const [empresa, setEmpresa] = useState<EmpresaProveedor | null>(null);
  const [placa, setPlaca] = useState("");
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const buscadorRef = useRef<HTMLInputElement>(null);
  const cedulaRef = useRef<HTMLInputElement>(null);
  const nombreRef = useRef<HTMLInputElement>(null);
  const empresaRef = useRef<HTMLInputElement>(null);
  const gafeteRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    listarEmpresasProveedorSeleccionables()
      .then(setEmpresas)
      .catch((error) => setError(String(error)));
    // El historial y los activos sólo alimentan el buscador y sus avisos: si
    // fallan, igual se puede registrar como "Nuevo proveedor".
    listarHistorialIngresosProveedorSitio()
      .then((historial) => setConocidos(proveedoresConocidos(historial)))
      .catch(() => {});
    listarTodosLosProveedoresActivos()
      .then((activos) =>
        setGafetesAdentro(new Map(activos.map((fila) => [fila.cedula.trim(), fila.gafete_numero]))),
      )
      .catch(() => {});
  }, []);

  // ---- Buscador de proveedor ----
  const opciones = useMemo<Opcion[]>(() => {
    if (!filtro.trim()) return [];
    const encontrados = conocidos
      .filter((proveedor) => coincideBusqueda(filtro, `${proveedor.cedula} ${proveedor.nombre}`))
      .slice(0, MAX_CONOCIDOS)
      .map((proveedor): Opcion => ({ tipo: "conocido", proveedor }));
    return [...encontrados, { tipo: "nuevo" }];
  }, [conocidos, filtro]);

  const listaVisible = seleccion.tipo === "ninguna" && filtro.trim().length > 0;
  const { campoRef, posicion } = useListaFlotante(listaVisible);
  const navegacion = useNavegacionFlechas(opciones, listaVisible, elegirOpcion);

  // ---- Buscador de empresa ----
  const empresasFiltradas = useMemo(
    () => empresas.filter((e) => coincideBusqueda(empresaTexto, e.nombre)).slice(0, MAX_EMPRESAS),
    [empresas, empresaTexto],
  );
  const listaEmpresasVisible =
    seleccion.tipo !== "ninguna" && empresa === null && empresaTexto.trim().length > 0;
  const { campoRef: campoEmpresaRef, posicion: posicionEmpresas } =
    useListaFlotante(listaEmpresasVisible);
  const navegacionEmpresas = useNavegacionFlechas(
    empresasFiltradas,
    listaEmpresasVisible,
    elegirEmpresa,
  );

  function cambiarFiltro(texto: string) {
    setFiltro(texto);
    setError(null);
    if (seleccion.tipo !== "ninguna") setSeleccion({ tipo: "ninguna" });
  }

  function elegirOpcion(opcion: Opcion) {
    setError(null);
    setPlaca("");
    setGafeteTexto("");
    if (opcion.tipo === "conocido") {
      const { proveedor } = opcion;
      setCedula(proveedor.cedula);
      setNombre(proveedor.nombre);
      // Su última empresa, si sigue activa en el catálogo.
      const ultima = proveedor.empresa_nombre
        ? (empresas.find((e) => plegar(e.nombre) === plegar(proveedor.empresa_nombre ?? "")) ?? null)
        : null;
      setEmpresa(ultima);
      setEmpresaTexto(ultima?.nombre ?? "");
      setSeleccion({ tipo: "conocido", proveedor });
      enfocarDespues(ultima ? gafeteRef : empresaRef);
    } else {
      const datos = datosNuevoDesdeBusqueda(filtro);
      setCedula(datos.cedula);
      setNombre(datos.nombre);
      setEmpresa(null);
      setEmpresaTexto("");
      setSeleccion({ tipo: "nuevo" });
      enfocarDespues(datos.cedula ? nombreRef : cedulaRef);
    }
  }

  function elegirEmpresa(elegida: EmpresaProveedor) {
    setEmpresa(elegida);
    setEmpresaTexto(elegida.nombre);
    setError(null);
    enfocarDespues(gafeteRef);
  }

  function cambiarEmpresaTexto(texto: string) {
    setEmpresaTexto(texto);
    // Escribir de nuevo suelta la empresa elegida: vuelve a buscar.
    if (empresa) setEmpresa(null);
  }

  function cambiarProveedor() {
    setError(null);
    setSeleccion({ tipo: "ninguna" });
    enfocarDespues(buscadorRef);
  }

  /** El campo a enfocar recién existe después del próximo render. */
  function enfocarDespues(ref: RefObject<HTMLInputElement | null>) {
    requestAnimationFrame(() => ref.current?.focus());
  }

  async function registrar() {
    if (!cedula.trim()) return setError("La cédula es obligatoria");
    if (!nombre.trim()) return setError("El nombre es obligatorio");
    if (!empresa) return setError("Elija una empresa del catálogo");
    const gafete = validarNumeroGafete(gafeteTexto);
    if (!gafete.valido) return setError(gafete.mensaje);
    setError(null);
    setEnviando(true);
    try {
      await registrarIngresoProveedor({
        cedula: cedula.trim(),
        nombre: nombre.trim(),
        empresa_id: empresa.id,
        placa: placa.trim() || null,
        gafete_numero: gafete.numero,
      });
      onRegistrado();
    } catch (error) {
      setError(String(error));
      setEnviando(false);
    }
  }

  const gafeteAdentro = gafetesAdentro.get(cedula.trim());

  return (
    <Modal titulo="Nuevo ingreso de proveedor" onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <div ref={campoRef}>
          <label className="campo">
            Buscar proveedor
            <input
              ref={buscadorRef}
              value={filtro}
              onChange={(evento) => cambiarFiltro(evento.target.value)}
              onKeyDown={navegacion.manejarTecla}
              autoFocus
              autoComplete="off"
              placeholder="Cédula o nombre…"
            />
          </label>
        </div>

        {listaVisible && posicion && (
          <ListaFlotante posicion={posicion}>
            {opciones.map((opcion, indice) =>
              opcion.tipo === "conocido" ? (
                <FilaListaFlotante
                  key={opcion.proveedor.cedula}
                  resaltada={indice === navegacion.resaltado}
                  onClick={() => elegirOpcion(opcion)}
                  onMouseEnter={() => navegacion.setResaltado(indice)}
                >
                  <span>
                    {opcion.proveedor.nombre}{" "}
                    <span style={{ color: "var(--muted)" }}>· {opcion.proveedor.cedula}</span>
                  </span>
                  {gafetesAdentro.has(opcion.proveedor.cedula) && (
                    <span
                      className="chip"
                      style={{
                        ["--chip-color" as string]: "var(--acento)",
                        alignSelf: "center",
                        flexShrink: 0,
                        whiteSpace: "nowrap",
                      }}
                    >
                      Adentro {textoGafete(gafetesAdentro.get(opcion.proveedor.cedula) ?? 0)}
                    </span>
                  )}
                </FilaListaFlotante>
              ) : (
                <FilaListaFlotante
                  key="nuevo"
                  resaltada={indice === navegacion.resaltado}
                  onClick={() => elegirOpcion(opcion)}
                  onMouseEnter={() => navegacion.setResaltado(indice)}
                >
                  <span
                    style={{ display: "flex", alignItems: "center", gap: "0.45rem", color: "var(--acento)" }}
                  >
                    <UserPlus size={15} strokeWidth={2} aria-hidden="true" />
                    Nuevo proveedor
                  </span>
                </FilaListaFlotante>
              ),
            )}
          </ListaFlotante>
        )}

        {error && seleccion.tipo === "ninguna" && (
          <p className="login-error" role="alert">
            {error}
          </p>
        )}

        {seleccion.tipo !== "ninguna" && (
          <form
            className="ficha-desplegable"
            onSubmit={(evento) => {
              evento.preventDefault();
              registrar();
            }}
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.9rem",
              padding: "0.85rem",
              border: "1px solid var(--borde)",
              borderRadius: "var(--radio-chico)",
              background: "var(--campo-fondo)",
            }}
          >
            {seleccion.tipo === "conocido" ? (
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div>
                  <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>{nombre}</p>
                  <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
                    {cedula}
                  </p>
                </div>
                <button type="button" className="boton" style={{ fontSize: "0.8rem" }} onClick={cambiarProveedor}>
                  Cambiar
                </button>
              </div>
            ) : (
              <>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>Nuevo proveedor</p>
                  <button type="button" className="boton" style={{ fontSize: "0.8rem" }} onClick={cambiarProveedor}>
                    Cambiar
                  </button>
                </div>
                <div style={{ display: "flex", gap: "0.75rem" }}>
                  <label className="campo" style={{ flex: 1 }}>
                    Cédula
                    <input
                      ref={cedulaRef}
                      value={cedula}
                      onChange={(evento) => setCedula(evento.target.value)}
                      autoComplete="off"
                    />
                  </label>
                  <label className="campo" style={{ flex: 1.6 }}>
                    Nombre
                    <input
                      ref={nombreRef}
                      value={nombre}
                      onChange={(evento) => setNombre(evento.target.value.toUpperCase())}
                      autoComplete="off"
                    />
                  </label>
                </div>
              </>
            )}

            {gafeteAdentro !== undefined && (
              <p style={{ margin: 0, color: "var(--advertencia)", fontSize: "0.85rem" }}>
                ⚠ Ya está adentro con el gafete {textoGafete(gafeteAdentro)}
              </p>
            )}

            <div ref={campoEmpresaRef}>
              <label className="campo">
                Empresa
                <input
                  ref={empresaRef}
                  value={empresaTexto}
                  onChange={(evento) => cambiarEmpresaTexto(evento.target.value)}
                  onKeyDown={navegacionEmpresas.manejarTecla}
                  autoComplete="off"
                  placeholder="Buscar en el catálogo…"
                />
              </label>
            </div>

            {listaEmpresasVisible && posicionEmpresas && (
              <ListaFlotante posicion={posicionEmpresas}>
                {empresasFiltradas.length === 0 && <SinResultados />}
                {empresasFiltradas.map((opcion, indice) => (
                  <FilaListaFlotante
                    key={opcion.id}
                    resaltada={indice === navegacionEmpresas.resaltado}
                    onClick={() => elegirEmpresa(opcion)}
                    onMouseEnter={() => navegacionEmpresas.setResaltado(indice)}
                  >
                    <span>{opcion.nombre}</span>
                  </FilaListaFlotante>
                ))}
              </ListaFlotante>
            )}

            <div style={{ display: "flex", gap: "0.75rem" }}>
              <label className="campo" style={{ flex: 1 }}>
                Placa (opcional)
                <input
                  value={placa}
                  onChange={(evento) => setPlaca(evento.target.value.toUpperCase())}
                  autoComplete="off"
                  placeholder="Vacía = caminando"
                />
              </label>
              <label className="campo" style={{ flex: 1 }}>
                N.° de gafete
                <input
                  ref={gafeteRef}
                  value={gafeteTexto}
                  onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
                  inputMode="numeric"
                  autoComplete="off"
                  placeholder="Número de gafete"
                />
              </label>
            </div>

            {error && (
              <p className="login-error" role="alert">
                {error}
              </p>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button type="submit" className="boton boton-primario" disabled={enviando}>
                {enviando ? "Registrando…" : "Registrar ingreso"}
              </button>
            </div>
          </form>
        )}
      </div>
    </Modal>
  );
}
