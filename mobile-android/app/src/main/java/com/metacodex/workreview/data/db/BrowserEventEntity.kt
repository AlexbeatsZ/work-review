package com.metacodex.workreview.data.db

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "browser_events",
    indices = [Index("ts"), Index("url"), Index("eventType")]
)
data class BrowserEventEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val ts: Long,
    val browserPackage: String = "mark.via.gp",
    val url: String,
    val title: String?,
    val referrer: String?,
    val eventType: String,
    val durationMs: Long?,
    val source: String = "via_userscript"
)
