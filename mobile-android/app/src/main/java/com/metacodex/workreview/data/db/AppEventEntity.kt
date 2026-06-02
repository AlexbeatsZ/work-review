package com.metacodex.workreview.data.db

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "app_events",
    indices = [Index("ts"), Index("packageName")]
)
data class AppEventEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val ts: Long,
    val packageName: String,
    val className: String?,
    val eventType: Int,
    val source: String = "usage_stats"
)
