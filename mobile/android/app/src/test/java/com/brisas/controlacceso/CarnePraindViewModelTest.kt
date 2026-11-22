package com.brisas.controlacceso

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import uniffi.control_acceso_mobile.CargaCarnePraind
import uniffi.control_acceso_mobile.CarnePraind
import uniffi.control_acceso_mobile.EstadoCarnePraind
import uniffi.control_acceso_mobile.GafeteGuardado
import uniffi.control_acceso_mobile.ModoGuardadoGafete
import uniffi.control_acceso_mobile.NucleoException
import uniffi.control_acceso_mobile.EmpresaParaFicha
import uniffi.control_acceso_mobile.LineaRevisionGafete
import uniffi.control_acceso_mobile.NivelRevisionGafete
import uniffi.control_acceso_mobile.PermisosCarnePraind
import uniffi.control_acceso_mobile.PreviaGafetePraind
import uniffi.control_acceso_mobile.RevisionGafetePraind

/// Guardar el carné, bajarlo sólo si hace falta y el aviso de "conéctese a
/// internet" se prueban en Rust (`control_acceso::nube::praind`). Acá sólo
/// cómo el ViewModel mueve el estado.
@OptIn(ExperimentalCoroutinesApi::class)
class CarnePraindViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun preparar() = Dispatchers.setMain(dispatcher)

    @After
    fun limpiar() = Dispatchers.resetMain()

    private class FuenteFalsa(
        val conCarne: MutableSet<Long> = mutableSetOf(1L),
        val error: String? = null,
        val puedeCargar: Boolean = true,
        val sinConexion: Boolean = false,
        val nombreEnOrigen: String? = "BETO SOLIS",
        val coincide: Boolean = true,
        val errorCarga: String? = null,
        val revision: RevisionGafetePraind? = null,
        val errorFecha: String? = null,
        val nombreFicha: String = "BETO SOLIS",
    ) : FuenteCarnes {
        var consultasPermisos = 0
        val guardados = mutableListOf<Pair<Long, String>>()
        val textosRevisados = mutableListOf<String>()
        val correcciones = mutableListOf<Correccion>()

        override fun previsualizarConImagen(contratistaId: Long, enlace: String): PreviaGafetePraind {
            errorCarga?.let { throw NucleoException.Rechazado(it) }
            return PreviaGafetePraind(nombreEnOrigen = nombreEnOrigen, nombreCoincide = coincide, imagen = byteArrayOf(9))
        }

        override fun revisar(contratistaId: Long, textoOcr: String, nombreEnOrigen: String?): RevisionGafetePraind {
            textosRevisados += textoOcr
            return revision ?: throw NucleoException.Rechazado("sin revisión")
        }

        /// Imita a `guardar_gafete_praind_revisado` del núcleo: guarda el
        /// gafete y corrige con lo que diga la revisión y las casillas. Los
        /// textos imitan a `mensajes::mensaje_gafete_guardado`.
        override fun guardarRevisado(
            contratistaId: Long,
            enlace: String,
            textoOcr: String,
            nombreEnOrigen: String?,
            corregirNombre: Boolean,
            corregirEmpresa: Boolean,
        ): GafeteGuardado {
            errorCarga?.let { throw NucleoException.Rechazado(it) }
            if (revision?.bloqueaGuardar == true) throw NucleoException.Rechazado("no es de la ficha")
            guardados += contratistaId to enlace
            conCarne += contratistaId
            val fecha = revision?.fechaParaFicha
            val nombre = revision?.nombreParaFicha?.takeIf { corregirNombre }
            val empresa = revision?.empresaParaFicha?.id?.takeIf { corregirEmpresa }
            val cambia = fecha != null || nombre != null || empresa != null
            if (cambia) {
                errorFecha?.let {
                    return GafeteGuardado(
                        "El gafete PRAIND se guardó, pero no se pudo corregir la ficha: $it",
                        avisoEsError = true,
                        fichaCambio = false,
                    )
                }
                correcciones += Correccion(contratistaId, fecha, nombre, empresa)
            }
            val aviso = buildString {
                append("Gafete PRAIND de ${nombre ?: nombreFicha} guardado")
                if (fecha != null) append("; la ficha vence el $fecha")
                if (nombre != null || empresa != null) append("; ficha corregida")
                append('.')
            }
            return GafeteGuardado(aviso, avisoEsError = false, fichaCambio = cambia)
        }

        override fun estado(contratistaId: Long) =
            EstadoCarnePraind(tieneCarne = contratistaId in conCarne, guardado = false)

        override fun ver(contratistaId: Long): CarnePraind {
            error?.let { throw NucleoException.Rechazado(it) }
            return CarnePraind(tipoMime = "image/png", bytes = byteArrayOf(1, 2, 3))
        }

        override fun permisos(): PermisosCarnePraind {
            consultasPermisos++
            return PermisosCarnePraind(puedeCargar = puedeCargar && !sinConexion, sinConexion = sinConexion)
        }

        override fun cargarDesdeQr(contratistaId: Long, enlace: String, soloPrevisualizar: Boolean): CargaCarnePraind {
            errorCarga?.let { throw NucleoException.Rechazado(it) }
            if (!soloPrevisualizar) {
                guardados += contratistaId to enlace
                conCarne += contratistaId
            }
            return CargaCarnePraind(
                clave = if (soloPrevisualizar) null else "clave",
                nombreEnOrigen = nombreEnOrigen,
                nombreCoincide = coincide,
            )
        }
    }

    private val enlace = "https://prd.pw/abc"

    /// Sin OCR (el lector no lee nada): queda la comparación del nombre de la
    /// página, como antes de la revisión.
    private fun nuevoVm(fuente: FuenteFalsa, lector: LectorTextoImagen = LectorTextoImagen { null }) =
        CarnePraindViewModel(fuente, dispatcher, lector)

    private data class Correccion(val id: Long, val fecha: String?, val nombre: String?, val empresaId: Long?)

    private fun revision(
        vararg lineas: Pair<NivelRevisionGafete, String>,
        bloquea: Boolean = false,
        fecha: String? = null,
        nombre: String? = null,
        empresa: EmpresaParaFicha? = null,
    ) = RevisionGafetePraind(
        lineas = lineas.map { (nivel, texto) -> LineaRevisionGafete(nivel, texto) },
        bloqueaGuardar = bloquea,
        // En el núcleo sale de la revisión; acá, de las líneas de la prueba.
        modoGuardado = when {
            bloquea -> ModoGuardadoGafete.NO_SE_PUEDE
            lineas.any { it.first == NivelRevisionGafete.ERROR } -> ModoGuardadoGafete.SOLO_IMAGEN
            lineas.any { it.first == NivelRevisionGafete.AVISO } -> ModoGuardadoGafete.CON_AVISOS
            else -> ModoGuardadoGafete.NORMAL
        },
        fechaParaFicha = fecha,
        nombreParaFicha = nombre,
        empresaParaFicha = empresa,
    )

    @Test
    fun laFichaSeCorrigeConElNombreYLaEmpresaSoloSiSeMarca() = runTest(dispatcher) {
        val propuesta = revision(
            NivelRevisionGafete.AVISO to "Nombre distinto",
            nombre = "HAZEL JOHANNA ANGULO AGUILAR",
            empresa = EmpresaParaFicha(7L, "SODEXO"),
        )
        // Sin marcar: el gafete se guarda y la ficha no cambia.
        val sinMarcar = FuenteFalsa(revision = propuesta)
        val vm1 = nuevoVm(sinMarcar, leeTexto)
        vm1.previsualizar(2L, "HAZEL JOHANA ANGULO AGUILAR", enlace)
        advanceUntilIdle()
        vm1.guardar()
        advanceUntilIdle()
        assertTrue(sinMarcar.correcciones.isEmpty())

        // Marcando las dos: una sola corrección con el nombre y la empresa.
        val marcando = FuenteFalsa(revision = propuesta)
        val vm2 = nuevoVm(marcando, leeTexto)
        vm2.previsualizar(2L, "HAZEL JOHANA ANGULO AGUILAR", enlace)
        advanceUntilIdle()
        vm2.alternarCorregirNombre()
        vm2.alternarCorregirEmpresa()
        assertTrue((vm2.carga.value as CargaCarne.Previa).corregirNombre)
        vm2.guardar()
        advanceUntilIdle()
        assertEquals(
            listOf(Correccion(2L, null, "HAZEL JOHANNA ANGULO AGUILAR", 7L)),
            marcando.correcciones,
        )
        assertEquals(
            "Gafete PRAIND de HAZEL JOHANNA ANGULO AGUILAR guardado; ficha corregida.",
            vm2.guardado.value,
        )
    }

    @Test
    fun sinPropuestaNoSePuedeMarcar() = runTest(dispatcher) {
        val vm = nuevoVm(FuenteFalsa(revision = revision(NivelRevisionGafete.BIEN to "ok")), leeTexto)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        vm.alternarCorregirNombre()
        vm.alternarCorregirEmpresa()
        val previa = vm.carga.value as CargaCarne.Previa
        assertFalse(previa.corregirNombre)
        assertFalse(previa.corregirEmpresa)
    }

    private val leeTexto = LectorTextoImagen { "texto del gafete" }

    @Test
    fun laRevisionDelOcrSeMuestraYUnGafeteMasNuevoActualizaLaFicha() = runTest(dispatcher) {
        val revisada = revision(
            NivelRevisionGafete.BIEN to "Cédula: coincide.",
            NivelRevisionGafete.BIEN to "Gafete más nuevo: vence el 06/06/2028.",
            fecha = "2028-06-06",
        )
        val fuente = FuenteFalsa(revision = revisada)
        val vm = nuevoVm(fuente, leeTexto)
        vm.consultar(2L)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        val previa = vm.carga.value as CargaCarne.Previa
        assertEquals(revisada, previa.revision)
        assertEquals(listOf("texto del gafete"), fuente.textosRevisados)
        assertEquals("Guardar", textoBotonGuardar(previa))

        vm.guardar()
        advanceUntilIdle()
        assertEquals(listOf(2L to enlace), fuente.guardados)
        assertEquals(listOf(Correccion(2L, "2028-06-06", null, null)), fuente.correcciones)
        assertEquals("Gafete PRAIND de BETO SOLIS guardado; la ficha vence el 2028-06-06.", vm.guardado.value)
    }

    @Test
    fun conLaCedulaDistintaNoSeGuarda() = runTest(dispatcher) {
        val fuente = FuenteFalsa(
            revision = revision(NivelRevisionGafete.ERROR to "Cédula distinta…", bloquea = true),
        )
        val vm = nuevoVm(fuente, leeTexto)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        val previa = vm.carga.value as CargaCarne.Previa
        assertFalse(previa.sePuedeGuardar)
        assertEquals("No se puede guardar", textoBotonGuardar(previa))
        vm.guardar()
        advanceUntilIdle()
        assertTrue(fuente.guardados.isEmpty())
    }

    @Test
    fun unGafeteVencidoSeGuardaComoImagenSinTocarLaFicha() = runTest(dispatcher) {
        val fuente = FuenteFalsa(
            revision = revision(
                NivelRevisionGafete.BIEN to "Cédula: coincide.",
                NivelRevisionGafete.ERROR to "Gafete PRAIND vencido el 19/06/2026…",
            ),
        )
        val vm = nuevoVm(fuente, leeTexto)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        assertEquals("Guardar como imagen", textoBotonGuardar(vm.carga.value as CargaCarne.Previa))
        vm.guardar()
        advanceUntilIdle()
        assertEquals(listOf(2L to enlace), fuente.guardados)
        assertTrue(fuente.correcciones.isEmpty())
        assertEquals("Gafete PRAIND de BETO SOLIS guardado.", vm.guardado.value)
    }

    @Test
    fun siFallaLaFechaDiceQueElGafeteSiSeGuardo() = runTest(dispatcher) {
        val fuente = FuenteFalsa(
            revision = revision(NivelRevisionGafete.BIEN to "Más nuevo", fecha = "2028-06-06"),
            errorFecha = "El PRAIND está vencido",
        )
        val vm = nuevoVm(fuente, leeTexto)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        vm.guardar()
        advanceUntilIdle()
        assertEquals(listOf(2L to enlace), fuente.guardados)
        assertEquals(
            CargaCarne.Error(
                "BETO SOLIS",
                "El gafete PRAIND se guardó, pero no se pudo corregir la ficha: El PRAIND está vencido",
            ),
            vm.carga.value,
        )
    }

    @Test
    fun siElOcrNoLeeNadaQuedaLaComparacionDelNombre() = runTest(dispatcher) {
        val fuente = FuenteFalsa(revision = revision(NivelRevisionGafete.BIEN to "no se usa"))
        val vm = nuevoVm(fuente)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        val previa = vm.carga.value as CargaCarne.Previa
        assertEquals(null, previa.revision)
        assertTrue(fuente.textosRevisados.isEmpty())
        assertEquals("Guardar", textoBotonGuardar(previa))
    }

    @Test
    fun ofreceCargarSoloSiLaNubeLoPermiteYPreguntaUnaVez() = runTest(dispatcher) {
        val fuente = FuenteFalsa(conCarne = mutableSetOf())
        val vm = nuevoVm(fuente)
        vm.consultar(2L)
        advanceUntilIdle()
        assertTrue(vm.puedeCargar.value)
        vm.consultar(3L)
        advanceUntilIdle()
        assertEquals(1, fuente.consultasPermisos)

        val sinPermiso = nuevoVm(FuenteFalsa(puedeCargar = false))
        sinPermiso.consultar(2L)
        advanceUntilIdle()
        assertFalse(sinPermiso.puedeCargar.value)
    }

    @Test
    fun sinConexionVuelveAPreguntarLosPermisos() = runTest(dispatcher) {
        val fuente = FuenteFalsa(sinConexion = true)
        val vm = nuevoVm(fuente)
        vm.consultar(2L)
        advanceUntilIdle()
        vm.consultar(3L)
        advanceUntilIdle()
        assertFalse(vm.puedeCargar.value)
        assertEquals(2, fuente.consultasPermisos)
    }

    @Test
    fun previsualizarNoGuardaYGuardarSi() = runTest(dispatcher) {
        val fuente = FuenteFalsa(conCarne = mutableSetOf())
        val vm = nuevoVm(fuente)
        vm.consultar(2L)
        advanceUntilIdle()
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        assertEquals(CargaCarne.Buscando("BETO SOLIS"), vm.carga.value)
        advanceUntilIdle()
        assertEquals(CargaCarne.Previa(2L, "BETO SOLIS", enlace, "BETO SOLIS", true), vm.carga.value)
        assertTrue(fuente.guardados.isEmpty())

        vm.guardar()
        advanceUntilIdle()
        assertEquals(listOf(2L to enlace), fuente.guardados)
        assertEquals(CargaCarne.Inactiva, vm.carga.value)
        // La hoja pasa a ofrecer "Ver gafete PRAIND" y avisa que se guardó.
        assertTrue(vm.disponible.value)
        assertEquals("Gafete PRAIND de BETO SOLIS guardado.", vm.guardado.value)
        vm.avisoGuardadoVisto()
        assertEquals(null, vm.guardado.value)
    }

    @Test
    fun siNoCoincideLoDiceYCancelarNoGuarda() = runTest(dispatcher) {
        val fuente = FuenteFalsa(nombreEnOrigen = "OTRA PERSONA", coincide = false)
        val vm = nuevoVm(fuente)
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        val previa = vm.carga.value as CargaCarne.Previa
        assertFalse(previa.coincide)
        assertEquals(
            "El gafete PRAIND es de «OTRA PERSONA», que no coincide con BETO SOLIS. Revise que sea el gafete correcto.",
            textoPrevia(previa),
        )
        vm.cancelarCarga()
        assertEquals(CargaCarne.Inactiva, vm.carga.value)
        assertTrue(fuente.guardados.isEmpty())
    }

    @Test
    fun unRechazoDeLaNubeSeMuestra() = runTest(dispatcher) {
        val aviso = "El enlace no es de un gafete PRAIND."
        val vm = nuevoVm(FuenteFalsa(errorCarga = aviso))
        vm.previsualizar(2L, "BETO SOLIS", enlace)
        advanceUntilIdle()
        assertEquals(CargaCarne.Error("BETO SOLIS", aviso), vm.carga.value)
    }

    @Test
    fun laCamaraSoloAceptaEnlacesWeb() {
        assertEquals(enlace, EnlaceQrPraind.desdeQr("  $enlace "))
        assertEquals("http://prd.pw/x", EnlaceQrPraind.desdeQr("http://prd.pw/x"))
        assertEquals(null, EnlaceQrPraind.desdeQr("brisas-acceso://vincular?codigo=ABC"))
        assertEquals(null, EnlaceQrPraind.desdeQr("117380396"))
        assertEquals(null, EnlaceQrPraind.desdeQr("https://"))
    }

    @Test
    fun ofreceVerCarneSoloSiElContratistaTiene() = runTest(dispatcher) {
        val vm = nuevoVm(FuenteFalsa())
        vm.consultar(1L)
        advanceUntilIdle()
        assertTrue(vm.disponible.value)
        vm.consultar(2L)
        advanceUntilIdle()
        assertFalse(vm.disponible.value)
        vm.consultar(null)
        advanceUntilIdle()
        assertFalse(vm.disponible.value)
    }

    @Test
    fun abrirMuestraLaImagenYCerrarLaQuita() = runTest(dispatcher) {
        val vm = nuevoVm(FuenteFalsa())
        vm.abrir(1L, "ANA PEREZ")
        assertEquals(VistaCarne.Cargando("ANA PEREZ"), vm.vista.value)
        advanceUntilIdle()
        val vista = vm.vista.value as VistaCarne.Imagen
        assertArrayEquals(byteArrayOf(1, 2, 3), vista.bytes)
        vm.cerrar()
        assertEquals(VistaCarne.Cerrada, vm.vista.value)
    }

    @Test
    fun sinInternetMuestraElAvisoDelNucleo() = runTest(dispatcher) {
        val aviso = "Para ver este gafete PRAIND por primera vez, conéctese a internet."
        val vm = nuevoVm(FuenteFalsa(error = aviso))
        vm.abrir(1L, "ANA PEREZ")
        advanceUntilIdle()
        assertEquals(VistaCarne.Error("ANA PEREZ", aviso), vm.vista.value)
    }
}
