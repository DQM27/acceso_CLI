//! Qué partes de una sincronización hacen falta para un aviso en vivo.
//!
//! El aviso de Realtime (`cambio_nube`, emitido por
//! `private.emitir_cambio_nube_sitio()` en Postgres) ya dice qué tabla
//! cambió (`payload.table`). Antes cada aviso disparaba la sincronización
//! COMPLETA -- ~13 consultas a la nube (catálogo, ingresos, proveedores,
//! gafetes provisionales, citas, visitas, historiales, conflictos...) para
//! un solo cambio en, por ejemplo, `empresas`. Eso es lo que hacía sentir
//! lento el "tiempo real": el aviso llegaba al instante, lo que tardaba
//! era lo que se hacía al recibirlo.
//!
//! [`AlcanceSincronizacion::desde_tablas`] traduce las tablas de los
//! avisos a las etapas que de verdad hay que correr. La bandeja de salida
//! (`drenar_cola`) se vacía SIEMPRE, sea cual sea el alcance: es local y
//! barata si está vacía. El pulso periódico, el botón manual y una
//! reconexión del canal usan el alcance completo -- son la red de
//! seguridad para cualquier aviso perdido --, pero incremental: sólo lo que
//! cambió desde la última marca. La reconciliación de los historiales (7
//! días hacia atrás) queda para la primera sincronización al abrir la app
//! ([`AlcanceSincronizacion::arranque`]). Un cambio registrado en este
//! mismo equipo sólo envía ([`AlcanceSincronizacion::solo_envio`]).

/// Etapas de recepción de una sincronización. `true` = se corre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // Un interruptor por etapa es exactamente lo que modela.
pub struct AlcanceSincronizacion {
    /// Empresas, contratistas, usuarios, gafetes, empresas de proveedor y
    /// rutas -- todo lo que trae `recibir_catalogo_del_sitio`.
    pub catalogo: bool,
    /// Vehículos y encargados de ruta (`recibir_catalogo_rutas_del_sitio`).
    pub catalogo_rutas: bool,
    /// Ingresos de contratistas: cierres propios, abiertos del sitio,
    /// historial y conflictos entre sitios.
    pub ingresos: bool,
    /// Ingresos de proveedores: mismas cuatro piezas que `ingresos`.
    pub ingresos_proveedor: bool,
    /// Ingresos por correo: cierres propios, abiertos del sitio y
    /// conflictos entre sitios (todavía sin historial del sitio).
    pub ingresos_correo: bool,
    /// Préstamos de gafete provisional: abiertos, devoluciones propias e
    /// historial.
    pub gafetes_provisionales: bool,
    /// Citas del sitio.
    pub citas: bool,
    /// Movimientos de visita: historial y conflictos de visitantes.
    pub visitas: bool,
    /// Los historiales se reconcilian: se compara el índice (`id`,
    /// `updated_at`) de los últimos 7 días con la copia local y se traen
    /// sólo las filas que faltan o cambiaron -- recupera lo que un traslape
    /// corto pudiera haber dejado afuera. Sólo lo pide la primera
    /// sincronización al abrir la app ([`Self::arranque`]); el pulso y los
    /// avisos piden lo cambiado desde la marca.
    pub reconciliar_historial: bool,
}

impl AlcanceSincronizacion {
    /// Todas las etapas -- el comportamiento de siempre.
    pub const fn completo() -> Self {
        Self {
            catalogo: true,
            catalogo_rutas: true,
            ingresos: true,
            ingresos_proveedor: true,
            ingresos_correo: true,
            gafetes_provisionales: true,
            citas: true,
            visitas: true,
            reconciliar_historial: false,
        }
    }

    /// La primera sincronización al abrir la app: todas las etapas y los
    /// historiales reconciliados (ver `reconciliar_historial`).
    pub const fn arranque() -> Self {
        Self {
            reconciliar_historial: true,
            ..Self::completo()
        }
    }

    /// Ninguna etapa de recepción: sólo se vacía la bandeja de salida. Un
    /// ingreso o salida registrado en este equipo no necesita bajar nada de
    /// la nube para subirse.
    pub const fn solo_envio() -> Self {
        Self {
            catalogo: false,
            catalogo_rutas: false,
            ingresos: false,
            ingresos_proveedor: false,
            ingresos_correo: false,
            gafetes_provisionales: false,
            citas: false,
            visitas: false,
            reconciliar_historial: false,
        }
    }

    /// Sólo el catálogo del sitio: el refresco previo al login, cuando
    /// todavía no hay sesión (ver `refrescar_catalogo_sin_sesion` en cada
    /// host).
    pub const fn solo_catalogo() -> Self {
        Self {
            catalogo: true,
            catalogo_rutas: false,
            ingresos: false,
            ingresos_proveedor: false,
            ingresos_correo: false,
            gafetes_provisionales: false,
            citas: false,
            visitas: false,
            reconciliar_historial: false,
        }
    }

