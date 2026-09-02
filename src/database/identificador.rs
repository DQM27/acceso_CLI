//! Identificadores estables para sincronizar con el receptor en la nube
//! (ver `docs/plan-persistencia-nube.md`).
//!
//! El `id INTEGER PRIMARY KEY` local sigue siendo la clave real de cada
//! tabla, sin tocarse -- este UUID es una identidad *adicional*, sin
//! relación con él, pensada para fuera de este dispositivo: dos bases
//! locales siempre coinciden en sus primeros `id` (1, 2, 3...), pero nunca
//! coincidirían en este UUID.

use rand_core::{OsRng, RngCore};

/// UUID v4 (aleatorio), en minúsculas con guiones.
#[must_use]
pub fn generar_uuid_v4() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);

    // Version 4 (aleatorio) y variant RFC 4122 -- mismos bits que fija
    // cualquier generador de UUID v4 estándar.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
         {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genera_uuids_distintos() {
        let a = generar_uuid_v4();
        let b = generar_uuid_v4();
        assert_ne!(a, b);
    }

    #[test]
    fn tiene_formato_uuid_v4() {
        let uuid = generar_uuid_v4();
        assert_eq!(uuid.len(), 36, "formato uuid: {uuid}");
        assert_eq!(uuid.chars().nth(14), Some('4'), "version 4: {uuid}");
        let variante = uuid.chars().nth(19).expect("caracter de variante");
        assert!(
            matches!(variante, '8' | '9' | 'a' | 'b'),
            "variante RFC 4122: {uuid}"
        );
    }
}
