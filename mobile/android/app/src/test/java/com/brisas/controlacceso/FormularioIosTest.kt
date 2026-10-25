package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FormularioIosTest {
    @Test
    fun `la fecha va y vuelve del selector sin cambiar`() {
        val millis = fechaDDMMAAAAaMillis("12-11-2026")!!
        assertEquals("12-11-2026", millisAFechaDDMMAAAA(millis))
        assertEquals("01-02-2027", millisAFechaDDMMAAAA(fechaDDMMAAAAaMillis("01-02-2027")!!))
    }

    @Test
    fun `una lectura a medias o invalida no se convierte`() {
        assertNull(fechaDDMMAAAAaMillis(""))
        assertNull(fechaDDMMAAAAaMillis("10-09-202"))
        assertNull(fechaDDMMAAAAaMillis("31-02-2026"))
        assertNull(fechaDDMMAAAAaMillis("aa-bb-cccc"))
    }

    @Test
    fun `la fecha se lee como en iOS y lo incompleto se muestra tal cual`() {
        assertEquals("12 nov 2026", textoFechaLegible("12-11-2026"))
        assertEquals("1 ene 2027", textoFechaLegible("01-01-2027"))
        assertEquals("10-09-202", textoFechaLegible("10-09-202"))
        assertNull(textoFechaLegible(""))
    }

    @Test
    fun `lo elegido en el selector pasa intacto por la mascara del ViewModel`() {
        val elegida = millisAFechaDDMMAAAA(fechaDDMMAAAAaMillis("05-03-2028")!!)
        assertEquals(elegida, formatearFechaDDMMYYYY(elegida))
    }
}
