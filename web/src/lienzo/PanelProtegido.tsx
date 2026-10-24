import { Component, type ErrorInfo, type ReactNode } from "react";

/**
 * Si un panel del lienzo falla al dibujarse, sólo ese panel muestra el aviso;
 * el resto de la vista y el chat siguen andando (sin esto, el error sube hasta
 * el ErrorBoundary de la app y tumba la página entera). Cuando cambia
 * `reinicio` (otro documento, otros datos) se vuelve a intentar.
 */
export default class PanelProtegido extends Component<
  { titulo?: string; reinicio: string; children: ReactNode },
  { error: Error | null; reinicio: string }
> {
  state: { error: Error | null; reinicio: string } = { error: null, reinicio: this.props.reinicio };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  static getDerivedStateFromProps(props: { reinicio: string }, state: { error: Error | null; reinicio: string }) {
    return props.reinicio === state.reinicio ? null : { error: null, reinicio: props.reinicio };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Error al dibujar un panel del lienzo:", error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <section className="tarjeta lienzo-bloque" role="alert">
        {this.props.titulo && (
          <header className="lienzo-bloque-cabecera">
            <h3>{this.props.titulo}</h3>
          </header>
        )}
        <p className="lienzo-vacio">
          No se pudo dibujar este panel. El resto de la vista sigue funcionando: pruebe quitar un filtro o pedirlo de otra forma.
        </p>
        <p className="lienzo-nota">{this.state.error.message}</p>
      </section>
    );
  }
}
