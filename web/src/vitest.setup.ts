/// <reference types="node" />
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";
import { iniciarReglasDesdeBytes } from "./reglas";

// Las reglas del núcleo (WebAssembly, ver src/reglas) se cargan una vez para
// todos los tests. En el navegador las baja `iniciarReglas` con `fetch`;
// acá, que es Node, se leen del disco.
// Vitest corre desde `web/` (bajo jsdom, `import.meta.url` no es una ruta).
iniciarReglasDesdeBytes(readFileSync(resolve("src/reglas/wasm/reglas_bg.wasm")));

// Sin esto, cada `render()` deja su DOM montado para el siguiente test del
// mismo archivo — `getByTestId`/`getByText` etc. empiezan a encontrar
// elementos de tests anteriores y fallan con "found multiple elements".
afterEach(() => {
  cleanup();
});
