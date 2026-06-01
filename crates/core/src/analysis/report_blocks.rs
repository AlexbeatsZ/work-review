//! 日报"动态统计区块"占位符与读时替换
//!
//! 解决 issue #80 / #79.cmt3：保存的日报 markdown 里固化的时长数字会变陈旧（自动生成
//! 触发后用户继续工作的活动不会被回填）。读取/展示时使用最新的 DailyStats 重新渲染
//! 这些区块，保证每次看到的数字都是当前的。
//!
//! 工作方式：
//! 1. 各 generator 调用 `wrap_block(BLOCK_*, rendered)` 把每个统计区块包进
//!    `<!-- WR_BLOCK_START:* --> ... <!-- WR_BLOCK_END:* -->` 标记。
//! 2. 读取时调用 `render_report_with_live_stats(saved_content, &fresh_stats, locale)`
//!    扫描标记并用基于当前 stats 的渲染结果原地替换。
//! 3. 找不到任何标记 → 老报告，原样返回；某个块在保存内容中没出现（生成时数据空）→
//!    保持其不存在，不强行插入（避免破坏非该 locale 的格式）。

use crate::analysis::{
    format_duration_for_locale, generate_hourly_activity_summary_for_locale,
    translate_category_name, translate_semantic_category_name, AppLocale,
};
use crate::database::{DailyStats, DomainUsage};

pub const BLOCK_CATEGORY_TABLE: &str = "CATEGORY_TABLE";
pub const BLOCK_APP_USAGE_TABLE: &str = "APP_USAGE_TABLE";
pub const BLOCK_HOURLY_SUMMARY: &str = "HOURLY_SUMMARY";
pub const BLOCK_DOMAIN_USAGE_TABLE: &str = "DOMAIN_USAGE_TABLE";
pub const BLOCK_LOCAL_OVERVIEW: &str = "LOCAL_OVERVIEW";
pub const BLOCK_LOCAL_CATEGORY: &str = "LOCAL_CATEGORY";
pub const BLOCK_LOCAL_APP_USAGE: &str = "LOCAL_APP_USAGE";
pub const BLOCK_LOCAL_DOMAIN_USAGE: &str = "LOCAL_DOMAIN_USAGE";

const BLOCK_PREFIX_START: &str = "<!-- WR_BLOCK_START:";
const BLOCK_PREFIX_END: &str = "<!-- WR_BLOCK_END:";
const MARKER_SUFFIX: &str = " -->";

/// 把内容包入占位符标记中。如果传入 content 是空串则返回空串（不插入空块）。
pub fn wrap_block(name: &str, content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    format!(
        "{prefix_start}{name}{suffix}\n{content}{newline}{prefix_end}{name}{suffix}\n",
        prefix_start = BLOCK_PREFIX_START,
        prefix_end = BLOCK_PREFIX_END,
        suffix = MARKER_SUFFIX,
        newline = if content.ends_with('\n') { "" } else { "\n" }
    )
}

