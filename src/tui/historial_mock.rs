use chrono::{Duration, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoHistorialMock {
    pub id: u64,
    pub fecha: NaiveDate,
    pub cedula: String,
    pub nombre: String,
    pub empresa: String,
    pub tipo: String,
    pub entrada: String,
    pub salida: Option<String>,
    pub gafete: Option<u32>,
    pub medio: String,
    pub usuario_ingreso: String,
    pub usuario_salida: Option<String>,
}

pub fn movimientos_historial() -> Vec<MovimientoHistorialMock> {
    let personas = [
        ("155824395105", "Juan Pérez", "Constructora Alfa"),
        ("204560789", "María Mora", "Brisas"),
        ("310220488", "Carlos Rojas", "Servicios CR"),
        ("118822334", "José Jiménez", "Ingeniería del Pacífico"),
        (
            "209944551",
            "Andrea Vargas",
            "Mantenimiento Industrial Vega",
        ),
        ("301177882", "Roberto Sánchez", "Brisas"),
        (
            "107770665",
            "Luis Fernando Rodríguez",
            "Transportes del Valle",
        ),
        ("208880774", "Daniela Castro", "Soluciones Técnicas CR"),
        (
            "309990443",
            "Marco Antonio Hernández",
            "Servicios Electromecánicos Nacionales",
        ),
        ("111223344", "Sofía Ramírez", "Proveedores Central"),
        ("222334455", "Diego López", "Brisas"),
        ("333445566", "Fernanda Gómez", "Construcciones del Este"),
    ];
    let tipos = [
        "PRAIND",
        "IN HOUSE",
        "POR CORREO",
        "PRAIND",
        "SWAT",
        "PRAIND / RUTA",
    ];
    let operadores = [
        "Quintana",
        "Ana Solís",
        "Miguel Vargas",
        "Laura Chaves",
        "Diego Mora",
    ];
    let inicio = NaiveDate::from_ymd_opt(2026, 8, 1).expect("fecha mock válida");

    (0..180)
        .map(|indice| {
            let persona = personas[indice % personas.len()];
            let tipo = tipos[indice % tipos.len()];
            let fecha = inicio + Duration::days((indice / 12) as i64);
            let hora = 6 + indice % 5;
            let minuto = (indice * 7) % 60;
            let activo = indice % 11 == 0;
            let requiere_gafete = matches!(tipo, "PRAIND" | "POR CORREO");
            MovimientoHistorialMock {
                id: indice as u64 + 1,
                fecha,
                cedula: format!("{}{}", persona.0, indice / personas.len()),
                nombre: persona.1.to_owned(),
                empresa: persona.2.to_owned(),
                tipo: tipo.to_owned(),
                entrada: format!("{hora:02}:{minuto:02}"),
                salida: (!activo).then(|| format!("{:02}:{:02}", hora + 8, minuto)),
                gafete: requiere_gafete.then_some((indice % 97 + 1) as u32),
                medio: if indice % 2 == 0 {
                    "Caminando"
                } else {
                    "Vehículo"
                }
                .to_owned(),
                usuario_ingreso: operadores[indice % operadores.len()].to_owned(),
                usuario_salida: (!activo)
                    .then(|| operadores[(indice + 2) % operadores.len()].to_owned()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genera_ciento_ochenta_movimientos_deterministas_con_activos() {
        let movimientos = movimientos_historial();
        assert_eq!(movimientos.len(), 180);
        assert!(
            movimientos
                .iter()
                .any(|movimiento| movimiento.salida.is_none())
        );
        assert!(
            movimientos
                .iter()
                .any(|movimiento| movimiento.salida.is_some())
        );
        assert!(
            movimientos
                .iter()
                .filter(|movimiento| movimiento.salida.is_none())
                .all(|movimiento| movimiento.usuario_salida.is_none())
        );
    }
}
