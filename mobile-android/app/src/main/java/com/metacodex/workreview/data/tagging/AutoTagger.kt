package com.metacodex.workreview.data.tagging

import java.net.URI

data class Classification(
    val category: String,
    val semanticCategory: String?,
    val semanticConfidence: Int
)

object AutoTagger {
    fun classifyApp(appName: String, packageName: String): Classification {
        val text = "$appName $packageName".lowercase()
        val category = when {
            listOf("code", "studio", "github", "git", "term", "jetbrains", "cursor").any { text.contains(it) } -> "development"
            listOf("via", "browser", "chrome", "firefox", "edge").any { text.contains(it) } -> "browser"
            listOf("qq", "wechat", "weixin", "telegram", "discord", "slack", "teams").any { text.contains(it) } -> "communication"
            listOf("obsidian", "notion", "office", "word", "excel", "wps", "note").any { text.contains(it) } -> "office"
            listOf("figma", "sketch", "photoshop", "canva").any { text.contains(it) } -> "design"
            listOf("bilibili", "youtube", "music", "game", "steam", "douyin").any { text.contains(it) } -> "entertainment"
            else -> "other"
        }
        return Classification(category, semanticForCategory(category), 80)
    }

    fun classifyUrl(url: String, title: String?): Classification {
        val host = runCatching { URI(url).host?.lowercase() }.getOrNull() ?: ""
        val text = "$host ${title.orEmpty()}".lowercase()
        val semantic = when {
            listOf("github.com", "stackoverflow.com", "developer.android.com", "docs.rs", "npmjs.com").any { host.endsWith(it) } -> "编码开发"
            listOf("docs.", "wiki", "readthedocs", "medium.com", "juejin.cn", "zhihu.com").any { text.contains(it) } -> "资料阅读"
            listOf("mail.", "gmail.com", "outlook.", "qq.com").any { text.contains(it) } -> "通讯协作"
            listOf("bilibili.com", "youtube.com", "douyin.com", "netflix.com").any { host.endsWith(it) } -> "休息娱乐"
            else -> "网页浏览"
        }
        val base = if (semantic == "休息娱乐") "entertainment" else "browser"
        return Classification(base, semantic, 78)
    }

    private fun semanticForCategory(category: String): String? =
        when (category) {
            "development" -> "编码开发"
            "browser" -> "网页浏览"
            "communication" -> "通讯协作"
            "office" -> "文档办公"
            "design" -> "设计创作"
            "entertainment" -> "休息娱乐"
            else -> null
        }
}
