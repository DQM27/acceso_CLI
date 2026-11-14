import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import VersionFooter from "./VersionFooter";

describe("VersionFooter", () => {
  it("muestra la versión real de Tauri (no la de package.json)", async () => {
    render(<VersionFooter />);
    await waitFor(() => expect(screen.getByText("v0.0.0-test")).toBeTruthy());
  });

  it("es sólo texto: buscar actualización está en el menú de la cuenta", async () => {
    render(<VersionFooter />);
    await screen.findByText("v0.0.0-test");
    expect(screen.queryByRole("button")).toBeNull();
  });
});
