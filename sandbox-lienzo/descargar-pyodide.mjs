// Baja Pyodide (Python compilado a WebAssembly) a public/pyodide/ para que el
// sandbox lo sirva desde su propio dominio: adentro del sandbox no hay acceso a
// otros sitios. No se guarda en git (son ~16 MB); correr antes de publicar:
//
//   cd sandbox-lienzo && node descargar-pyodide.mjs && npx wrangler deploy
//
// Cada archivo se compara con su huella (sha256) fijada acá: si el CDN
// devolviera otra cosa, no se guarda. Para cambiar de versión hay que revisar y
// actualizar VERSION y las huellas a mano.
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const VERSION = "314.0.7";
const NPM = `https://cdn.jsdelivr.net/npm/pyodide@${VERSION}/`;
const PAQUETES = `https://cdn.jsdelivr.net/pyodide/v${VERSION}/full/`;

const ARCHIVOS = [
  { nombre: "pyodide.mjs", base: NPM, sha256: "6f1d60f7bf529beb300f0f47983c921d3982363640ba20af0e38efdddbc66109" },
  { nombre: "pyodide.asm.mjs", base: NPM, sha256: "f7cdc8ece80678ceb712f8e65ebe6d3a83203a180c399865f49612a051693635" },
  { nombre: "pyodide.asm.wasm", base: NPM, sha256: "cc36e3cab04fdfc9a63ff13eb52eae2b911bf46c025cc7b281f394bd3de1d5e6" },
  { nombre: "python_stdlib.zip", base: NPM, sha256: "fa1957e5777068fc4f7437f96d860ae2fbe9c19732ba06c84e004ec16dd7dd7a" },
  { nombre: "pyodide-lock.json", base: NPM, sha256: "5dc2fc119108bc148c7457dc86e7675b5c87e1cafd420b9c34c1eaef7b36c010" },
  // Único paquete además de la biblioteca estándar.
  {
    nombre: "numpy-2.4.6-cp314-cp314-pyemscripten_2026_0_wasm32.whl",
    base: PAQUETES,
    sha256: "a292c1f5d7d8a2208cd5e94fc467604c131cabcd2fc14fed6eefde121e7fabdf",
  },
];

const destino = join(dirname(fileURLToPath(import.meta.url)), "public", "pyodide");
mkdirSync(destino, { recursive: true });

const huella = (contenido) => createHash("sha256").update(contenido).digest("hex");

for (const { nombre, base, sha256 } of ARCHIVOS) {
  const ruta = join(destino, nombre);
  if (existsSync(ruta) && huella(readFileSync(ruta)) === sha256) {
    console.log(`ya estaba: ${nombre}`);
    continue;
  }
  const respuesta = await fetch(base + nombre);
  if (!respuesta.ok) throw new Error(`${nombre}: HTTP ${respuesta.status}`);
  const contenido = Buffer.from(await respuesta.arrayBuffer());
  const obtenida = huella(contenido);
  if (obtenida !== sha256) throw new Error(`${nombre}: la huella no coincide (${obtenida}); no se guarda`);
  writeFileSync(ruta, contenido);
  console.log(`bajado: ${nombre} (${(contenido.length / 1e6).toFixed(1)} MB)`);
}
console.log(`Pyodide ${VERSION} listo en public/pyodide/`);