/// 读时用最新 stats 重新渲染所有已知统计区块。未识别 / 未出现的块原样保留。
pub fn render_report_with_live_stats(
    content: &str,
    stats: &DailyStats,
    locale: AppLocale,
) -> String {
    let mut output = content.to_string();
    output = replace_block(
        &output,
        BLOCK_CATEGORY_TABLE,
        &render_category_table(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_APP_USAGE_TABLE,
        &render_app_usage_table(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_HOURLY_SUMMARY,
        &render_hourly_summary(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_DOMAIN_USAGE_TABLE,
        &render_domain_usage_table(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_OVERVIEW,
        &render_local_overview(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_CATEGORY,
        &render_local_category_list(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_APP_USAGE,
        &render_local_app_usage_list(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_DOMAIN_USAGE,
        &render_local_domain_usage_list(stats, locale),
    );
    output
}

fn replace_block(content: &str, name: &str, fresh: &str) -> String {
    let start = format!("{BLOCK_PREFIX_START}{name}{MARKER_SUFFIX}");
    let end = format!("{BLOCK_PREFIX_END}{name}{MARKER_SUFFIX}");

    let mut result = String::with_capacity(content.len());
    let mut cursor = 0usize;

    loop {
        let Some(rel_start) = content[cursor..].find(&start) else {
            result.push_str(&content[cursor..]);
            break;
        };
        let abs_start = cursor + rel_start;
        result.push_str(&content[cursor..abs_start]);

        let Some(rel_end) = content[abs_start..].find(&end) else {
            // 未配对，原样保留
            result.push_str(&content[abs_start..]);
            break;
        };
        let abs_end = abs_start + rel_end + end.len();

        result.push_str(&start);
        result.push('\n');
        let trimmed = fresh.trim_matches('\n');
        if !trimmed.is_empty() {
            result.push_str(trimmed);
            result.push('\n');
        }
        result.push_str(&end);
        cursor = abs_end;
    }

    result
}

// ─────────── summary mode 的块渲染器 ───────────

pub fn render_category_table(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.category_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 二、时间分配\n\n| 类别 | 时长 | 占比 |\n|:--|--:|--:|\n",
        AppLocale::ZhTw => "## 二、時間分配\n\n| 類別 | 時長 | 佔比 |\n|:--|--:|--:|\n",
        AppLocale::En => {
            "## 2. Time Allocation\n\n| Category | Duration | Share |\n|:--|--:|--:|\n"
        }
    });
    for cat in &stats.category_usage {
        let percentage = if stats.total_duration > 0 {
            (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
        } else {
            0
        };
        out.push_str(&format!(
            "| {} | {} | {}% |\n",
            translate_category_name(&cat.category, locale),
            format_duration_for_locale(cat.duration, locale),
            percentage
        ));
    }
    out.push('\n');
    out
}

pub fn render_app_usage_table(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.app_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 三、应用使用明细\n\n| 序号 | 应用名称 | 使用时长 |\n|--:|:--|--:|\n",
        AppLocale::ZhTw => "## 三、應用使用明細\n\n| 序號 | 應用名稱 | 使用時長 |\n|--:|:--|--:|\n",
        AppLocale::En => "## 3. App Details\n\n| # | App | Duration |\n|--:|:--|--:|\n",
    });
    for (index, app) in stats.app_usage.iter().enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            index + 1,
            app.app_name,
            format_duration_for_locale(app.duration, locale)
        ));
    }
    out.push('\n');
    out
}

pub fn render_hourly_summary(stats: &DailyStats, locale: AppLocale) -> String {
    let Some(hourly) = generate_hourly_activity_summary_for_locale(stats, locale) else {
        return String::new();
    };
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 四、按小时活跃度\n\n",
        AppLocale::ZhTw => "## 四、按小時活躍度\n\n",
        AppLocale::En => "## 4. Hourly Activity\n\n",
    });
    out.push_str(&hourly);
    out.push('\n');
    out
}

pub fn render_domain_usage_table(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.domain_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 五、网站访问明细\n\n| 序号 | 网站域名 | 访问时长 |\n|--:|:--|--:|\n",
        AppLocale::ZhTw => "## 五、網站造訪明細\n\n| 序號 | 網站網域 | 造訪時長 |\n|--:|:--|--:|\n",
        AppLocale::En => "## 5. Website Details\n\n| # | Domain | Duration |\n|--:|:--|--:|\n",
    });
    for (index, domain) in stats.domain_usage.iter().enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            index + 1,
            format_domain_label_local(domain, locale),
            format_duration_for_locale(domain.duration, locale)
        ));
    }
    out.push('\n');
    out
}

// ─────────── local mode 的块渲染器（与 summary mode 表格风格不同，是列表风格）───────────

