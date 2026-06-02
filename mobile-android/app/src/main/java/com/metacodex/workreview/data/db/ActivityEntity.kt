package com.metacodex.workreview.data.db

import androidx.room.Entity
import androidx.room.Index
import androidx.room.ColumnInfo
import androidx.room.PrimaryKey

@Entity(
    tableName = "activities",
    indices = [Index("timestamp", "app_name"), Index("browser_url")]
)
data class ActivityEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val timestamp: Long,
    @ColumnInfo(name = "app_name")
    val appName: String,
    @ColumnInfo(name = "window_title")
    val windowTitle: String,
    @ColumnInfo(name = "screenshot_path")
    val screenshotPath: String = "",
    @ColumnInfo(name = "ocr_text")
    val ocrText: String? = null,
    val category: String,
    val duration: Long,
    @ColumnInfo(name = "browser_url")
    val browserUrl: String? = null,
    @ColumnInfo(name = "executable_path")
    val executablePath: String? = null,
    @ColumnInfo(name = "semantic_category")
    val semanticCategory: String? = null,
    @ColumnInfo(name = "semantic_confidence")
    val semanticConfidence: Int? = null,
    @ColumnInfo(name = "intent_purpose")
    val intentPurpose: String? = null,
    @ColumnInfo(name = "intent_note")
    val intentNote: String? = null,
    @ColumnInfo(name = "intent_start_timestamp")
    val intentStartTimestamp: Long? = null,
    @ColumnInfo(name = "intent_end_timestamp")
    val intentEndTimestamp: Long? = null,
    @ColumnInfo(name = "intent_completed_at")
    val intentCompletedAt: Long? = null,
    @ColumnInfo(name = "mobile_source")
    val mobileSource: String = "android"
)
