//! Votación por carácter entre lecturas de varios frames.
//!
//! Exigir que la misma cadena completa se repita (lo que hacía la app
//! antes) falla con reflejos: cada frame suele errar en un carácter
//! DISTINTO y ninguna lectura coincide entera con otra. Votar posición por
//! posición combina lo que cada frame leyó bien. Es la técnica medida en
//! MIDV-500 (combinación ponderada de resultados por frame; ver
//! `docs/auditorias/investigacion-lectura-documentos-2026-09-28.md`).
//!
//! Las lecturas se agrupan por largo: una cadena con un carácter de más o
//! de menos no se alinea con las demás, y cuenta como voto en contra del
//! grupo ganador en su conjunto.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Resultado de la votación sobre las lecturas actuales.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ConsensoVotacion {
    /// El carácter más votado de cada posición.
    pub texto: String,
    /// Peso que respalda al ganador en su posición MÁS débil: cuántos
    /// "frames completos" lo sostienen, como mínimo, en todas las
    /// posiciones.
    pub soporte_minimo: f32,
    /// Ventaja del ganador sobre la segunda opción en la posición más
    /// disputada (incluye a las lecturas de otro largo como rival). Si es
    /// mayor que el peso de un frame, un frame más ya no puede cambiar el
    /// resultado.
    pub margen_minimo: f32,
    /// Lecturas que forman el grupo ganador.
    pub lecturas: u32,
}

struct Lectura {
    caracteres: Vec<char>,
    peso: f32,
}

/// Ventana deslizante de lecturas ponderadas. Los frames sin lectura
/// también ocupan lugar (ver [`VotadorPorPosicion::agregar_vacio`]) para
/// que una lectura vieja se descarte aunque no llegue otra que la
/// reemplace.
#[derive(uniffi::Object)]
pub struct VotadorPorPosicion {
    capacidad: usize,
    lecturas: Mutex<VecDeque<Option<Lectura>>>,
}

#[uniffi::export]
impl VotadorPorPosicion {
    /// `capacidad`: cuántos frames recientes (con o sin lectura) se
    /// consideran. Mínimo 1.
    #[uniffi::constructor]
    pub fn new(capacidad: u32) -> Arc<Self> {
        Arc::new(Self {
            capacidad: (capacidad as usize).max(1),
            lecturas: Mutex::new(VecDeque::new()),
        })
    }

    /// Agrega la lectura de un frame. `peso` en (0, 1]: 1 para un frame
    /// nítido; menos para uno de menor calidad. Un peso no finito o no
    /// positivo cuenta como frame sin lectura.
    pub fn agregar(&self, texto: String, peso: f32) {
        let lectura = (peso.is_finite() && peso > 0.0 && !texto.is_empty()).then(|| Lectura {
            caracteres: texto.chars().collect(),
            peso: peso.min(1.0),
        });
        self.empujar(lectura);
    }

    /// Un frame que no trajo lectura: envejece la ventana.
    pub fn agregar_vacio(&self) {
        self.empujar(None);
    }

    pub fn reiniciar(&self) {
        self.bloquear().clear();
    }

    pub fn consenso(&self) -> Option<ConsensoVotacion> {
        consenso_de(self.bloquear().iter().flatten())
    }
}

impl VotadorPorPosicion {
    fn empujar(&self, lectura: Option<Lectura>) {
        let mut lecturas = self.bloquear();
        lecturas.push_back(lectura);
        while lecturas.len() > self.capacidad {
            lecturas.pop_front();
        }
        drop(lecturas);
    }

