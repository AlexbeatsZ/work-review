package com.metacodex.workreview.data.usage

private val knownAppNames = mapOf(
    "com.tencent.mobileqq" to "QQ",
    "com.tencent.mm" to "微信",
    "md.obsidian" to "Obsidian",
    "mark.via.gp" to "Via"
)

fun resolveAppDisplayName(packageName: String, resolvedLabel: String?): String {
    resolvedLabel
        ?.trim()
        ?.takeIf { it.isNotBlank() && !it.equals(packageName, ignoreCase = true) }
        ?.let { return it }

    knownAppNames[packageName]?.let { return it }

    return packageName
        .substringAfterLast('.')
        .replace('_', ' ')
        .replace('-', ' ')
        .split(' ')
        .filter { it.isNotBlank() }
        .joinToString(" ") { word -> word.replaceFirstChar { it.uppercase() } }
        .ifBlank { packageName }
}