    /// Catálogo y catálogo de rutas: la activación inicial de un
    /// dispositivo con la base vacía.
    pub const fn catalogo_y_rutas() -> Self {
        Self {
            catalogo_rutas: true,
            ..Self::solo_catalogo()
        }
    }

    /// Alcance mínimo para los avisos de estas tablas (nombres tal cual los
    /// manda el trigger, `tg_table_name`). Cualquier tabla que no esté en
    /// el mapa -- una nueva que alguien agregue mañana, `salidas_ruta` (que
    /// hoy no se recibe por ninguna etapa), o una lista vacía -- cae en
    /// [`Self::completo`]: ante la duda se sincroniza todo, nunca se pierde
    /// un cambio.
    pub fn desde_tablas<S: AsRef<str>>(tablas: &[S]) -> Self {
        if tablas.is_empty() {
            return Self::completo();
        }
        let mut alcance = Self::default();
        for tabla in tablas {
            match tabla.as_ref() {
                "contratistas" | "empresas" | "usuarios" | "gafetes" | "empresas_proveedor"
                | "rutas" => alcance.catalogo = true,
                "vehiculos_ruta" | "encargados_ruta" => alcance.catalogo_rutas = true,
                "ingresos" => alcance.ingresos = true,
                "ingresos_proveedor" => alcance.ingresos_proveedor = true,
                "ingresos_correo" => alcance.ingresos_correo = true,
                "prestamos_gafete_provisional" => alcance.gafetes_provisionales = true,
                "cita_sitios" | "citas" => alcance.citas = true,
                // Un check-in/out de visita también puede tocar la cita
                // (ver migración `toca_cita_al_cambiar_sus_visitantes`).
                "movimientos_visita" => {
                    alcance.visitas = true;
                    alcance.citas = true;
                }
                _ => return Self::completo(),
            }
        }
        alcance
    }

    pub const fn es_completo(&self) -> bool {
        // `reconciliar_historial` no cuenta: dice CÓMO se piden los
        // historiales, no si se piden.
        self.catalogo
            && self.catalogo_rutas
            && self.ingresos
            && self.ingresos_proveedor
            && self.ingresos_correo
            && self.gafetes_provisionales
            && self.citas
            && self.visitas
    }
}

#[cfg(test)]
mod tests {
    use super::AlcanceSincronizacion;

    #[test]
    fn un_aviso_de_catalogo_solo_pide_el_catalogo() {
        let alcance = AlcanceSincronizacion::desde_tablas(&["empresas"]);
        assert_eq!(
            alcance,
            AlcanceSincronizacion {
                catalogo: true,
                ..AlcanceSincronizacion::default()
            }
        );
    }

    #[test]
    fn varios_avisos_juntan_sus_etapas() {
        let alcance = AlcanceSincronizacion::desde_tablas(&["ingresos", "gafetes", "ingresos"]);
        assert!(alcance.ingresos && alcance.catalogo);
        assert!(!alcance.citas && !alcance.visitas && !alcance.ingresos_proveedor);
    }

    #[test]
    fn una_visita_tambien_refresca_las_citas() {
        let alcance = AlcanceSincronizacion::desde_tablas(&["movimientos_visita"]);
        assert!(alcance.visitas && alcance.citas);
        assert!(!alcance.catalogo);
    }

    #[test]
    fn tabla_desconocida_o_lista_vacia_sincronizan_todo() {
        let vacia: [&str; 0] = [];
        assert!(AlcanceSincronizacion::desde_tablas(&vacia).es_completo());
        assert!(AlcanceSincronizacion::desde_tablas(&["tabla_nueva"]).es_completo());
        assert!(AlcanceSincronizacion::desde_tablas(&["salidas_ruta"]).es_completo());
        assert!(AlcanceSincronizacion::desde_tablas(&["empresas", "tabla_nueva"]).es_completo());
    }

    #[test]
    fn cada_tabla_con_trigger_de_aviso_tiene_etapa_propia() {
        // Las 15 tablas con trigger `*_emitir_cambio_nube` en Postgres
        // (menos `salidas_ruta`, que no se recibe): ninguna debe caer en
        // el alcance completo por olvido.
        for tabla in [
            "cita_sitios",
            "contratistas",
            "empresas",
            "empresas_proveedor",
            "encargados_ruta",
            "gafetes",
            "ingresos",
            "ingresos_proveedor",
            "ingresos_correo",
            "movimientos_visita",
            "prestamos_gafete_provisional",
            "rutas",
            "usuarios",
            "vehiculos_ruta",
        ] {
            assert!(
                !AlcanceSincronizacion::desde_tablas(&[tabla]).es_completo(),
                "{tabla} cayó en el alcance completo"
            );
        }
    }
}
