package com.metacodex.workreview.data.db

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "app_sessions",
    indices = [Index("startTs"), Index("endTs"), Index("packageName")]
)
data class AppSessionEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val startTs: Long,
    val endTs: Long,
    val durationMs: Long,
    val packageName: String,
    val appLabel: String?,
    val source: String = "usage_stats",
    val confidence: Double = 1.0,
    val category: String = "other",
    val semanticCategory: String? = null,
    val semanticConfidence: Int? = null
)
