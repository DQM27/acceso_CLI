package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LectorDocumentosIdentidadTest {

    @Test
    fun clasificaCedulaNacional() {
        val texto = "TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA"
        assertEquals(TipoDocumento.CEDULA_NACIONAL, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaCedulaResidencia() {
        val texto = "DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA\nRESIDENTE PERMANENTE"
        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaLicenciaNacional() {
        val texto = "Licencia de Conducir\nNº: 112340567"
        assertEquals(TipoDocumento.LICENCIA_NACIONAL, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaLicenciaExtranjero() {
        val texto = "Licencia de Conducir\nNº: DM-999888777"
        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaDesconocidoSinSenales() {
        val texto = "Documento sin ninguna palabra clave reconocible"
        assertEquals(TipoDocumento.DESCONOCIDO, clasificarTipoDocumento(texto))
    }

    // --- Carnet de inducción PRAIND -- dos variantes de diseño reales ---

    @Test
    fun praindVarianteConEncabezadoCarnetDeInduccionAlSite() {
        val texto = """
            CARNET DE INDUCCIÓN AL SITE
            CEDIS COSTA RICA
            Nombre: Marco Anthony Jimenez Reyes
            No. de cédula: 155834532920
            Empresa: Expenic Ing S.A
            Fecha de inducción: 09/07/2026
            Fecha de vencimiento de
            inducción: 08/07/2028
            COCA COLA FEMSA COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_INDUCCION_PRAIND, doc?.tipo)
        assertEquals("155834532920", doc?.numeroDocumento)
        assertEquals("Marco Anthony Jimenez Reyes", doc?.nombre)
        assertEquals(FechaDocumento(8, 7, 2028), doc?.vencimiento)
    }

    @Test
    fun praindVarianteConPieDeNormasDeSeguridad() {
        // Segunda variante de diseño: el texto "CARNET DE INDUCCIÓN" está en
        // el pie de página, no en el encabezado, y el número de cédula acá
        // tiene 9 dígitos (el mismo largo que una cédula nacional) -- por
        // eso la clasificación de PRAIND tiene que ganarle a la de cédula
        // nacional, no sólo coincidir con ella.
        val texto = """
            Nombre: Mariela Cordero Salazar
            No. de cédula: 113850770
            Empresa: Expenic Ing S.A
            Fecha de inducción: 29/07/2026
            Fecha de vencimiento de
            inducción: 29/07/2027
            CARNET DE INDUCCIÓN EN NORMAS DE SEGURIDAD, AMBIENTE, CALIDAD E INOCUIDAD PARA CONTRATISTAS ML-SC-RGCR0369
            COCA COLA FEMSA COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_INDUCCION_PRAIND, doc?.tipo)
        assertEquals("113850770", doc?.numeroDocumento)
        assertEquals("Mariela Cordero Salazar", doc?.nombre)
        assertEquals(FechaDocumento(29, 7, 2027), doc?.vencimiento)
    }

    // --- Carnets de contratista in-house / BAC ---

    @Test
    fun inHouseFrenteBuscaPorNombreSiNoTraeCedulaVisible() {
        val texto = """
            Wardner Eduardo
            Marin Umaña
            CONTRATISTA
            COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_IN_HOUSE, doc?.tipo)
        assertEquals("Wardner Eduardo Marin Umaña", doc?.textoBusqueda)
        assertEquals("Wardner Eduardo Marin Umaña", doc?.nombre)
    }

    @Test
    fun inHouseReversoBuscaPorCedula() {
        val texto = """
            EMPRESA:
            ALDAMA
            CEDULA:
            172400408127
            CONTRATISTA
            COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_IN_HOUSE, doc?.tipo)
        assertEquals("172400408127", doc?.numeroDocumento)
    }

    @Test
    fun bacFrenteBuscaPorCedula() {
        val texto = """
            BAC
            ANTHONNY JOSE MURILLO RAMIREZ
            116030489
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_BAC, doc?.tipo)
        assertEquals("116030489", doc?.numeroDocumento)
        assertEquals("ANTHONNY JOSE MURILLO RAMIREZ", doc?.nombre)
    }

    @Test
    fun gafeteContratistaReconoceCodigoCrcSinUsarloComoCedula() {
        val texto = """
            CARNÉ
            PROVISIONAL
            CRC - 16
            CONTRATISTAS
            Costa Rica
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.GAFETE_CONTRATISTA, doc?.tipo)
        assertEquals("16", doc?.numeroDocumento)
        assertEquals("CRC 16", doc?.textoBusqueda)
    }

    @Test
    fun carneProvisionalPermisoLaboralBuscaPorDocumento() {
        val texto = """
            Carné Provisional - Permiso Laboral
            Expediente 135 788985
            N° Documento 155846198814
            CATEGORIA ESPECIAL R
            Primer Apellido CASTILLO
            Segundo Apellido MONTIEL
            Fecha Vencimiento 14/03/2027
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertEquals("155846198814", doc?.numeroDocumento)
        assertEquals(FechaDocumento(14, 3, 2027), doc?.vencimiento)
    }

    // --- Caso central del plan: Documento No. vs Expediente No. ---

    @Test
    fun dimexExtraeDocumentoNoExpediente() {
        val texto = """
            DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA
            REPÚBLICA DE COSTA RICA
            RESIDENTE PERMANENTE
            LIBRE CONDICIÓN
            Apellidos:
            PEREZ RAMIREZ
            Nombre:
            MARIA JOSE
            Nacionalidad:
            NICARAGUA
            Documento No.: 999888777
            Expediente No.: 135-453544
            Vence: 28 07 2026
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertEquals("999888777", doc?.numeroDocumento)
        assertEquals("MARIA JOSE", doc?.nombre)
        assertEquals("PEREZ RAMIREZ", doc?.apellidos)
        assertEquals("NICARAGUA", doc?.nacionalidad)
        assertEquals(FechaDocumento(28, 7, 2026), doc?.vencimiento)
    }

    @Test
    fun dimexNoConfundeConNumeroDeExpedienteSiApareceAntes() {
        val texto = "Expediente No.: 135-453544\nDocumento No.: 999888777"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals("999888777", doc?.numeroDocumento)
    }

    // --- Licencias ---

    @Test
    fun licenciaNacionalNoMarcaExtranjero() {
        val texto = "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2026"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.LICENCIA_NACIONAL, doc?.tipo)
        assertEquals("112340567", doc?.numeroDocumento)
        assertEquals(false, doc?.esExtranjero)
    }

    @Test
    fun licenciaNacionalAceptaPrefijoCi() {
        val texto = """
            REPUBLICA DE COSTA RICA
            Licencia de Conducir
            N° CI-205300606
            Expedición 08-08-2024
            Nacimiento 04-02-1978
            Vencimiento 08-08-2027
            Tipo B3
            GUTIERREZ MARTINEZ JUAN JOSE
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.LICENCIA_NACIONAL, doc?.tipo)
        assertEquals("205300606", doc?.numeroDocumento)
        assertEquals(false, doc?.esExtranjero)
        assertEquals(FechaDocumento(8, 8, 2027), doc?.vencimiento)
    }

    @Test
    fun licenciaExtranjeroRemuevePrefijoDM() {
        val texto = "Licencia de Conducir\nNº: DM-999888777\nVencimiento 03-04-2026"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("999888777", doc?.numeroDocumento)
        assertTrue(doc?.esExtranjero == true)
        assertEquals(FechaDocumento(3, 4, 2026), doc?.vencimiento)
    }

    @Test
    fun licenciaExtranjeroAceptaDmAunqueOcrPierdaEtiquetaNumero() {
        val texto = """
            REPUBLICA DE COSTA RICA
            Licencia de Conducir
            DM-155824395105
            Expedición 03-04-2023
            Vencimiento 03-04-2026
            Tipo A3
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("155824395105", doc?.numeroDocumento)
        assertTrue(doc?.esExtranjero == true)
        assertEquals(FechaDocumento(3, 4, 2026), doc?.vencimiento)
    }

    // --- Vigencia ---

    @Test
    fun documentoVigenteSiVencimientoEsFuturo() {
        val vencimiento = FechaDocumento(28, 7, 2026)
        val hoy = FechaDocumento(8, 9, 2025)
        assertEquals(false, vencimiento.estaVencida(hoy))
    }

    @Test
    fun documentoVencidoSiVencimientoEsPasado() {
        val vencimiento = FechaDocumento(28, 7, 2024)
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(true, vencimiento.estaVencida(hoy))
    }

    // --- Casos sin match ---

    @Test
    fun devuelveNullSiNoHaySuficienteInformacion() {
        assertNull(leerDocumentoDeTexto("Documento sin numeros completos"))
    }

    // --- Edad / TIM vs cédula de adulto ---

    @Test
    fun edadCuandoYaPasoElCumpleañosEsteAnio() {
        val nacimiento = FechaDocumento(15, 6, 2000)
        val hoy = FechaDocumento(8, 9, 2026) // cumpleaños (15/06) ya pasó
        assertEquals(26, nacimiento.edadEnAnios(hoy))
    }

    @Test
    fun edadCuandoAunNoLlegaElCumpleañosEsteAnio() {
        val nacimiento = FechaDocumento(15, 12, 2000)
        val hoy = FechaDocumento(8, 9, 2026) // cumpleaños (15/12) todavía no llega
        assertEquals(25, nacimiento.edadEnAnios(hoy))
    }

    @Test
    fun edadElMismoDiaDelCumpleañosYaCuentaComoCumplida() {
        val nacimiento = FechaDocumento(8, 9, 2000)
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(26, nacimiento.edadEnAnios(hoy))
    }

    @Test
    fun reclasificaCedulaNacionalDeMenorComoTim() {
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_NACIONAL,
            numeroDocumento = "202020202",
            fechaNacimiento = FechaDocumento(15, 6, 2018),
        )
        val hoy = FechaDocumento(8, 9, 2026) // 8 años
        assertEquals(TipoDocumento.TARJETA_IDENTIDAD_MENOR, documento.reclasificarPorEdad(hoy).tipo)
    }

    @Test
    fun noReclasificaCedulaNacionalDeAdulto() {
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_NACIONAL,
            numeroDocumento = "101110111",
            fechaNacimiento = FechaDocumento(15, 6, 1990),
        )
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(TipoDocumento.CEDULA_NACIONAL, documento.reclasificarPorEdad(hoy).tipo)
    }

    @Test
    fun noReclasificaOtrosTiposAunqueSeanMenoresDeEdad() {
        // La regla es específica de cédula nacional -- DIMEX/licencia/pasaporte
        // de un menor no deben convertirse en TIM (no es lo que son).
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_RESIDENCIA,
            numeroDocumento = "999888777",
            fechaNacimiento = FechaDocumento(15, 6, 2018),
        )
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, documento.reclasificarPorEdad(hoy).tipo)
    }

    @Test
    fun noReclasificaSiNoHayFechaDeNacimiento() {
        val documento = DocumentoDetectado(tipo = TipoDocumento.CEDULA_NACIONAL, numeroDocumento = "101110111")
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(TipoDocumento.CEDULA_NACIONAL, documento.reclasificarPorEdad(hoy).tipo)
    }
}
