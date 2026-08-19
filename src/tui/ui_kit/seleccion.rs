/// Mueve una selección dentro de una lista de `longitud` elementos, saturando
/// en los bordes (no da la vuelta). `None` si la lista está vacía.
///
/// Patrón compartido por toda pantalla con una lista navegable con
/// Arriba/Abajo — antes copiado letra por letra en `nuevo_ingreso` y
/// `salida_rapida`.
pub fn mover_seleccion(seleccion: Option<usize>, delta: isize, longitud: usize) -> Option<usize> {
    if longitud == 0 {
        return None;
    }
    let actual = seleccion.unwrap_or(0);
    Some(if delta < 0 {
        actual.saturating_sub(1)
    } else {
        (actual + 1).min(longitud - 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lista_vacia_no_selecciona_nada() {
        assert_eq!(mover_seleccion(None, 1, 0), None);
        assert_eq!(mover_seleccion(Some(0), -1, 0), None);
    }

    #[test]
    fn satura_en_los_bordes_sin_dar_la_vuelta() {
        assert_eq!(mover_seleccion(Some(0), -1, 3), Some(0));
        assert_eq!(mover_seleccion(Some(2), 1, 3), Some(2));
    }

    #[test]
    fn sin_seleccion_previa_arranca_en_cero() {
        assert_eq!(mover_seleccion(None, -1, 3), Some(0));
        assert_eq!(mover_seleccion(None, 1, 3), Some(1));
    }
}
