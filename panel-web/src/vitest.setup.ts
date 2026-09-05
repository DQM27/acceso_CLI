import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Sin esto, cada `render()` deja su DOM montado para el siguiente test del
// mismo archivo — `getByTestId`/`getByText` etc. empiezan a encontrar
// elementos de tests anteriores y fallan con "found multiple elements".
afterEach(() => {
  cleanup();
});
