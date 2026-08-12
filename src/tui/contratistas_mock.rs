#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoIngresoMock {
    Praind,
    InHouse,
    PorCorreo,
    Swat,
}

impl TipoIngresoMock {
    pub const TODOS: [Self; 4] = [Self::Praind, Self::InHouse, Self::PorCorreo, Self::Swat];

    pub fn texto(self) -> &'static str {
        match self {
            Self::Praind => "PRAIND",
            Self::InHouse => "IN HOUSE",
            Self::PorCorreo => "POR CORREO",
            Self::Swat => "SWAT",
        }
    }

    pub fn requiere_praind(self, personal_ruta: bool) -> bool {
        personal_ruta || matches!(self, Self::Praind | Self::InHouse)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContratistaMock {
    pub id: u64,
    pub cedula: String,
    pub nombre: String,
    pub empresa: String,
    pub tipo_ingreso: TipoIngresoMock,
    pub fecha_praind: Option<String>,
    pub personal_ruta: bool,
    pub tiene_acceso: bool,
}

pub const EMPRESAS: &[&str] = &[
    "Brisas",
    "Constructora Alfa",
    "Servicios CR",
    "Ingeniería del Pacífico",
    "Mantenimiento Industrial Vega",
    "Transportes del Valle",
    "Soluciones Técnicas CR",
    "Logística Costarricense",
];

pub fn contratistas() -> Vec<ContratistaMock> {
    let nombres = [
        "Juan Pérez",
        "María Mora",
        "Carlos Rojas",
        "Ana Vargas",
        "José Jiménez",
        "Andrea Solano",
        "Roberto Sánchez",
        "Daniela Castro",
        "Luis Rodríguez",
        "Sofía Ramírez",
    ];
    let fechas = [
        "01/08/2026",
        "12/08/2026",
        "20/08/2026",
        "15/09/2026",
        "31/12/2026",
    ];

    (0..40)
        .map(|indice| {
            let tipo_ingreso = TipoIngresoMock::TODOS[indice % TipoIngresoMock::TODOS.len()];
            let personal_ruta = indice % 9 == 6;
            let requiere_praind = tipo_ingreso.requiere_praind(personal_ruta);
            ContratistaMock {
                id: indice as u64 + 1,
                cedula: format!(
                    "{}{:02}",
                    1_558_243_951_u64 + indice as u64 * 91_337,
                    indice
                ),
                nombre: format!("{} {}", nombres[indice % nombres.len()], indice + 1),
                empresa: EMPRESAS[indice % EMPRESAS.len()].to_owned(),
                tipo_ingreso,
                fecha_praind: requiere_praind.then(|| fechas[indice % fechas.len()].to_owned()),
                personal_ruta,
                tiene_acceso: indice % 7 != 3,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genera_cuarenta_con_variedad_determinista() {
        let datos = contratistas();
        assert_eq!(datos.len(), 40);
        assert!(datos.iter().any(|c| c.personal_ruta));
        assert!(datos.iter().any(|c| !c.tiene_acceso));
        assert!(
            TipoIngresoMock::TODOS
                .iter()
                .all(|tipo| datos.iter().any(|c| c.tipo_ingreso == *tipo))
        );
    }
}
