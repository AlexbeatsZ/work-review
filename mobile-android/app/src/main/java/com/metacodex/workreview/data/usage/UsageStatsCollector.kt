package com.metacodex.workreview.data.usage

import android.app.usage.UsageEvents
import android.app.usage.UsageStatsManager
import android.content.Context
import com.metacodex.workreview.data.db.AppEventEntity
import com.metacodex.workreview.data.db.AppSessionEntity
import com.metacodex.workreview.data.tagging.AutoTagger
import com.metacodex.workreview.domain.sessionize.UsageEventModel
import com.metacodex.workreview.domain.sessionize.UsageSessionizer

class UsageStatsCollector(
    private val context: Context,
    private val minSessionMs: Long = 60_000L,
    private val mergeGapMs: Long = 15_000L,
    private val sessionizer: UsageSessionizer = UsageSessionizer(
        minSessionMs = minSessionMs,
        mergeGapMs = mergeGapMs
    )
) {
    private val usageStatsManager =
        context.getSystemService(Context.USAGE_STATS_SERVICE) as UsageStatsManager
    private val labelResolver = PackageLabelResolver(context)

    fun collect(startTs: Long, endTs: Long): CollectionResult {
        val rawEvents = queryEvents(startTs, endTs)
        val sessions = sessionizer.sessionize(rawEvents, startTs, endTs, labelResolver)
        return CollectionResult(
            events = rawEvents.map {
                AppEventEntity(
                    ts = it.ts,
                    packageName = it.packageName,
                    className = it.className,
                    eventType = it.eventType
                )
            },
            sessions = sessions.map {
                val appName = it.appLabel ?: it.packageName
                val classification = AutoTagger.classifyApp(appName, it.packageName)
                AppSessionEntity(
                    startTs = it.startTs,
                    endTs = it.endTs,
                    durationMs = it.durationMs,
                    packageName = it.packageName,
                    appLabel = it.appLabel,
                    source = it.source,
                    confidence = it.confidence,
                    category = classification.category,
                    semanticCategory = classification.semanticCategory,
                    semanticConfidence = classification.semanticConfidence
                )
            }
        )
    }

    private fun queryEvents(startTs: Long, endTs: Long): List<UsageEventModel> {
        val usageEvents = usageStatsManager.queryEvents(startTs, endTs)
        val event = UsageEvents.Event()
        val output = mutableListOf<UsageEventModel>()
        while (usageEvents.hasNextEvent()) {
            usageEvents.getNextEvent(event)
            output += UsageEventModel(
                ts = event.timeStamp,
                packageName = event.packageName ?: "",
                className = event.className,
                eventType = event.eventType
            )
        }
        return output
    }
}

data class CollectionResult(
    val events: List<AppEventEntity>,
    val sessions: List<AppSessionEntity>
)
