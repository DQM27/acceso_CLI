import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Sin esto, cada `render()` deja su DOM montado para el siguiente test del
// mismo archivo — `getByTestId`/`getByText` etc. empiezan a encontrar
// elementos de tests anteriores y fallan con "found multiple elements".
afterEach(() => {
  cleanup();
});

// jsdom no tiene el puente de IPC de Tauri (`__TAURI_INTERNALS__`) -- sin
// este mock, cualquier componente que llame a `getVersion()` (ver
// VersionFooter.tsx) dispara una promesa rechazada en cada test que lo
// renderiza, aunque el propio componente la atrape en silencio. Valor fijo
// y reconocible para que un test que sí verifique el texto no lo confunda
// con una versión real.
vi.mock("@tauri-apps/api/app", () => ({
  getVersion: () => Promise.resolve("0.0.0-test"),
}));
