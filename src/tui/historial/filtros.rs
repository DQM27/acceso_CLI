use chrono::NaiveDate;

pub(super) const EMPRESAS: &[&str] = &[
    "Todas",
    "Brisas",
    "Constructora Alfa",
    "Servicios CR",
    "Ingeniería del Pacífico",
    "Mantenimiento Industrial Vega",
    "Transportes del Valle",
    "Soluciones Técnicas CR",
    "Servicios Electromecánicos Nacionales",
    "Proveedores Central",
    "Construcciones del Este",
];
pub(super) const TIPOS: &[&str] = &[
    "Todos",
    "PRAIND",
    "IN HOUSE",
    "POR CORREO",
    "SWAT",
    "PRAIND / RUTA",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoFiltro {
    Todos,
    Cerrados,
    Activos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltrosHistorial {
    pub desde: String,
    pub hasta: String,
    pub nombre_cedula: String,
    pub empresa: String,
    pub tipo: String,
    pub gafete: String,
    pub estado: EstadoFiltro,
}

impl Default for FiltrosHistorial {
    fn default() -> Self {
        Self {
            desde: "01/08/2026".into(),
            hasta: "12/08/2026".into(),
            nombre_cedula: String::new(),
            empresa: "Todas".into(),
            tipo: "Todos".into(),
            gafete: String::new(),
            estado: EstadoFiltro::Todos,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CampoFiltro {
    Desde,
    Hasta,
    NombreCedula,
    Empresa,
    Tipo,
    Gafete,
    Estado,
}

impl CampoFiltro {
    pub(super) const TODOS: [Self; 7] = [
        Self::Desde,
        Self::Hasta,
        Self::NombreCedula,
        Self::Empresa,
        Self::Tipo,
        Self::Gafete,
        Self::Estado,
    ];
}

pub(super) fn fecha(valor: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(valor, "%d/%m/%Y").ok()
}
pub(super) fn fechas_validas(filtros: &FiltrosHistorial) -> bool {
    matches!((fecha(&filtros.desde), fecha(&filtros.hasta)), (Some(desde), Some(hasta)) if desde <= hasta)
}

pub(super) fn estado_texto(estado: EstadoFiltro) -> &'static str {
    match estado {
        EstadoFiltro::Todos => "Todos",
        EstadoFiltro::Cerrados => "Cerrados",
        EstadoFiltro::Activos => "Activos",
    }
}

pub(super) fn opciones_campo(campo: CampoFiltro) -> &'static [&'static str] {
    match campo {
        CampoFiltro::Empresa => EMPRESAS,
        CampoFiltro::Tipo => TIPOS,
        CampoFiltro::Estado => &["Todos", "Cerrados", "Activos"],
        _ => &[],
    }
}
