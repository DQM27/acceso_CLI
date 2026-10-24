// Motor de Python del lienzo (panel "analisis"): Pyodide en un Web Worker.
//
// `trabajarPython` se copia tal cual (toString) a un Worker armado en memoria
// (ver python.js): no puede usar nada de afuera de su propio cuerpo. Carga
// Pyodide desde este mismo sitio (public/pyodide/, bajado con
// descargar-pyodide.mjs), prepara el entorno una sola vez y queda vivo: los
// cálculos siguientes no vuelven a pagar el arranque.
//
// El código del asistente recibe `datos` (las filas del lienzo) y `columnas`, y
// deja en `resultado` una lista de filas. Lo que vuelve al panel es sólo esa
// lista, con las columnas declaradas y valores simples (texto, número, sí/no).

/** Lo que se corre una vez al arrancar: `_ejecutar` y `_paquetes`. */
export const PREPARACION = String.raw`
import ast, builtins, datetime, decimal, json, math, traceback

_MAXIMO_FILAS = 2000
_PERMITIDOS = {"numpy"}  # además de la biblioteca estándar
# Nada que salga del cálculo: el puente a JavaScript, la red, procesos o el propio intérprete.
_PROHIBIDOS = {
    "js", "pyodide", "pyodide_js", "_pyodide", "micropip", "pyodide_http", "ctypes", "socket", "ssl",
    "urllib", "http", "webbrowser", "subprocess", "multiprocessing", "threading", "asyncio",
    "importlib", "sys", "os", "builtins", "gc", "inspect", "shutil", "pathlib", "io", "pickle", "marshal",
}

def _paquetes(codigo):
    """Los paquetes que importa el código y hay que cargar antes (de los permitidos)."""
    try:
        arbol = ast.parse(codigo)
    except SyntaxError:
        return []
    nombres = set()
    for nodo in ast.walk(arbol):
        if isinstance(nodo, ast.Import):
            nombres.update(a.name.split(".")[0] for a in nodo.names)
        elif isinstance(nodo, ast.ImportFrom) and nodo.module and nodo.level == 0:
            nombres.add(nodo.module.split(".")[0])
    return sorted(nombres & _PERMITIDOS)

_importar_real = builtins.__import__

def _importar(nombre, globales=None, locales=None, desde=(), nivel=0):
    if nivel != 0 or nombre.split(".")[0] in _PROHIBIDOS:
        raise ImportError(f"en el análisis no se puede usar «{nombre}»")
    return _importar_real(nombre, globales, locales, desde, nivel)

_QUITAR = {"open", "input", "breakpoint", "exit", "quit", "help", "eval", "exec", "compile",
           "globals", "locals", "vars", "memoryview", "__import__"}
_SEGUROS = {k: v for k, v in vars(builtins).items() if k not in _QUITAR}
_SEGUROS["__import__"] = _importar

def _valor(v):
    """Un valor de una fila del resultado, como lo puede mostrar el panel."""
    if v is None or isinstance(v, bool):
        return v
    if isinstance(v, str):
        return v[:500]
    if isinstance(v, int):
        return v if abs(v) <= 2**53 else float(v)
    if isinstance(v, float):
        return v if math.isfinite(v) else None
    if isinstance(v, datetime.datetime):
        return v.date().isoformat()
    if isinstance(v, datetime.date):
        return v.isoformat()
    if isinstance(v, decimal.Decimal):
        return _valor(float(v))
    convertir = getattr(v, "item", None)  # números de numpy
    if callable(convertir):
        try:
            return _valor(convertir())
        except Exception:
            pass
    return str(v)[:500]

def _error(mensaje):
    return json.dumps({"error": mensaje[:400]}, ensure_ascii=False)

def _ejecutar(codigo, datos_json, columnas_json, salida_json):
    salida = json.loads(salida_json)
    entorno = {
        "__builtins__": _SEGUROS,
        "__name__": "analisis",
        "datos": json.loads(datos_json),
        "columnas": json.loads(columnas_json),
    }
    try:
        exec(compile(codigo, "<analisis>", "exec"), entorno)
    except SyntaxError as e:
        return _error(f"Error de sintaxis en la línea {e.lineno}: {e.msg}")
    except Exception as e:
        linea = None
        for marco in traceback.extract_tb(e.__traceback__):
            if marco.filename == "<analisis>":
                linea = marco.lineno
        donde = f" (línea {linea})" if linea else ""
        return _error(f"{type(e).__name__}{donde}: {e}")
    if "resultado" not in entorno:
        return _error("El análisis no dejó nada en «resultado» (una lista de filas).")
    resultado = entorno["resultado"]
    if isinstance(resultado, dict):
        resultado = [resultado]
    if not isinstance(resultado, (list, tuple)):
        return _error("«resultado» tiene que ser una lista de filas (diccionarios).")
    if len(resultado) > _MAXIMO_FILAS:
        return _error(f"«resultado» tiene {len(resultado)} filas; el máximo es {_MAXIMO_FILAS}.")
    if any(not isinstance(fila, dict) for fila in resultado):
        return _error("Cada fila de «resultado» tiene que ser un diccionario.")
    faltan = [c for c in salida if resultado and all(c not in fila for fila in resultado)]
    if faltan:
        return _error("A «resultado» le faltan las columnas: " + ", ".join(faltan) + ".")
    filas = [{c: _valor(fila.get(c)) for c in salida} for fila in resultado]
    return json.dumps({"filas": filas}, ensure_ascii=False)
`;

