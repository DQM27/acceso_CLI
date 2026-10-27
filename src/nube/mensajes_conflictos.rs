//! Los avisos de conflicto de sincronización ya redactados, para que las
//! interfaces sólo los muestren (antes cada una los armaba por su cuenta:
//! `FranjaConflictos.kt`/`ConflictoGafete.kt` en el móvil y `App.tsx`/
//! `conflictoGafete.ts` en escritorio, con el riesgo de que dijeran cosas
//! distintas).
//!
//! Los de ingreso son simétricos (los dos lados "tienen razón" hasta que
//! alguien cierra uno); en los de gafete Postgres ya decidió y el movimiento
//! de ESTE equipo es el que no quedó (ver [`ConflictoGafeteActivo`]). Siguen
//! saliendo en cada sincronización mientras no se resuelvan.

use crate::nube::{
    ConflictoGafeteActivo, ConflictoIngresoActivo, ConflictoIngresoProveedorActivo,
    TipoMovimientoGafete,
};
use crate::tiempo::{a_costa_rica, parsear_utc};

/// Todos los avisos de una sincronización, en este orden: contratistas,
/// proveedores, ingresos por correo y gafetes.
pub fn mensajes_conflictos(
    ingreso: &[ConflictoIngresoActivo],
    proveedor: &[ConflictoIngresoProveedorActivo],
    correo: &[ConflictoIngresoProveedorActivo],
    gafete: &[ConflictoGafeteActivo],
) -> Vec<String> {
    let ingresos = ingreso.iter().map(|conflicto| {
        format!(
            "{} tiene un ingreso activo aquí Y en {} — hay que resolverlo.",
            conflicto.contratista_nombre, conflicto.sitio_conflicto
        )
    });
    let proveedores = proveedor.iter().map(|conflicto| {
        format!(
            "{} tiene un ingreso de proveedor activo aquí Y en {} — hay que resolverlo.",
            conflicto.nombre, conflicto.sitio_conflicto
        )
    });
    let correos = correo.iter().map(|conflicto| {
        format!(
            "{} tiene un ingreso por correo activo aquí Y en {} — hay que resolverlo.",
            conflicto.nombre, conflicto.sitio_conflicto
        )
    });
    let gafetes = gafete.iter().map(mensaje_conflicto_gafete);
    ingresos
        .chain(proveedores)
        .chain(correos)
        .chain(gafetes)
        .collect()
}

/// Un movimiento con gafete que la nube rechazó porque otro equipo de la
/// unidad ya tiene ese gafete activo. Postgres ya decidió: el de ESTE equipo
/// es el que no quedó, así que el aviso lo dice con esa certeza.
pub fn mensaje_conflicto_gafete(conflicto: &ConflictoGafeteActivo) -> String {
    let nombre = &conflicto.nombre;
    let gafete = conflicto.gafete_numero;
    let hora = hora_del_conflicto(&conflicto.fecha_hora);
    let cierre =
        "no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado.";
    match conflicto.tipo {
        TipoMovimientoGafete::Proveedor => {
            format!("El ingreso del proveedor {nombre} con gafete {gafete} ({hora}) {cierre}")
        }
        TipoMovimientoGafete::PorCorreo => format!(
            "El ingreso por correo de {nombre} con gafete de visita {gafete} ({hora}) {cierre}"
        ),
        TipoMovimientoGafete::ProvisionalKof => {
            format!("El préstamo del gafete provisional {gafete} a {nombre} ({hora}) {cierre}")
        }
        TipoMovimientoGafete::Visita => format!(
            "El check-in de visita de {nombre} con gafete de visita {gafete} ({hora}) {cierre}"
        ),
        TipoMovimientoGafete::Contratista => {
            format!("El ingreso de {nombre} con gafete {gafete} ({hora}) {cierre}")
        }
    }
}

/// "27/10/2026 06:55" en la hora de la aplicación (Costa Rica), como se ve en
/// las listas.
fn hora_del_conflicto(fecha_hora: &str) -> String {
    parsear_utc(fecha_hora).map_or_else(
        |_| "Fecha no disponible".to_string(),
        |instante| a_costa_rica(instante).format("%d/%m/%Y %H:%M").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gafete(tipo: TipoMovimientoGafete) -> ConflictoGafeteActivo {
        ConflictoGafeteActivo {
            tipo,
            nombre: "Juan Pérez".to_string(),
            gafete_numero: 12,
            // 14:00 UTC = 08:00 en Costa Rica (UTC-6, sin horario de verano).
            fecha_hora: "2026-09-30T14:00:00Z".to_string(),
        }
    }

    #[test]
    fn sin_conflictos_no_hay_avisos() {
        assert!(mensajes_conflictos(&[], &[], &[], &[]).is_empty());
    }

    #[test]
    fn un_aviso_por_conflicto_en_orden() {
        let ingreso = ConflictoIngresoActivo {
            cedula: "1".to_string(),
            contratista_nombre: "Ana Mora".to_string(),
            sitio_conflicto: "Planta Norte".to_string(),
        };
        let proveedor = ConflictoIngresoProveedorActivo {
            cedula: "2".to_string(),
            nombre: "Luis Rojas".to_string(),
            sitio_conflicto: "Bodega".to_string(),
        };
        let correo = ConflictoIngresoProveedorActivo {
            cedula: "3".to_string(),
            nombre: "Eva Solís".to_string(),
            sitio_conflicto: "Oficinas".to_string(),
        };
        let mensajes = mensajes_conflictos(
            &[ingreso],
            &[proveedor],
            &[correo],
            &[gafete(TipoMovimientoGafete::Contratista)],
        );
        assert_eq!(
            mensajes,
            vec![
                "Ana Mora tiene un ingreso activo aquí Y en Planta Norte — hay que resolverlo.",
                "Luis Rojas tiene un ingreso de proveedor activo aquí Y en Bodega — hay que resolverlo.",
                "Eva Solís tiene un ingreso por correo activo aquí Y en Oficinas — hay que resolverlo.",
                "El ingreso de Juan Pérez con gafete 12 (30/09/2026 08:00) no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado.",
            ]
        );
    }

    #[test]
    fn el_aviso_de_gafete_nombra_el_movimiento() {
        let cierre =
            "no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado.";
        let casos = [
            (
                TipoMovimientoGafete::Proveedor,
                "El ingreso del proveedor Juan Pérez con gafete 12",
            ),
            (
                TipoMovimientoGafete::PorCorreo,
                "El ingreso por correo de Juan Pérez con gafete de visita 12",
            ),
            (
                TipoMovimientoGafete::ProvisionalKof,
                "El préstamo del gafete provisional 12 a Juan Pérez",
            ),
            (
                TipoMovimientoGafete::Visita,
                "El check-in de visita de Juan Pérez con gafete de visita 12",
            ),
        ];
        for (tipo, inicio) in casos {
            assert_eq!(
                mensaje_conflicto_gafete(&gafete(tipo)),
                format!("{inicio} (30/09/2026 08:00) {cierre}")
            );
        }
    }

    #[test]
    fn una_hora_ilegible_no_rompe_el_aviso() {
        let mut conflicto = gafete(TipoMovimientoGafete::Contratista);
        conflicto.fecha_hora = "ayer".to_string();
        assert!(mensaje_conflicto_gafete(&conflicto).contains("(Fecha no disponible)"));
    }
}
