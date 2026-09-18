import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import VersionFooter from "./VersionFooter";

describe("VersionFooter", () => {
  it("muestra la versión real de Tauri (no la de package.json)", async () => {
    render(<VersionFooter colapsado={false} />);
    await waitFor(() => expect(screen.getByText("v0.0.0-test")).toBeTruthy());
  });

  it("colapsado oculta el texto pero conserva el title para el tooltip", async () => {
    const { container } = render(<VersionFooter colapsado />);
    await waitFor(() =>
      expect(container.querySelector(".shell-sidebar-version")?.getAttribute("title")).toBe(
        "Control de Acceso v0.0.0-test",
      ),
    );
    expect(screen.queryByText("v0.0.0-test")).toBeNull();
  });

  it("no renderiza nada mientras la versión todavía no llegó", () => {
    const { container } = render(<VersionFooter colapsado={false} />);
    expect(container.querySelector(".shell-sidebar-version")).toBeNull();
  });
});