export async function trabajarPython(base, preparacion) {
  // Sin red, salvo para bajar Pyodide y numpy de este mismo sitio (y sólo mientras se bajan).
  const fetchReal = self.fetch.bind(self);
  let conRed = true;
  self.fetch = (recurso, opciones) => {
    const url = typeof recurso === "string" ? recurso : (recurso?.url ?? String(recurso));
    if (!conRed || !url.startsWith(base)) return Promise.reject(new TypeError("Sin acceso a la red"));
    return fetchReal(recurso, opciones);
  };
  for (const nombre of ["XMLHttpRequest", "WebSocket", "EventSource", "indexedDB", "caches"]) {
    try {
      Object.defineProperty(self, nombre, { value: undefined, configurable: false, writable: false });
    } catch {
      // Si no se puede redefinir, igual la bloquea la política de seguridad.
    }
  }
  const texto = (error) => String(error instanceof Error ? error.message : error).slice(0, 400);

  let py;
  try {
    const { loadPyodide } = await import(`${base}pyodide.mjs`);
    py = await loadPyodide({ indexURL: base, stdout: () => {}, stderr: () => {} });
    py.runPython(preparacion);
  } catch (error) {
    self.postMessage({ tipo: "error_carga", mensaje: texto(error) });
    return;
  } finally {
    conRed = false;
  }
  const paquetes = py.globals.get("_paquetes");
  const ejecutar = py.globals.get("_ejecutar");
  const cargados = new Set();

  self.onmessage = async (evento) => {
    const { id, codigo, datos, columnas, salida } = evento.data ?? {};
    try {
      const lista = paquetes(String(codigo));
      const pedidos = lista.toJs();
      lista.destroy();
      for (const paquete of pedidos) {
        if (cargados.has(paquete)) continue;
        conRed = true;
        try {
          await py.loadPackage(paquete, { messageCallback: () => {}, errorCallback: () => {} });
        } finally {
          conRed = false;
        }
        cargados.add(paquete);
      }
      const respuesta = JSON.parse(
        ejecutar(String(codigo), JSON.stringify(datos ?? []), JSON.stringify(columnas ?? []), JSON.stringify(salida ?? [])),
      );
      self.postMessage({ tipo: "resultado", id, ...respuesta });
    } catch (error) {
      self.postMessage({ tipo: "resultado", id, error: texto(error) });
    }
  };
  self.postMessage({ tipo: "listo" });
}

/** El código fuente del Worker (módulo): `trabajarPython` con la dirección de Pyodide y la preparación. */
export function fuenteDelTrabajadorPython(base) {
  return `${trabajarPython.toString()}\ntrabajarPython(${JSON.stringify(base)}, ${JSON.stringify(PREPARACION)});\n`;
}
