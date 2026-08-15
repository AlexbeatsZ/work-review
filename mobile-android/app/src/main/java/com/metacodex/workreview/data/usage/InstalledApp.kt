package com.metacodex.workreview.data.usage

import android.graphics.Bitmap

data class InstalledApp(
    val packageName: String,
    val label: String,
    val icon: Bitmap?,
    val isIgnored: Boolean
)
