package com.metacodex.workreview.data.usage

import android.content.Context
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.graphics.Bitmap
import androidx.collection.LruCache
import androidx.core.graphics.drawable.toBitmap

data class AppVisual(
    val label: String,
    val icon: Bitmap?
)

class PackageVisualResolver(context: Context) {
    private val packageManager = context.applicationContext.packageManager
    private val cache = LruCache<String, AppVisual>(160)

    fun resolve(packageName: String, fallbackLabel: String? = null): AppVisual {
        cache[packageName]?.let { cached ->
            if (fallbackLabel.isNullOrBlank() || cached.label != resolveAppDisplayName(packageName, null)) {
                return cached
            }
        }

        val applicationInfo = runCatching { applicationInfo(packageName) }.getOrNull()
        val resolvedLabel = applicationInfo?.let { info ->
            runCatching { packageManager.getApplicationLabel(info).toString() }.getOrNull()
        }
        val icon = applicationInfo?.let { info ->
            runCatching {
                packageManager.getApplicationIcon(info).toBitmap(
                    width = APP_ICON_SIZE,
                    height = APP_ICON_SIZE,
                    config = Bitmap.Config.ARGB_8888
                )
            }.getOrNull()
        }
        return AppVisual(
            label = resolveAppDisplayName(packageName, resolvedLabel ?: fallbackLabel),
            icon = icon
        ).also { cache.put(packageName, it) }
    }

    @Suppress("DEPRECATION")
    private fun applicationInfo(packageName: String): ApplicationInfo =
        packageManager.getApplicationInfo(packageName, PackageManager.GET_META_DATA)

    private companion object {
        const val APP_ICON_SIZE = 96
    }
}
