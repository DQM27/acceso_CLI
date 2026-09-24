import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CustomDateProps } from "ag-grid-react";
import FiltroFechaTabla from "./FiltroFechaTabla";

function props(date: Date | null, onDateChange = vi.fn()): CustomDateProps {
  return { date, onDateChange } as unknown as CustomDateProps;
}

describe("FiltroFechaTabla", () => {
  it("muestra la fecha actual del filtro en DD/MM/AAAA", () => {
    render(<FiltroFechaTabla {...props(new Date(2026, 8, 23))} />);
    expect((screen.getByPlaceholderText("DD/MM/AAAA") as HTMLInputElement).value).toBe("23/09/2026");
  });

  it("pone las barras y avisa la fecha recién cuando está completa", () => {
    const onDateChange = vi.fn();
    render(<FiltroFechaTabla {...props(null, onDateChange)} />);
    const campo = screen.getByPlaceholderText("DD/MM/AAAA") as HTMLInputElement;

    fireEvent.change(campo, { target: { value: "2309" } });
    expect(campo.value).toBe("23/09");
    expect(onDateChange).toHaveBeenLastCalledWith(null);

    fireEvent.change(campo, { target: { value: "23/09/2026" } });
    expect(onDateChange).toHaveBeenLastCalledWith(new Date(2026, 8, 23));
  });
});
