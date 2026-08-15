package com.metacodex.workreview.data.export

import org.junit.Assert.assertEquals
import org.junit.Test

class ExportScheduleTest {
    @Test
    fun parsesMultipleTimesAndDeduplicates() {
        assertEquals(
            listOf("08:30", "11:30", "18:00"),
            ExportSchedule.parseTimes("08:30, 11:30\n18:00\n08:30")
        )
    }

    @Test
    fun normalizesInvalidTimesToDefaultShape() {
        assertEquals(listOf("23:59", "11:30"), ExportSchedule.parseTimes("99:99\nabc"))
    }

    @Test
    fun fallsBackToDefaultWhenEmpty() {
        assertEquals(listOf("11:30"), ExportSchedule.parseTimes(""))
    }
}