pub fn render_local_overview(stats: &DailyStats, locale: AppLocale) -> String {
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 一、今日概览\n\n",
        AppLocale::ZhTw => "## 一、今日概覽\n\n",
        AppLocale::En => "## 1. Overview\n\n",
    });
    let line = match locale {
        AppLocale::ZhCn => format!(
            "- **总工作时长**: {}\n- **截图数量**: {} 张\n- **使用应用**: {} 个\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
        AppLocale::ZhTw => format!(
            "- **總工作時長**: {}\n- **截圖數量**: {} 張\n- **使用應用**: {} 個\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
        AppLocale::En => format!(
            "- **Total work duration**: {}\n- **Screenshots**: {}\n- **Apps used**: {}\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
    };
    out.push_str(&line);
    out
}

pub fn render_local_category_list(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.category_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 二、时间分配\n\n",
        AppLocale::ZhTw => "## 二、時間分配\n\n",
        AppLocale::En => "## 2. Time allocation\n\n",
    });
    for cat in &stats.category_usage {
        let percentage = if stats.total_duration > 0 {
            (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
        } else {
            0
        };
        out.push_str(&format!(
            "- **{}**: {} ({}%)\n",
            translate_category_name(&cat.category, locale),
            format_duration_for_locale(cat.duration, locale),
            percentage
        ));
    }
    out
}

pub fn render_local_app_usage_list(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.app_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 三、应用使用情况\n\n",
        AppLocale::ZhTw => "## 三、應用使用情況\n\n",
        AppLocale::En => "## 3. App usage\n\n",
    });
    for (index, app) in stats.app_usage.iter().take(5).enumerate() {
        out.push_str(&format!(
            "{}. **{}**: {}\n",
            index + 1,
            app.app_name,
            format_duration_for_locale(app.duration, locale)
        ));
    }
    out
}

pub fn render_local_domain_usage_list(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.domain_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 五、网站访问\n\n",
        AppLocale::ZhTw => "## 五、網站造訪\n\n",
        AppLocale::En => "## 5. Website visits\n\n",
    });
    for domain in stats.domain_usage.iter().take(5) {
        out.push_str(&format!(
            "- **{}**: {}\n",
            format_domain_label_local(domain, locale),
            format_duration_for_locale(domain.duration, locale)
        ));
    }
    out
}

fn format_domain_label_local(domain: &DomainUsage, locale: AppLocale) -> String {
    match domain.semantic_category.as_deref().map(str::trim) {
        Some(semantic_category) if !semantic_category.is_empty() => {
            let semantic_category = translate_semantic_category_name(semantic_category, locale);
            match locale {
                AppLocale::En => format!("{} ({})", domain.domain, semantic_category),
                _ => format!("{}（{}）", domain.domain, semantic_category),
            }
        }
        _ => domain.domain.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_block_skips_empty() {
        assert!(wrap_block(BLOCK_CATEGORY_TABLE, "").is_empty());
    }

    #[test]
    fn wrap_block_adds_markers_and_trailing_newline() {
        let wrapped = wrap_block(BLOCK_CATEGORY_TABLE, "hello\n");
        assert!(wrapped.starts_with("<!-- WR_BLOCK_START:CATEGORY_TABLE -->\n"));
        assert!(wrapped.contains("hello\n"));
        assert!(wrapped
            .trim_end()
            .ends_with("<!-- WR_BLOCK_END:CATEGORY_TABLE -->"));
    }

    #[test]
    fn replace_block_overwrites_content_between_markers() {
        let original = "before\n<!-- WR_BLOCK_START:APP_USAGE_TABLE -->\nstale rows\n<!-- WR_BLOCK_END:APP_USAGE_TABLE -->\nafter\n";
        let updated = replace_block(original, BLOCK_APP_USAGE_TABLE, "fresh rows\n");
        assert!(updated.contains("fresh rows"));
        assert!(!updated.contains("stale rows"));
        assert!(updated.starts_with("before\n"));
        assert!(updated.ends_with("after\n"));
    }

    #[test]
    fn replace_block_returns_input_when_no_markers() {
        let original = "no markers here\n";
        let updated = replace_block(original, BLOCK_CATEGORY_TABLE, "ignored");
        assert_eq!(updated, original);
    }

    #[test]
    fn replace_block_handles_unmatched_start() {
        let original = "<!-- WR_BLOCK_START:CATEGORY_TABLE -->\nnever closed\n";
        let updated = replace_block(original, BLOCK_CATEGORY_TABLE, "ignored");
        assert_eq!(updated, original);
    }

    #[test]
    fn render_report_with_live_stats_passes_through_legacy_content() {
        let legacy = "# 工作日报\n\n直接写死的旧版本，没有任何标记\n";
        let stats = DailyStats {
            total_duration: 0,
            screenshot_count: 0,
            app_usage: vec![],
            category_usage: vec![],
            browser_duration: 0,
            url_usage: vec![],
            domain_usage: vec![],
            browser_usage: vec![],
            work_time_duration: 0,
            hourly_activity_distribution: vec![],
        };
        assert_eq!(
            render_report_with_live_stats(legacy, &stats, AppLocale::ZhCn),
            legacy
        );
    }
}
