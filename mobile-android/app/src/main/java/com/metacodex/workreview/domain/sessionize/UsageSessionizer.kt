package com.metacodex.workreview.domain.sessionize

import java.time.Instant
import java.time.ZoneId
import java.time.ZonedDateTime

class UsageSessionizer(
    private val zoneId: ZoneId = ZoneId.systemDefault(),
    private val maxSessionMs: Long = 12L * 60L * 60L * 1000L,
    private val minSessionMs: Long = 60_000L,
    private val mergeGapMs: Long = 15_000L,
    private val ignoredPackages: Set<String> = emptySet()
) {
    fun sessionize(
        events: List<UsageEventModel>,
        rangeStart: Long,
        rangeEnd: Long,
        labelResolver: AppLabelResolver
    ): List<UsageSessionModel> {
        if (rangeEnd <= rangeStart) return emptyList()

        val sorted = events
            .asSequence()
            .filter { it.ts in rangeStart..rangeEnd }
            .filter { it.packageName.isNotBlank() }
            .filterNot { ignoredPackages.contains(it.packageName) }
            .sortedBy { it.ts }
            .toList()

        val sessions = mutableListOf<UsageSessionModel>()
        var activePackage: String? = null
        var activeStart: Long? = null

        fun close(endTs: Long, confidence: Double = 1.0) {
            val pkg = activePackage ?: return
            val start = activeStart ?: return
            val boundedStart = start.coerceAtLeast(rangeStart)
            val boundedEnd = endTs.coerceAtMost(rangeEnd)
            if (boundedEnd <= boundedStart) {
                activePackage = null
                activeStart = null
                return
            }
            splitByDay(boundedStart, boundedEnd).forEach { (partStart, partEnd) ->
                val duration = partEnd - partStart
                if (duration > 0 && duration <= maxSessionMs) {
                    sessions += UsageSessionModel(
                        startTs = partStart,
                        endTs = partEnd,
                        durationMs = duration,
                        packageName = pkg,
                        appLabel = labelResolver.labelFor(pkg),
                        confidence = confidence
                    )
                }
            }
            activePackage = null
            activeStart = null
        }

        for (event in sorted) {
            when (event.eventType) {
                EVENT_ACTIVITY_RESUMED, EVENT_MOVE_TO_FOREGROUND -> {
                    if (activePackage != null && activePackage != event.packageName) {
                        close(event.ts, confidence = 0.9)
                    }
                    activePackage = event.packageName
                    activeStart = event.ts
                }
                EVENT_ACTIVITY_PAUSED, EVENT_MOVE_TO_BACKGROUND,
                EVENT_KEYGUARD_SHOWN, EVENT_SCREEN_NON_INTERACTIVE -> {
                    if (activePackage == event.packageName || event.eventType == EVENT_KEYGUARD_SHOWN || event.eventType == EVENT_SCREEN_NON_INTERACTIVE) {
                        close(event.ts)
                    }
                }
            }
        }

        close(rangeEnd, confidence = 0.6)
        return mergeAdjacent(sessions)
            .filter { it.durationMs >= minSessionMs }
            .sortedBy { it.startTs }
    }

    private fun mergeAdjacent(sessions: List<UsageSessionModel>): List<UsageSessionModel> {
        val merged = mutableListOf<UsageSessionModel>()
        sessions.sortedBy { it.startTs }.forEach { current ->
            val previous = merged.lastOrNull()
            if (
                previous != null &&
                previous.packageName == current.packageName &&
                sameLocalDay(previous.startTs, current.startTs) &&
                current.startTs - previous.endTs <= mergeGapMs
            ) {
                merged[merged.lastIndex] = previous.copy(
                    endTs = current.endTs,
                    durationMs = current.endTs - previous.startTs,
                    confidence = minOf(previous.confidence, current.confidence)
                )
            } else {
                merged += current
            }
        }
        return merged
    }

    private fun sameLocalDay(leftTs: Long, rightTs: Long): Boolean =
        ZonedDateTime.ofInstant(Instant.ofEpochMilli(leftTs), zoneId).toLocalDate() ==
            ZonedDateTime.ofInstant(Instant.ofEpochMilli(rightTs), zoneId).toLocalDate()

    private fun splitByDay(startTs: Long, endTs: Long): List<Pair<Long, Long>> {
        val parts = mutableListOf<Pair<Long, Long>>()
        var cursor = startTs
        while (cursor < endTs) {
            val nextDay = ZonedDateTime.ofInstant(Instant.ofEpochMilli(cursor), zoneId)
                .toLocalDate()
                .plusDays(1)
                .atStartOfDay(zoneId)
                .toInstant()
                .toEpochMilli()
            val partEnd = minOf(endTs, nextDay)
            parts += cursor to partEnd
            cursor = partEnd
        }
        return parts
    }

    companion object {
        const val EVENT_MOVE_TO_FOREGROUND = 1
        const val EVENT_MOVE_TO_BACKGROUND = 2
        const val EVENT_SCREEN_NON_INTERACTIVE = 15
        const val EVENT_KEYGUARD_SHOWN = 18
        const val EVENT_ACTIVITY_RESUMED = 23
        const val EVENT_ACTIVITY_PAUSED = 24
    }
}