    fn bloquear(&self) -> std::sync::MutexGuard<'_, VecDeque<Option<Lectura>>> {
        // Un pánico con el lock tomado no deja el estado a medio escribir
        // (cada operación es una sola mutación de la cola): se recupera.
        self.lecturas
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn consenso_de<'a>(lecturas: impl Iterator<Item = &'a Lectura>) -> Option<ConsensoVotacion> {
    // Por largo: lecturas del grupo e índice de su lectura más reciente.
    let mut grupos: HashMap<usize, (Vec<&Lectura>, usize)> = HashMap::new();
    for (indice, lectura) in lecturas.enumerate() {
        let entrada = grupos.entry(lectura.caracteres.len()).or_default();
        entrada.0.push(lectura);
        entrada.1 = indice;
    }
    let peso_de = |grupo: &[&Lectura]| grupo.iter().map(|l| l.peso).sum::<f32>();
    // Grupo ganador: más peso; a igualdad, el de la lectura más reciente
    // (desempate determinista, independiente del orden del HashMap).
    let (largo_ganador, grupo) = grupos
        .iter()
        .max_by(|(_, (a, reciente_a)), (_, (b, reciente_b))| {
            peso_de(a)
                .total_cmp(&peso_de(b))
                .then(reciente_a.cmp(reciente_b))
        })
        .map(|(largo, (grupo, _))| (*largo, grupo))?;
    let peso_rival_de_otro_largo = grupos
        .iter()
        .filter(|(largo, _)| **largo != largo_ganador)
        .map(|(_, (g, _))| peso_de(g))
        .fold(0.0_f32, f32::max);

    let mut texto = String::with_capacity(largo_ganador);
    let mut soporte_minimo = f32::INFINITY;
    let mut margen_minimo = peso_de(grupo) - peso_rival_de_otro_largo;
    for posicion in 0..largo_ganador {
        let mut votos: Vec<(char, f32)> = Vec::new();
        for lectura in grupo {
            let c = lectura.caracteres[posicion];
            match votos.iter_mut().find(|(v, _)| *v == c) {
                Some((_, peso)) => *peso += lectura.peso,
                None => votos.push((c, lectura.peso)),
            }
        }
        // Orden estable por peso descendente: a igualdad gana el carácter
        // que apareció primero en la ventana.
        votos.sort_by(|a, b| b.1.total_cmp(&a.1));
        let (ganador, peso_ganador) = votos[0];
        let segundo = votos.get(1).map_or(0.0, |v| v.1);
        texto.push(ganador);
        soporte_minimo = soporte_minimo.min(peso_ganador);
        margen_minimo = margen_minimo.min(peso_ganador - segundo);
    }
    if largo_ganador == 0 {
        soporte_minimo = 0.0;
    }
    Some(ConsensoVotacion {
        texto,
        soporte_minimo,
        margen_minimo,
        lecturas: u32::try_from(grupo.len()).unwrap_or(u32::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn casi(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    fn votador_con(lecturas: &[&str]) -> Arc<VotadorPorPosicion> {
        let votador = VotadorPorPosicion::new(10);
        for l in lecturas {
            votador.agregar((*l).to_string(), 1.0);
        }
        votador
    }

    #[test]
    fn sin_lecturas_no_hay_consenso() {
        assert_eq!(VotadorPorPosicion::new(4).consenso(), None);
    }

    #[test]
    fn dos_lecturas_iguales_dan_soporte_y_margen_dos() {
        let c = votador_con(&["112340567", "112340567"]).consenso().unwrap();
        assert_eq!(c.texto, "112340567");
        casi(c.soporte_minimo, 2.0);
        casi(c.margen_minimo, 2.0);
        assert_eq!(c.lecturas, 2);
    }

    #[test]
    fn errores_en_posiciones_distintas_se_corrigen_entre_frames() {
        // Ninguna lectura es correcta entera, pero cada posición lo es en
        // la mayoría: exactamente el caso que la igualdad de cadenas nunca
        // confirmaba.
        let c = votador_con(&["712340567", "118340567", "112340561"])
            .consenso()
            .unwrap();
        assert_eq!(c.texto, "112340567");
        casi(c.soporte_minimo, 2.0);
        casi(c.margen_minimo, 1.0);
    }

    #[test]
    fn empate_en_una_posicion_deja_margen_cero() {
        let c = votador_con(&["112340567", "112348567"]).consenso().unwrap();
        casi(c.margen_minimo, 0.0);
        casi(c.soporte_minimo, 1.0);
    }

    #[test]
    fn lectura_de_otro_largo_cuenta_como_rival_del_grupo_ganador() {
        let c = votador_con(&["112340567", "11234056", "112340567"])
            .consenso()
            .unwrap();
        assert_eq!(c.texto, "112340567");
        casi(c.margen_minimo, 1.0);
    }

    #[test]
    fn el_peso_de_cada_frame_pondera_el_voto() {
        let votador = VotadorPorPosicion::new(10);
        votador.agregar("A".into(), 1.0);
        votador.agregar("B".into(), 0.6);
        votador.agregar("B".into(), 0.6);
        let c = votador.consenso().unwrap();
        assert_eq!(c.texto, "B");
        casi(c.margen_minimo, 0.2);
    }

    #[test]
    fn la_ventana_descarta_lo_viejo_incluidos_los_frames_vacios() {
        let votador = VotadorPorPosicion::new(3);
        votador.agregar("111".into(), 1.0);
        votador.agregar_vacio();
        votador.agregar_vacio();
        votador.agregar("222".into(), 1.0);
        assert_eq!(votador.consenso().unwrap().texto, "222");
        assert_eq!(votador.consenso().unwrap().lecturas, 1);
    }

    #[test]
    fn peso_invalido_cuenta_como_frame_vacio() {
        let votador = VotadorPorPosicion::new(5);
        votador.agregar("111".into(), f32::NAN);
        votador.agregar("111".into(), 0.0);
        votador.agregar("111".into(), -1.0);
        assert_eq!(votador.consenso(), None);
    }

    #[test]
    fn peso_mayor_que_uno_se_limita_a_uno() {
        let votador = VotadorPorPosicion::new(5);
        votador.agregar("7".into(), 5.0);
        casi(votador.consenso().unwrap().soporte_minimo, 1.0);
    }

    #[test]
    fn empate_entre_largos_lo_gana_la_lectura_mas_reciente() {
        for _ in 0..20 {
            let c = votador_con(&["1234", "123"]).consenso().unwrap();
            assert_eq!(c.texto, "123");
            casi(c.margen_minimo, 0.0);
        }
    }

    #[test]
    fn reiniciar_vacia_la_ventana() {
        let votador = votador_con(&["123"]);
        votador.reiniciar();
        assert_eq!(votador.consenso(), None);
    }
}
