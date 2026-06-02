package com.metacodex.workreview.data.export

object ExportSchedule {
    fun parseTimes(value: String): List<String> =
        value.split(',', '\n', ';')
            .map { it.trim() }
            .filter { it.isNotBlank() }
            .map(::normalizeTime)
            .distinct()
            .ifEmpty { listOf(DEFAULT_TIME) }

    fun normalizeTime(value: String): String {
        val parts = value.trim().split(':')
        val hour = parts.getOrNull(0)?.toIntOrNull()?.coerceIn(0, 23) ?: 11
        val minute = parts.getOrNull(1)?.toIntOrNull()?.coerceIn(0, 59) ?: 30
        return "%02d:%02d".format(hour, minute)
    }

    const val DEFAULT_TIME = "11:30"
}
