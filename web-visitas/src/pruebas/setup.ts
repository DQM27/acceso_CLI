/// <reference types="node" />
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";
import { iniciarReglasDesdeBytes } from "../reglas";

// Las reglas del núcleo (WebAssembly) desde sus bytes: en Node no hay fetch
// de la URL del paquete. Vitest corre desde `web-visitas/`.
iniciarReglasDesdeBytes(readFileSync(resolve("src/reglas/wasm/reglas_bg.wasm")));

afterEach(cleanup);
