#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoActivoMock {
    pub id: u64,
    pub cedula: String,
    pub nombre: String,
    pub empresa: String,
    pub tipo: String,
    pub hora_ingreso: String,
    pub gafete: Option<u32>,
    pub medio: String,
    pub usuario_ingreso: String,
    pub advertencia: Option<String>,
}

pub fn ingresos_activos() -> Vec<IngresoActivoMock> {
    let datos = [
        (
            "155824395105",
            "Juan Pérez",
            "Constructora Alfa",
            "PRAIND",
            "06:42",
            Some(5),
            "Caminando",
        ),
        (
            "204560789",
            "María Mora",
            "Brisas",
            "IN HOUSE",
            "06:55",
            None,
            "Vehículo",
        ),
        (
            "310220488",
            "Carlos Rojas",
            "Servicios CR",
            "POR CORREO",
            "07:03",
            Some(8),
            "Caminando",
        ),
        (
            "118822334",
            "José Jiménez",
            "Ingeniería del Pacífico",
            "PRAIND",
            "07:15",
            Some(12),
            "Vehículo",
        ),
        (
            "209944551",
            "Andrea Vargas",
            "Mantenimiento Industrial Vega",
            "PRAIND",
            "07:28",
            Some(17),
            "Caminando",
        ),
        (
            "301177882",
            "Roberto Sánchez",
            "Brisas",
            "SWAT",
            "07:36",
            None,
            "Vehículo",
        ),
        (
            "107770665",
            "Luis Fernando Rodríguez",
            "Transportes del Valle",
            "PRAIND / RUTA",
            "07:41",
            None,
            "Vehículo",
        ),
        (
            "208880774",
            "Daniela Castro",
            "Soluciones Técnicas CR",
            "PRAIND",
            "07:52",
            Some(21),
            "Caminando",
        ),
        (
            "309990443",
            "Marco Antonio Hernández",
            "Servicios Electromecánicos Nacionales",
            "PRAIND",
            "08:01",
            Some(25),
            "Vehículo",
        ),
        (
            "111223344",
            "Sofía Ramírez",
            "Proveedores Central",
            "POR CORREO",
            "08:07",
            Some(29),
            "Caminando",
        ),
        (
            "222334455",
            "Diego López",
            "Brisas",
            "IN HOUSE",
            "08:16",
            None,
            "Caminando",
        ),
        (
            "333445566",
            "Fernanda Gómez",
            "Construcciones del Este",
            "PRAIND",
            "08:24",
            Some(31),
            "Vehículo",
        ),
        (
            "444556677",
            "Alejandro Méndez",
            "Seguridad Industrial del Caribe",
            "PRAIND",
            "08:31",
            Some(34),
            "Caminando",
        ),
        (
            "555667788",
            "Natalia Solano",
            "Distribuidora Nacional",
            "POR CORREO",
            "08:47",
            Some(37),
            "Vehículo",
        ),
        (
            "666778899",
            "Esteban Quesada",
            "Brisas",
            "SWAT",
            "09:02",
            None,
            "Caminando",
        ),
        (
            "777889900",
            "Ricardo Araya",
            "Logística Costarricense",
            "PRAIND / RUTA",
            "09:18",
            None,
            "Vehículo",
        ),
        (
            "888990011",
            "Gabriela Chaves",
            "Automatización y Control Industrial",
            "PRAIND",
            "09:27",
            Some(41),
            "Caminando",
        ),
        (
            "999001122",
            "Mauricio Brenes",
            "Proyectos Integrales",
            "PRAIND",
            "09:39",
            Some(44),
            "Vehículo",
        ),
        (
            "101112233",
            "Laura Villalobos",
            "Suministros Técnicos",
            "POR CORREO",
            "09:51",
            Some(47),
            "Caminando",
        ),
        (
            "121314151",
            "Andrés Salazar",
            "Brisas",
            "IN HOUSE",
            "10:04",
            None,
            "Vehículo",
        ),
    ];

    let operadores = [
        "Quintana",
        "Ana Solís",
        "Miguel Vargas",
        "Laura Chaves",
        "Diego Mora",
        "Sofía Jiménez",
        "Carlos Brenes",
        "Daniela Rojas",
    ];

    (0..4)
        .flat_map(|ronda| {
            datos.iter().enumerate().map(move |(indice, dato)| {
                let cedula = if ronda == 0 {
                    dato.0.to_owned()
                } else {
                    format!("{}{ronda}", dato.0)
                };
                IngresoActivoMock {
                    id: (ronda * datos.len() + indice) as u64 + 1,
                    cedula,
                    nombre: dato.1.to_owned(),
                    empresa: dato.2.to_owned(),
                    tipo: dato.3.to_owned(),
                    hora_ingreso: dato.4.to_owned(),
                    gafete: dato.5.map(|gafete| gafete + ronda as u32 * 50),
                    medio: dato.6.to_owned(),
                    usuario_ingreso: operadores[(indice + ronda) % operadores.len()].to_owned(),
                    advertencia: match (indice, ronda) {
                        (3 | 8 | 16, 0 | 2) => Some("PRAIND próximo a vencer".to_owned()),
                        _ => None,
                    },
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn genera_ochenta_ingresos_con_gafetes_unicos_y_operadores_variados() {
        let ingresos = ingresos_activos();
        assert_eq!(ingresos.len(), 80);

        let gafetes: Vec<_> = ingresos
            .iter()
            .filter_map(|ingreso| ingreso.gafete)
            .collect();
        let gafetes_unicos: HashSet<_> = gafetes.iter().copied().collect();
        assert_eq!(gafetes.len(), gafetes_unicos.len());

        let operadores: HashSet<_> = ingresos
            .iter()
            .map(|ingreso| ingreso.usuario_ingreso.as_str())
            .collect();
        assert_eq!(operadores.len(), 8);
    }
}
