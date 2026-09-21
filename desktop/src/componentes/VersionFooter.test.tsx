import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import VersionFooter from "./VersionFooter";

describe("VersionFooter", () => {
  it("muestra la versión real de Tauri (no la de package.json)", async () => {
    render(<VersionFooter />);
    await waitFor(() => expect(screen.getByText("v0.0.0-test")).toBeTruthy());
  });

  it("conserva el title con el nombre completo para el tooltip", async () => {
    const { container } = render(<VersionFooter />);
    await waitFor(() =>
      expect(container.querySelector(".shell-sidebar-version")?.getAttribute("title")).toBe(
        "Lattis v0.0.0-test",
      ),
    );
  });

  it("no renderiza nada mientras la versión todavía no llegó", () => {
    const { container } = render(<VersionFooter />);
    expect(container.querySelector(".shell-sidebar-version")).toBeNull();
  });
});
