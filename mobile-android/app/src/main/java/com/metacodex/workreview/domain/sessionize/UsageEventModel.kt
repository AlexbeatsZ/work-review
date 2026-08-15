package com.metacodex.workreview.domain.sessionize

data class UsageEventModel(
    val ts: Long,
    val packageName: String,
    val className: String?,
    val eventType: Int
)

data class UsageSessionModel(
    val startTs: Long,
    val endTs: Long,
    val durationMs: Long,
    val packageName: String,
    val appLabel: String?,
    val source: String = "usage_stats",
    val confidence: Double = 1.0
)

interface AppLabelResolver {
    fun labelFor(packageName: String): String?
}
