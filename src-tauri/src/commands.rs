use crate::analysis::AppLocale;
use crate::config::{
    AppCategoryRule, AppConfig, ManualFollowupItem, CustomSemanticCategory, PrivacyConfig,
    WebsiteSemanticRule,
};
use crate::database::Database;
use crate::database::{
    Activity, AppUsage, BrowserUsage, CategoryUsage, DailyReport, DailyStats, DomainUsage,
    HourlyActivityBucket, HourlyAppBucket, MemorySearchItem, UrlDetail, UrlUsage,
};
use crate::error::AppError;
#[cfg(target_os = "linux")]
use crate::linux_session::{
    current_linux_desktop_environment, current_linux_desktop_session, LinuxDesktopSession,
};
use crate::privacy::PrivacyFilter;
use crate::screenshot::ScreenshotService;
use crate::storage::StorageManager;
use crate::work_intelligence::{
    analyze_intents, build_work_sessions, extract_todos,
    generate_weekly_review as build_weekly_review, IntentAnalysisResult, TodoExtractionResult,
    WeeklyReviewResult, WorkSession,
};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use work_review_core::categorize::is_merged_domain;

const MANAGED_DATA_ENTRIES: &[&str] = &[
    "config.json",
    "workreview.db",
    "screenshots",
    "ocr_logs",
    "background.jpg",
];
const LIVE_DATABASE_FILES: &[&str] = &["workreview.db", "workreview.db-shm", "workreview.db-wal"];

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppCategoryOverviewItem {
    pub app_name: String,
    pub category: String,
    pub total_duration: i64,
    pub is_overridden: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LinuxSessionSupportInfo {
    pub platform: String,
    pub session_type: String,
    pub desktop_environment: String,
    pub active_window_provider: String,
    pub active_window_supported: bool,
    pub screenshot_supported: bool,
    pub browser_url_support_level: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ManualFollowupInput {
    pub title: String,
    pub note: Option<String>,
    pub date: String,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
    pub project_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentNoteInput {
    pub purpose: String,
    pub note: Option<String>,
    pub start_timestamp: i64,
    pub end_timestamp: Option<i64>,
}

fn resolve_saved_report_metadata(
    configured_mode: &crate::config::AiMode,
    configured_model_name: &str,
    used_ai: bool,
) -> (String, Option<String>) {
    let configured_mode = format!("{configured_mode:?}").to_lowercase();

    match (configured_mode.as_str(), used_ai) {
        ("summary", false) => ("local".to_string(), None),
        ("cloud", false) => ("local".to_string(), None),
        (_, false) => (configured_mode, None),
        _ => {
            let model_name = configured_model_name.trim();
            (
                configured_mode,
                if model_name.is_empty() {
                    None
                } else {
                    Some(model_name.to_string())
                },
            )
        }
    }
}

#[allow(dead_code)]
fn normalize_saved_report_ai_mode(value: &str) -> String {
    value.trim().to_lowercase()
}

fn build_daily_report_export_path(export_dir: &Path, date: &str) -> PathBuf {
    let safe_date = date.replace('/', "-").replace('\\', "-");
    export_dir.join(format!("{safe_date}.md"))
}

fn export_daily_report_markdown(
    export_dir: &Path,
    date: &str,
    content: &str,
) -> Result<(), AppError> {
    std::fs::create_dir_all(export_dir)?;
    let output_path = build_daily_report_export_path(export_dir, date);
    std::fs::write(output_path, content)?;
    Ok(())
}

fn matches_ignored_app(app_name: &str, ignored_apps: &[String]) -> bool {
    let app_lower = app_name.to_lowercase();
    ignored_apps
        .iter()
        .any(|ignored| app_lower.contains(ignored) || ignored.contains(&app_lower))
}

fn apply_ignored_apps_to_stats(mut stats: DailyStats, ignored_apps: &[String]) -> DailyStats {
    if ignored_apps.is_empty() {
        return stats;
    }

    let filtered_app_usage: Vec<_> = stats
        .app_usage
        .into_iter()
        .filter(|app| !matches_ignored_app(&app.app_name, ignored_apps))
        .collect();

    stats.total_duration = filtered_app_usage.iter().map(|app| app.duration).sum();
    stats.app_usage = filtered_app_usage;

    stats
        .browser_usage
        .retain(|browser| !matches_ignored_app(&browser.browser_name, ignored_apps));
    stats.browser_duration = stats
        .browser_usage
        .iter()
        .map(|browser| browser.duration)
        .sum();

    if stats.work_time_duration > stats.total_duration {
        stats.work_time_duration = stats.total_duration;
    }

    stats
}

fn matches_excluded_domain(target: &str, excluded_domains: &[String]) -> bool {
    let domain = PrivacyConfig::extract_domain(target);
    if domain.is_empty() {
        return false;
    }

    excluded_domains.iter().any(|excluded| {
        let excluded_domain = PrivacyConfig::extract_domain(excluded);
        !excluded_domain.is_empty()
            && (PrivacyConfig::domain_matches(&domain, &excluded_domain)
                || merged_domain_matches_excluded(&domain, &excluded_domain))
    })
}

fn merged_domain_matches_excluded(domain: &str, excluded_domain: &str) -> bool {
    if !is_merged_domain(domain) {
        return false;
    }

    let domain = domain.trim_end_matches('.').to_lowercase();
    let excluded_domain = excluded_domain.trim_end_matches('.').to_lowercase();
    let domain_labels: Vec<&str> = domain.split('.').collect();
    let excluded_labels: Vec<&str> = excluded_domain.split('.').collect();

    domain_labels.len() == 2
        && excluded_labels.len() == 2
        && domain_labels[0] == excluded_labels[0]
        && domain_labels[1].starts_with(excluded_labels[1])
        && domain_labels[1].len() > excluded_labels[1].len()
}

fn apply_excluded_domains_to_stats(
    mut stats: DailyStats,
    excluded_domains: &[String],
) -> DailyStats {
    if excluded_domains.is_empty() {
        return stats;
    }

    stats
        .url_usage
        .retain(|url| !matches_excluded_domain(&url.domain, excluded_domains));
    stats
        .domain_usage
        .retain(|domain| !matches_excluded_domain(&domain.domain, excluded_domains));

    for browser in &mut stats.browser_usage {
        browser
            .domains
            .retain(|domain| !matches_excluded_domain(&domain.domain, excluded_domains));
        browser.duration = browser.domains.iter().map(|domain| domain.duration).sum();
    }
    stats.browser_usage.retain(|browser| browser.duration > 0);
    stats.browser_usage.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.browser_name.cmp(&right.browser_name))
    });
    stats.browser_duration = stats
        .browser_usage
        .iter()
        .map(|browser| browser.duration)
        .sum();

    stats
}

fn load_daily_stats_for_overview(state: &AppState, date: &str) -> Result<DailyStats, AppError> {
    let segments = state.config.effective_work_segments();
    state
        .database
        .get_daily_stats_with_segments(date, &segments)
}

fn overview_week_bounds_for_date(anchor: chrono::NaiveDate) -> (String, String) {
    use chrono::Datelike;

    let monday = anchor - chrono::Duration::days(anchor.weekday().num_days_from_monday() as i64);
    (
        monday.format("%Y-%m-%d").to_string(),
        anchor.format("%Y-%m-%d").to_string(),
    )
}

#[derive(Default)]
struct DomainAggregate {
    duration: i64,
    semantic_votes: HashMap<String, i64>,
    urls: HashMap<String, i64>,
}

#[derive(Default)]
struct BrowserAggregate {
    duration: i64,
    executable_path: Option<String>,
    domains: HashMap<String, DomainAggregate>,
}

fn update_preferred_path(target: &mut Option<String>, candidate: Option<String>) {
    if target.is_none() {
        *target = candidate.filter(|value| !value.trim().is_empty());
    }
}

fn record_semantic_vote(
    votes: &mut HashMap<String, i64>,
    semantic_category: Option<String>,
    duration: i64,
) {
    if duration <= 0 {
        return;
    }

    if let Some(category) = semantic_category
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        *votes.entry(category).or_insert(0) += duration;
    }
}

fn resolve_primary_semantic(votes: HashMap<String, i64>) -> Option<String> {
    votes
        .into_iter()
        .max_by(
            |(left_label, left_duration), (right_label, right_duration)| {
                left_duration
                    .cmp(right_duration)
                    .then_with(|| right_label.cmp(left_label))
            },
        )
        .map(|(label, _)| label)
}

fn sort_url_details(items: &mut [UrlDetail]) {
    items.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| right.url.cmp(&left.url))
    });
}

fn sort_domain_usage(items: &mut [DomainUsage]) {
    items.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.domain.cmp(&right.domain))
    });

    for item in items {
        sort_url_details(&mut item.urls);
    }
}

fn build_domain_usage_from_aggregate(domain: String, aggregate: DomainAggregate) -> DomainUsage {
    let mut urls = aggregate
        .urls
        .into_iter()
        .map(|(url, duration)| UrlDetail { url, duration })
        .collect::<Vec<_>>();
    sort_url_details(&mut urls);

    DomainUsage {
        domain,
        duration: aggregate.duration,
        semantic_category: resolve_primary_semantic(aggregate.semantic_votes),
        urls,
    }
}

fn merge_domain_usage_maps(
    target: &mut HashMap<String, DomainAggregate>,
    domains: Vec<DomainUsage>,
) {
    for domain in domains {
        let domain_key = domain.domain.clone();
        let entry = target.entry(domain_key).or_default();
        entry.duration += domain.duration;
        record_semantic_vote(
            &mut entry.semantic_votes,
            domain.semantic_category.clone(),
            domain.duration,
        );

        for url in domain.urls {
            *entry.urls.entry(url.url).or_insert(0) += url.duration;
        }
    }
}

fn sum_daily_stats(days: Vec<DailyStats>) -> DailyStats {
    let mut total_duration = 0;
    let mut screenshot_count = 0;
    let mut browser_duration = 0;
    let mut work_time_duration = 0;

    let mut app_usage_map: HashMap<String, AppUsage> = HashMap::new();
    let mut category_usage_map: HashMap<String, i64> = HashMap::new();
    let mut url_usage_map: HashMap<String, UrlUsage> = HashMap::new();
    let mut domain_usage_map: HashMap<String, DomainAggregate> = HashMap::new();
    let mut browser_usage_map: HashMap<String, BrowserAggregate> = HashMap::new();
    let mut hourly_activity_distribution: Vec<HourlyActivityBucket> = (0..24)
        .map(|hour| HourlyActivityBucket { hour, duration: 0 })
        .collect();

    for day in days {
        total_duration += day.total_duration;
        screenshot_count += day.screenshot_count;
        browser_duration += day.browser_duration;
        work_time_duration += day.work_time_duration;

        for app in day.app_usage {
            let entry = app_usage_map
                .entry(app.app_name.clone())
                .or_insert(AppUsage {
                    app_name: app.app_name.clone(),
                    duration: 0,
                    count: 0,
                    executable_path: None,
                });
            entry.duration += app.duration;
            entry.count += app.count;
            update_preferred_path(&mut entry.executable_path, app.executable_path);
        }

        for category in day.category_usage {
            *category_usage_map.entry(category.category).or_insert(0) += category.duration;
        }

        for url in day.url_usage {
            let entry = url_usage_map.entry(url.url.clone()).or_insert(UrlUsage {
                url: url.url.clone(),
                domain: url.domain.clone(),
                duration: 0,
            });
            entry.duration += url.duration;
            if entry.domain.trim().is_empty() {
                entry.domain = url.domain;
            }
        }

        merge_domain_usage_maps(&mut domain_usage_map, day.domain_usage);

        for browser in day.browser_usage {
            let entry = browser_usage_map
                .entry(browser.browser_name.clone())
                .or_default();
            entry.duration += browser.duration;
            update_preferred_path(&mut entry.executable_path, browser.executable_path);
            merge_domain_usage_maps(&mut entry.domains, browser.domains);
        }

        for bucket in day.hourly_activity_distribution {
            if (0..24).contains(&bucket.hour) {
                hourly_activity_distribution[bucket.hour as usize].duration += bucket.duration;
            }
        }
    }

    let mut app_usage = app_usage_map.into_values().collect::<Vec<_>>();
    app_usage.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.app_name.cmp(&right.app_name))
    });

    let mut category_usage = category_usage_map
        .into_iter()
        .map(|(category, duration)| CategoryUsage { category, duration })
        .collect::<Vec<_>>();
    category_usage.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.category.cmp(&right.category))
    });

    let mut url_usage = url_usage_map.into_values().collect::<Vec<_>>();
    url_usage.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| right.url.cmp(&left.url))
    });

    let mut domain_usage = domain_usage_map
        .into_iter()
        .map(|(domain, aggregate)| build_domain_usage_from_aggregate(domain, aggregate))
        .collect::<Vec<_>>();
    sort_domain_usage(&mut domain_usage);

    let mut browser_usage = browser_usage_map
        .into_iter()
        .map(|(browser_name, aggregate)| {
            let mut domains = aggregate
                .domains
                .into_iter()
                .map(|(domain, domain_aggregate)| {
                    build_domain_usage_from_aggregate(domain, domain_aggregate)
                })
                .collect::<Vec<_>>();
            sort_domain_usage(&mut domains);

            BrowserUsage {
                browser_name,
                duration: aggregate.duration,
                executable_path: aggregate.executable_path,
                domains,
            }
        })
        .collect::<Vec<_>>();
    browser_usage.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.browser_name.cmp(&right.browser_name))
    });

    DailyStats {
        total_duration,
        screenshot_count,
        app_usage,
        category_usage,
        browser_duration,
        url_usage,
        domain_usage,
        browser_usage,
        work_time_duration,
        hourly_activity_distribution,
    }
}

fn resolve_overview_anchor_date(date: Option<&str>) -> Result<chrono::NaiveDate, AppError> {
    match date.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|e| AppError::Config(format!("解析概览日期失败: {e}"))),
        None => Ok(chrono::Local::now().date_naive()),
    }
}

fn resolve_overview_date_span(
    date: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<(chrono::NaiveDate, chrono::NaiveDate), AppError> {
    let fallback = resolve_overview_anchor_date(date)?;
    let start = date_from
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|e| AppError::Config(format!("解析概览开始日期失败: {e}")))
        })
        .transpose()?
        .unwrap_or(fallback);
    let end = date_to
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|e| AppError::Config(format!("解析概览结束日期失败: {e}")))
        })
        .transpose()?
        .unwrap_or(start);

    Ok(if start <= end {
        (start, end)
    } else {
        (end, start)
    })
}

/// 获取今日统计 —— 内部复用版（供 Tauri 命令与 localhost API 共用）
pub(crate) fn get_today_stats_inner(state: &Arc<Mutex<AppState>>) -> Result<DailyStats, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let stats = load_daily_stats_for_overview(&state, &today)?;
    let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);
    let stats = apply_ignored_apps_to_stats(stats, &ignored_apps);
    Ok(apply_excluded_domains_to_stats(stats, &excluded_domains))
}

/// 获取今日统计
#[tauri::command]
pub async fn get_today_stats(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DailyStats, AppError> {
    get_today_stats_inner(state.inner())
}

/// 获取概览统计 —— 内部复用版
pub(crate) fn get_overview_stats_inner(
    mode: String,
    date: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    state: &Arc<Mutex<AppState>>,
) -> Result<DailyStats, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let normalized_mode = mode.trim().to_lowercase();
    let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);

    let stats = match normalized_mode.as_str() {
        "date" => {
            let (start, end) = resolve_overview_date_span(
                date.as_deref(),
                date_from.as_deref(),
                date_to.as_deref(),
            )?;

            if start == end {
                let date_value = start.format("%Y-%m-%d").to_string();
                load_daily_stats_for_overview(&state, &date_value)?
            } else {
                let mut daily_stats = Vec::new();
                let mut current = start;
                while current <= end {
                    let current_date = current.format("%Y-%m-%d").to_string();
                    daily_stats.push(load_daily_stats_for_overview(&state, &current_date)?);
                    current = current
                        .succ_opt()
                        .ok_or_else(|| AppError::Config("计算概览日期范围失败".to_string()))?;
                }
                sum_daily_stats(daily_stats)
            }
        }
        "week" => {
            let anchor = resolve_overview_anchor_date(date.as_deref())?;
            let (date_from, date_to) = overview_week_bounds_for_date(anchor);
            let start = chrono::NaiveDate::parse_from_str(&date_from, "%Y-%m-%d")
                .map_err(|e| AppError::Config(format!("解析周概览开始日期失败: {e}")))?;
            let end = chrono::NaiveDate::parse_from_str(&date_to, "%Y-%m-%d")
                .map_err(|e| AppError::Config(format!("解析周概览结束日期失败: {e}")))?;

            let mut daily_stats = Vec::new();
            let mut current = start;
            while current <= end {
                let current_date = current.format("%Y-%m-%d").to_string();
                daily_stats.push(load_daily_stats_for_overview(&state, &current_date)?);
                current = current
                    .succ_opt()
                    .ok_or_else(|| AppError::Config("计算周概览日期范围失败".to_string()))?;
            }
            sum_daily_stats(daily_stats)
        }
        _ => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            load_daily_stats_for_overview(&state, &today)?
        }
    };

    let stats = apply_ignored_apps_to_stats(stats, &ignored_apps);
    Ok(apply_excluded_domains_to_stats(stats, &excluded_domains))
}

/// 获取概览统计（支持今日 / 指定日期 / 本周）
#[tauri::command]
pub async fn get_overview_stats(
    mode: String,
    date: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DailyStats, AppError> {
    get_overview_stats_inner(mode, date, date_from, date_to, state.inner())
}

/// 获取指定日期的统计 —— 内部复用版
pub(crate) fn get_daily_stats_inner(
    date: &str,
    state: &Arc<Mutex<AppState>>,
) -> Result<DailyStats, AppError> {
    let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let segments = s.config.effective_work_segments();
    let raw_stats = s.database.get_daily_stats_with_segments(date, &segments)?;
    let (ignored_apps, excluded_domains) = collect_privacy_filters(&s);
    Ok(apply_excluded_domains_to_stats(
        apply_ignored_apps_to_stats(raw_stats, &ignored_apps),
        &excluded_domains,
    ))
}

/// 获取指定日期的统计
#[tauri::command]
pub async fn get_daily_stats(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DailyStats, AppError> {
    get_daily_stats_inner(&date, state.inner())
}

/// 获取指定日期的时间线 —— 内部复用版（供 Tauri 命令与 localhost API 共用）
pub(crate) fn get_timeline_inner(
    date: String,
    limit: Option<u32>,
    offset: Option<u32>,
    state: &Arc<Mutex<AppState>>,
) -> Result<Vec<Activity>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = state.database.get_timeline(&date, limit, offset)?;
    let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);
    let filtered = filter_activities_by_privacy(activities, &ignored_apps, &excluded_domains);

    if !ignored_apps.is_empty() || !excluded_domains.is_empty() {
        log::info!(
            "隐私过滤: 需过滤应用 {:?}, 域名 {:?}，结果 {} 条",
            ignored_apps,
            excluded_domains,
            filtered.len()
        );
    }

    Ok(filtered)
}

/// 获取指定日期的时间线
#[tauri::command]
pub async fn get_timeline(
    date: String,
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Activity>, AppError> {
    get_timeline_inner(date, limit, offset, state.inner())
}

/// 获取每小时×应用的时长分布
#[tauri::command]
pub async fn get_hourly_app_breakdown(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<HourlyAppBucket>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.get_hourly_app_breakdown(&date)
}

fn collect_privacy_filters(state: &AppState) -> (Vec<String>, Vec<String>) {
    let ignored_apps = state.config.privacy.collect_ignored_app_names();
    let excluded_domains = state.config.privacy.collect_excluded_domains();
    (ignored_apps, excluded_domains)
}

fn filter_activities_by_privacy(
    activities: Vec<Activity>,
    ignored_apps: &[String],
    excluded_domains: &[String],
) -> Vec<Activity> {
    let no_app_filter = ignored_apps.is_empty();
    let no_domain_filter = excluded_domains.is_empty();

    if no_app_filter && no_domain_filter {
        return activities;
    }

    activities
        .into_iter()
        .filter(|activity| {
            let app_lower = activity.app_name.to_lowercase();
            if !no_app_filter
                && ignored_apps
                    .iter()
                    .any(|ignored| app_lower.contains(ignored) || ignored.contains(&app_lower))
            {
                return false;
            }

            if !no_domain_filter {
                if let Some(url) = &activity.browser_url {
                    let domain = PrivacyConfig::extract_domain(url);
                    if excluded_domains
                        .iter()
                        .any(|excluded| PrivacyConfig::domain_matches(&domain, excluded))
                    {
                        return false;
                    }
                }
            }

            true
        })
        .collect()
}

pub(crate) fn load_filtered_activities_in_range(
    state: &AppState,
    date_from: Option<&str>,
    date_to: Option<&str>,
    limit: usize,
) -> Result<Vec<Activity>, AppError> {
    let activities = state
        .database
        .get_activities_in_range(date_from, date_to, limit)?;
    let (ignored_apps, excluded_domains) = collect_privacy_filters(state);
    Ok(filter_activities_by_privacy(
        activities,
        &ignored_apps,
        &excluded_domains,
    ))
}

fn manual_followups_in_range(
    items: &[ManualFollowupItem],
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Vec<ManualFollowupItem> {
    items
        .iter()
        .filter(|item| item.status == "open")
        .filter(|item| {
            date_from
                .map(|start| item.date.as_str() >= start)
                .unwrap_or(true)
                && date_to.map(|end| item.date.as_str() <= end).unwrap_or(true)
        })
        .cloned()
        .collect()
}

fn merge_manual_followups_into_todos(
    mut extracted: TodoExtractionResult,
    manual_items: &[ManualFollowupItem],
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> TodoExtractionResult {
    let manual_items = manual_followups_in_range(manual_items, date_from, date_to);
    if manual_items.is_empty() {
        return extracted;
    }

    let mut seen = std::collections::HashSet::new();
    for item in &extracted.items {
        seen.insert(item.title.trim().to_lowercase());
    }

    for item in manual_items {
        let normalized = item.title.trim().to_lowercase();
        if normalized.is_empty() || !seen.insert(normalized) {
            continue;
        }

        extracted.items.push(crate::work_intelligence::TodoItem {
            title: item.title.clone(),
            date: item.date.clone(),
            source_title: item.source_title.clone(),
            source_app: item.source_app.clone(),
            confidence: 96,
            reason: "手动加入待跟进".to_string(),
        });
    }

    extracted.items.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| b.date.cmp(&a.date))
            .then_with(|| a.title.cmp(&b.title))
    });
    extracted.items.truncate(20);
    extracted.summary = format!(
        "共整理出 {} 条待跟进项（含手动加入）。",
        extracted.items.len()
    );
    extracted
}

/// 获取单个活动（用于刷新详情页，获取最新 OCR 结果）
#[tauri::command]
pub async fn get_activity(
    id: i64,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<Activity>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.get_activity_by_id(id)
}

/// 搜索工作记忆
#[tauri::command]
pub async fn search_memory(
    query: String,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<MemorySearchItem>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.search_memory(
        &query,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(20) as usize,
    )
}

/// 获取连续工作 session 聚合结果
#[tauri::command]
pub async fn get_work_sessions(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<WorkSession>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(build_work_sessions(&activities))
}

/// 基于 session 识别主要工作意图
#[tauri::command]
pub async fn recognize_work_intents(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<IntentAnalysisResult, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(analyze_intents(&activities))
}

/// 生成周报 / 阶段复盘 —— 内部复用版（供 Tauri 命令与 localhost API 共用）
pub(crate) fn generate_weekly_review_inner(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: &Arc<Mutex<AppState>>,
) -> Result<WeeklyReviewResult, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(build_weekly_review(
        &activities,
        date_from.as_deref(),
        date_to.as_deref(),
    ))
}

/// 生成周报 / 阶段复盘
#[tauri::command]
pub async fn generate_weekly_review(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<WeeklyReviewResult, AppError> {
    generate_weekly_review_inner(date_from, date_to, limit, state.inner())
}

/// 提取待跟进事项
#[tauri::command]
pub async fn extract_todo_items(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<TodoExtractionResult, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(merge_manual_followups_into_todos(
        extract_todos(&activities),
        &state.config.manual_followups,
        date_from.as_deref(),
        date_to.as_deref(),
    ))
}

/// 生成日报
pub(crate) async fn generate_report_inner(
    date: String,
    force: Option<bool>,
    locale: Option<String>,
    _app: &AppHandle,
    state: &Arc<Mutex<AppState>>,
) -> Result<String, AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let report_locale_code = report_locale.as_code();
    // 如果不是强制重新生成，先检查缓存
    if !force.unwrap_or(false) {
        let state_guard = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if let Ok(Some(cached)) = state_guard
            .database
            .get_report(&date, Some(report_locale_code))
        {
            log::info!("使用缓存日报: {date}");
            return Ok(cached.content);
        }
    }

    let (config, stats, activities, data_dir) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let segments = state.config.effective_work_segments();
        let raw_stats = state
            .database
            .get_daily_stats_with_segments(&date, &segments)?;
        // 生成日报时获取最多 2000 条记录
        let raw_activities = state.database.get_timeline(&date, Some(2000), None)?;
        let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);
        let stats = apply_excluded_domains_to_stats(
            apply_ignored_apps_to_stats(raw_stats, &ignored_apps),
            &excluded_domains,
        );
        let activities =
            filter_activities_by_privacy(raw_activities, &ignored_apps, &excluded_domains);
        (
            state.config.clone(),
            stats,
            activities,
            state.data_dir.clone(),
        )
    };

    // 创建分析器（使用 text_model 配置）
    let analyzer = crate::analysis::create_analyzer(
        config.ai_mode,
        config.text_model.provider,
        &config.text_model.endpoint,
        &config.text_model.model,
        config.text_model.api_key.as_deref(),
        &config.daily_report_custom_prompt,
        report_locale,
    );

    // 生成报告（spawn 隔离 panic，防止内部错误杀死整个 tokio 线程）
    // 外层加 300 秒总超时，防止 AI 调用卡死后前端永远等待
    let screenshots_dir = data_dir.clone();
    let date_gen = date.clone();
    let spawn_result = tokio::spawn(async move {
        analyzer
            .generate_report(
                &date_gen,
                &stats,
                &activities,
                &screenshots_dir,
                report_locale,
            )
            .await
    });

    let report_result = match tokio::time::timeout(
        std::time::Duration::from_secs(300),
        spawn_result,
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(work_review_core::error::AppError::Analysis(
            match report_locale {
                AppLocale::ZhCn => "日报生成过程中发生内部错误，请重试".to_string(),
                AppLocale::ZhTw => "日報生成過程中發生內部錯誤，請重試".to_string(),
                AppLocale::En => {
                    "Internal error during report generation, please retry".to_string()
                }
            },
        )),
        Err(_) => Err(work_review_core::error::AppError::Analysis(
            match report_locale {
                AppLocale::ZhCn => "日报生成超时，请稍后重试".to_string(),
                AppLocale::ZhTw => "日報生成逾時，請稍後重試".to_string(),
                AppLocale::En => "Report generation timed out, please try again later".to_string(),
            },
        )),
    };

    let generated_report = report_result?;
    let report = generated_report.content.clone();
    let (saved_ai_mode, saved_model_name) = resolve_saved_report_metadata(
        &config.ai_mode,
        &config.text_model.model,
        generated_report.used_ai,
    );

    // 保存报告
    {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let daily_report = DailyReport {
            date: date.clone(),
            locale: report_locale_code.to_string(),
            content: report.clone(),
            ai_mode: saved_ai_mode,
            model_name: saved_model_name,
            fallback_reason: generated_report.fallback_reason.clone(),
            created_at: chrono::Utc::now().timestamp(),
        };
        state.database.save_report(&daily_report)?;
    }

    if config.daily_report_auto_export {
        if let Some(export_dir) = config.daily_report_export_dir.as_deref() {
            export_daily_report_markdown(Path::new(export_dir), &date, &report)?;
        }
    }

    Ok(report)
}

struct ReportGenerationGuard {
    state: Arc<Mutex<AppState>>,
}

impl Drop for ReportGenerationGuard {
    fn drop(&mut self) {
        if let Ok(mut s) = self.state.lock() {
            s.generating_report = false;
        }
    }
}

#[tauri::command]
pub async fn generate_report(
    date: String,
    force: Option<bool>,
    locale: Option<String>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    {
        let mut s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if s.generating_report {
            return Err(AppError::Unknown("日报正在生成中，请稍候".to_string()));
        }
        s.generating_report = true;
    }
    let _guard = ReportGenerationGuard { state: state.inner().clone() };
    generate_report_inner(date, force, locale, &app, state.inner()).await
}

/// 获取已保存的日报
pub(crate) fn get_saved_report_inner(
    date: String,
    locale: Option<String>,
    state: &Arc<Mutex<AppState>>,
) -> Result<Option<DailyReport>, AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let saved = state
        .database
        .get_report(&date, Some(report_locale.as_code()))?;
    let Some(mut report) = saved else {
        return Ok(None);
    };

    // 用最新的 stats 重新渲染统计区块，解决 issue #80：保存的 markdown 里固化的时长
    // 数字会随着工作日继续推进而变得陈旧。老报告若没有占位符标记则原样返回。
    let segments = state.config.effective_work_segments();
    if let Ok(raw_stats) = state.database.get_daily_stats_with_segments(&date, &segments) {
        let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);
        let live_stats = apply_excluded_domains_to_stats(
            apply_ignored_apps_to_stats(raw_stats, &ignored_apps),
            &excluded_domains,
        );
        report.content = crate::analysis::report_blocks::render_report_with_live_stats(
            &report.content,
            &live_stats,
            report_locale,
        );
    }

    Ok(Some(report))
}

#[tauri::command]
pub async fn get_saved_report(
    date: String,
    locale: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<DailyReport>, AppError> {
    get_saved_report_inner(date, locale, state.inner())
}

/// 更新已保存日报的内容（用于结构化编辑）
#[tauri::command]
pub async fn update_report_content(
    date: String,
    locale: Option<String>,
    content: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let locale_code = report_locale.as_code();
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let existing = state
        .database
        .get_report(&date, Some(locale_code))?
        .ok_or_else(|| AppError::Database(rusqlite::Error::InvalidParameterName("报告不存在".to_string())))?;
    let updated = DailyReport {
        content,
        ..existing
    };
    state.database.save_report(&updated)?;
    Ok(())
}

pub(crate) fn export_report_markdown_inner(
    date: String,
    content: Option<String>,
    export_dir: Option<String>,
    state: &Arc<Mutex<AppState>>,
) -> Result<String, AppError> {
    let (export_dir, saved_content) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let requested_export_dir = export_dir
            .as_deref()
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(|dir| dir.to_string());
        let configured_export_dir = state
            .config
            .daily_report_export_dir
            .as_deref()
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(|dir| dir.to_string());
        let export_dir = requested_export_dir
            .or(configured_export_dir)
            .ok_or_else(|| {
                AppError::Config(
                    "请先选择导出目录，或在设置中配置日报 Markdown 导出目录".to_string(),
                )
            })?;
        let saved_content = if let Some(content) = content {
            content
        } else {
            state
                .database
                .get_report(&date, Some("zh-CN"))?
                .ok_or_else(|| AppError::Config("未找到可导出的日报".to_string()))?
                .content
        };
        (export_dir, saved_content)
    };

    let export_dir_path = Path::new(&export_dir);
    export_daily_report_markdown(export_dir_path, &date, &saved_content)?;
    Ok(build_daily_report_export_path(export_dir_path, &date)
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub async fn export_report_markdown(
    date: String,
    content: Option<String>,
    export_dir: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    export_report_markdown_inner(date, content, export_dir, state.inner())
}

/// 获取配置
#[tauri::command]
pub async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> Result<AppConfig, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(state.config.clone())
}

pub(crate) fn persist_app_config(
    mut config: AppConfig,
    app: AppHandle,
    state: &Arc<Mutex<AppState>>,
) -> Result<(), AppError> {
    config.normalize();
    let (previous_hide_dock_icon, previous_lightweight_mode) = {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let previous_config = state.config.clone();

        // 更新配置
        state.config = config.clone();
        state.storage_manager.update_config(config.storage.clone());
        state.screenshot_service.update_config(&config.storage);

        // 保存到文件
        let config_path = state.config_path.clone();
        config.save(&config_path)?;

        // 更新隐私过滤器
        state.privacy_filter.update_config(&config.privacy);
        (previous_config.hide_dock_icon, previous_config.lightweight_mode)
    };

    let dock_visibility_changed = previous_hide_dock_icon != config.hide_dock_icon
        || previous_lightweight_mode != config.lightweight_mode;

    if dock_visibility_changed {
        crate::sync_effective_dock_visibility(&app);
    }

    crate::emit_config_changed(&app, &config);

    log::info!("配置已保存");
    Ok(())
}

/// 保存配置
#[tauri::command]
pub async fn save_config(
    config: AppConfig,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    persist_app_config(config, app, state.inner())
}

/// 开始录制
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_recording = true;
    state.is_paused = false;
    log::info!("开始录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 停止录制
#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_recording = false;
    state.is_paused = false;
    log::info!("停止录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 暂停录制
#[tauri::command]
pub async fn pause_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_paused = true;
    log::info!("暂停录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 恢复录制
#[tauri::command]
pub async fn resume_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_recording = true;
    state.is_paused = false;
    log::info!("恢复录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 获取录制状态
#[tauri::command]
pub async fn get_recording_state(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(bool, bool), AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok((state.is_recording, state.is_paused))
}

/// 显示主窗口
#[tauri::command]
pub async fn show_main_window(
    app: AppHandle,
    source_window_label: Option<String>,
) -> Result<(), AppError> {
    crate::reveal_main_window(&app, source_window_label.as_deref())
}

#[tauri::command]
pub async fn get_manual_followups(
    date_from: Option<String>,
    date_to: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ManualFollowupItem>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(manual_followups_in_range(
        &state.config.manual_followups,
        date_from.as_deref(),
        date_to.as_deref(),
    ))
}

#[tauri::command]
pub async fn add_manual_followup(
    input: ManualFollowupInput,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ManualFollowupItem, AppError> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::Config("待跟进内容不能为空".to_string()));
    }

    let date = input.date.trim().to_string();
    if date.is_empty() {
        return Err(AppError::Config("待跟进日期不能为空".to_string()));
    }

    let source_app = input
        .source_app
        .unwrap_or_default()
        .trim()
        .to_string();
    let source_title = input
        .source_title
        .unwrap_or_default()
        .trim()
        .to_string();
    let note = input.note.unwrap_or_default().trim().to_string();
    let project_key = input
        .project_key
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            let fallback = if !source_app.is_empty() {
                source_app.as_str()
            } else {
                title.as_str()
            };
            fallback
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
                .collect::<String>()
        })
        .trim()
        .to_string();

    let item = ManualFollowupItem {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        note,
        date,
        source_app,
        source_title,
        project_key,
        created_at: chrono::Local::now().timestamp(),
        status: "open".to_string(),
    };

    let mut config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.config.clone()
    };
    config.manual_followups.push(item.clone());
    config.normalize();
    persist_app_config(config, app, state.inner())?;

    Ok(item)
}

#[tauri::command]
pub async fn update_manual_followup_status(
    id: String,
    status: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(AppError::Config("待跟进 ID 不能为空".to_string()));
    }
    let next_status = match status.trim() {
        "done" => "done",
        _ => "open",
    };

    let mut config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.config.clone()
    };

    if let Some(item) = config.manual_followups.iter_mut().find(|item| item.id == id) {
        item.status = next_status.to_string();
    } else {
        return Err(AppError::Config("待跟进不存在".to_string()));
    }

    config.normalize();
    persist_app_config(config, app, state.inner())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_manual_followup(
    id: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(AppError::Config("待跟进 ID 不能为空".to_string()));
    }

    let mut config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.config.clone()
    };
    let before_len = config.manual_followups.len();
    config.manual_followups.retain(|item| item.id != id);
    if config.manual_followups.len() == before_len {
        return Err(AppError::Config("待跟进不存在".to_string()));
    }

    config.normalize();
    persist_app_config(config, app, state.inner())?;
    Ok(())
}

fn flush_current_activity_for_intent(state: &Arc<Mutex<AppState>>, now_ts: i64) {
    let active_window = match crate::monitor::get_active_window() {
        Ok(window) => window,
        Err(error) => {
            log::warn!("目的备注保存前读取当前窗口失败: {error}");
            return;
        }
    };

    let mut state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
    let classification = crate::resolve_activity_classification(
        &state_guard.config,
        &active_window.app_name,
        &active_window.window_title,
        active_window.browser_url.as_deref(),
    );
    let latest = if let Some(url) = active_window
        .browser_url
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        state_guard.database.get_latest_activity_by_url(url).ok().flatten()
    } else {
        state_guard
            .database
            .get_latest_activity_by_app_title(&active_window.app_name, &active_window.window_title)
            .ok()
            .flatten()
    };

    if let Some(activity) = latest {
        let delta = now_ts.saturating_sub(activity.timestamp);
        if delta > 0 && delta <= 6 * 60 * 60 {
            if let Some(id) = activity.id {
                let _ = state_guard.database.merge_activity(
                    id,
                    delta,
                    None,
                    &activity.screenshot_path,
                    now_ts,
                );
            }
            return;
        }
    }

    let activity = Activity {
        id: None,
        timestamp: now_ts,
        app_name: active_window.app_name,
        window_title: active_window.window_title,
        screenshot_path: String::new(),
        ocr_text: None,
        category: classification.base_category,
        duration: 1,
        browser_url: active_window.browser_url,
        executable_path: active_window.executable_path,
        semantic_category: Some(classification.semantic_category),
        semantic_confidence: Some(i32::from(classification.confidence)),
        ..Activity::default()
    };
    let _ = state_guard.database.insert_activity(&activity);
}

#[tauri::command]
pub async fn save_intent_note_interval(
    input: IntentNoteInput,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let purpose = input.purpose.trim().to_string();
    if purpose.is_empty() {
        return Err(AppError::Config("目的不能为空".to_string()));
    }

    let now_ts = chrono::Local::now().timestamp();
    let purpose_start = input.start_timestamp;
    let purpose_end = input.end_timestamp.unwrap_or(now_ts).min(now_ts);
    if purpose_end <= purpose_start {
        return Err(AppError::Config("目的时间区间无效".to_string()));
    }

    flush_current_activity_for_intent(state.inner(), purpose_end);

    let annotated = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.database.apply_intent_to_interval(
            purpose_start,
            purpose_end,
            &purpose,
            input.note.as_deref(),
            now_ts,
        )?
    };

    Ok(serde_json::json!({
        "ok": true,
        "annotatedSegments": annotated,
        "startTimestamp": purpose_start,
        "endTimestamp": purpose_end,
        "completedAt": now_ts,
    }))
}

/// 获取数据目录
#[tauri::command]
pub async fn get_data_dir(state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(path_for_display(&state.data_dir))
}

/// 获取默认数据目录
#[tauri::command]
pub async fn get_default_data_dir() -> Result<String, AppError> {
    Ok(path_for_display(&crate::default_data_dir()))
}

#[tauri::command]
pub async fn get_runtime_platform() -> Result<String, AppError> {
    Ok(std::env::consts::OS.to_string())
}

#[tauri::command]
pub async fn get_linux_session_support() -> Result<LinuxSessionSupportInfo, AppError> {
    #[cfg(target_os = "linux")]
    {
        let session = current_linux_desktop_session();
        let desktop_environment = current_linux_desktop_environment();
        let active_window_provider =
            crate::monitor::current_linux_active_window_provider(session, desktop_environment);
        let screenshot_support = crate::screenshot::current_linux_screenshot_support();
        let active_window_supported = active_window_provider.is_some();
        let browser_url_support_level = if active_window_supported {
            "mixed"
        } else {
            "limited"
        };

        return Ok(LinuxSessionSupportInfo {
            platform: "linux".to_string(),
            session_type: session.as_str().to_string(),
            desktop_environment: desktop_environment.as_str().to_string(),
            active_window_provider: active_window_provider.unwrap_or("none").to_string(),
            active_window_supported,
            screenshot_supported: screenshot_support.supported,
            browser_url_support_level: browser_url_support_level.to_string(),
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(LinuxSessionSupportInfo {
            platform: std::env::consts::OS.to_string(),
            session_type: "not_applicable".to_string(),
            desktop_environment: "not_applicable".to_string(),
            active_window_provider: "not_applicable".to_string(),
            active_window_supported: false,
            screenshot_supported: false,
            browser_url_support_level: "not_applicable".to_string(),
        })
    }
}

fn path_for_display(path: &Path) -> String {
    let raw = path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        raw.strip_prefix(r"\\?\")
            .or_else(|| raw.strip_prefix(r"\??\"))
            .unwrap_or(&raw)
            .to_string()
    }

    #[cfg(not(target_os = "windows"))]
    {
        raw
    }
}

fn is_ignorable_dir_entry(name: &str) -> bool {
    name.starts_with('.') || name == "Thumbs.db"
}

fn is_managed_dir_entry(name: &str) -> bool {
    MANAGED_DATA_ENTRIES.contains(&name)
}

fn is_cleanup_managed_dir_entry(name: &str) -> bool {
    MANAGED_DATA_ENTRIES.contains(&name) || LIVE_DATABASE_FILES.contains(&name)
}

fn to_absolute_path(path: &Path) -> Result<PathBuf, AppError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn ensure_target_dir_ready(target_dir: &Path) -> Result<bool, AppError> {
    std::fs::create_dir_all(target_dir)?;

    let mut has_existing_app_data = false;

    for entry in std::fs::read_dir(target_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if is_ignorable_dir_entry(&name) {
            continue;
        }

        if !is_managed_dir_entry(&name) {
            return Err(AppError::Config(format!(
                "目标目录包含非 Work Review 数据（{}），为避免误覆盖，请选择空目录或旧的数据目录",
                name
            )));
        }

        has_existing_app_data = true;
    }

    if !has_existing_app_data {
        return Ok(false);
    }

    // 目标目录若已存在旧版应用数据，先清空受管条目，再完整覆盖为当前数据。
    for entry_name in MANAGED_DATA_ENTRIES {
        let path = target_dir.join(entry_name);
        if !path.exists() {
            continue;
        }

        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }

    Ok(true)
}

fn copy_managed_data_without_live_db(
    source_dir: &Path,
    target_dir: &Path,
) -> Result<u64, AppError> {
    let mut copied_files = 0u64;

    for entry_name in MANAGED_DATA_ENTRIES {
        if LIVE_DATABASE_FILES.contains(entry_name) {
            continue;
        }

        let source_path = source_dir.join(entry_name);
        if !source_path.exists() {
            continue;
        }

        let target_path = target_dir.join(entry_name);
        if source_path.is_dir() {
            copied_files += crate::copy_dir_contents(&source_path, &target_path, true)?;
        } else {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source_path, &target_path)?;
            copied_files += 1;
        }
    }

    Ok(copied_files)
}

fn remove_app_managed_entries(target_dir: &Path) -> Result<(u64, Vec<String>), AppError> {
    let mut removed_entries = 0u64;
    let mut preserved_entries = Vec::new();

    if !target_dir.exists() {
        return Ok((0, preserved_entries));
    }

    for entry in std::fs::read_dir(target_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if is_ignorable_dir_entry(&name) {
            continue;
        }

        if is_cleanup_managed_dir_entry(&name) {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
            removed_entries += 1;
            continue;
        }

        preserved_entries.push(name);
    }

    if preserved_entries.is_empty() {
        let mut remaining_entries = std::fs::read_dir(target_dir)?;
        if remaining_entries.next().is_none() {
            let _ = std::fs::remove_dir(target_dir);
        }
    }

    Ok((removed_entries, preserved_entries))
}

/// 切换数据目录，并迁移当前数据
#[tauri::command]
pub async fn change_data_dir(
    target_dir: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let requested_dir = target_dir.trim();
    if requested_dir.is_empty() {
        return Err(AppError::Config("目标目录不能为空".to_string()));
    }

    let requested_path = to_absolute_path(Path::new(requested_dir))?;
    let current_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state
            .data_dir
            .canonicalize()
            .unwrap_or_else(|_| state.data_dir.clone())
    };

    if requested_path == current_dir {
        return Ok(serde_json::json!({
            "dataDir": current_dir.to_string_lossy().to_string(),
            "copiedFiles": 0,
            "message": "数据目录未变化",
        }));
    }

    if requested_path.starts_with(&current_dir) || current_dir.starts_with(&requested_path) {
        return Err(AppError::Config(
            "新旧数据目录不能互为父子目录，请选择独立目录".to_string(),
        ));
    }

    let target_dir = {
        std::fs::create_dir_all(&requested_path)?;
        requested_path
            .canonicalize()
            .unwrap_or_else(|_| requested_path.clone())
    };

    // 先清空目标目录中已有的受管条目（必须在 backup_to 之前，否则会删掉刚备份的数据库）
    let replaced_existing_data = ensure_target_dir_ready(&target_dir)?;

    // 复制截图等文件（在锁外执行，不阻塞截图循环）
    let copied_files = copy_managed_data_without_live_db(&current_dir, &target_dir)?;

    // 短暂获取锁，做安全 SQLite 备份，然后立即释放
    let config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        // SQLite 备份必须在持锁状态下执行（backup_to 内部做 WAL checkpoint + VACUUM INTO）
        state
            .database
            .backup_to(&target_dir.join("workreview.db"))?;
        state.config.clone()
    };

    let config_path = target_dir.join("config.json");
    config.save(&config_path)?;
    crate::save_data_dir_preference(&target_dir)?;

    // 重新获取锁，仅做轻量状态更新
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database = Database::new(&target_dir.join("workreview.db"))?;
    if let Err(e) = state.database.rebuild_fts_index() {
        log::warn!("迁移后 FTS 索引重建失败: {e}");
    }
    state.privacy_filter = PrivacyFilter::from_config(&config.privacy);
    state.screenshot_service = ScreenshotService::new(&target_dir, &config.storage);
    state.storage_manager = StorageManager::new(&target_dir, config.storage.clone());
    state.data_dir = target_dir.clone();
    state.config_path = config_path;

    log::info!("数据目录已切换到: {:?}", target_dir);
    drop(state);
    crate::emit_recording_state_changed(&app);

    Ok(serde_json::json!({
        "dataDir": target_dir.to_string_lossy().to_string(),
        "oldDataDir": current_dir.to_string_lossy().to_string(),
        "copiedFiles": copied_files,
        "replacedExistingData": replaced_existing_data,
        "message": format!(
            "数据目录已更新，已迁移 {} 个文件{}",
            copied_files,
            if replaced_existing_data { "，并覆盖旧目录中的 Work Review 数据" } else { "" }
        ),
    }))
}

#[tauri::command]
pub async fn cleanup_old_data_dir(
    target_dir: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let requested_dir = target_dir.trim();
    if requested_dir.is_empty() {
        return Err(AppError::Config("旧目录不能为空".to_string()));
    }

    let requested_path = to_absolute_path(Path::new(requested_dir))?;
    if !requested_path.exists() {
        return Ok(serde_json::json!({
            "removedEntries": 0,
            "preservedEntries": [],
            "message": "旧目录不存在，无需清理",
        }));
    }

    let current_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state
            .data_dir
            .canonicalize()
            .unwrap_or_else(|_| state.data_dir.clone())
    };

    let cleanup_dir = requested_path
        .canonicalize()
        .unwrap_or_else(|_| requested_path.clone());

    if cleanup_dir == current_dir {
        return Err(AppError::Config(
            "不能清理当前正在使用的数据目录".to_string(),
        ));
    }

    if cleanup_dir.starts_with(&current_dir) || current_dir.starts_with(&cleanup_dir) {
        return Err(AppError::Config(
            "为避免误删，当前数据目录与待清理目录不能互为父子目录".to_string(),
        ));
    }

    let (removed_entries, preserved_entries) = remove_app_managed_entries(&cleanup_dir)?;
    let message = if preserved_entries.is_empty() {
        if cleanup_dir.exists() {
            format!("已清理旧目录中的 {} 项 Work Review 数据", removed_entries)
        } else {
            format!(
                "已清理旧目录中的 {} 项 Work Review 数据，并移除空目录",
                removed_entries
            )
        }
    } else {
        format!(
            "已清理旧目录中的 {} 项 Work Review 数据，保留其他文件：{}",
            removed_entries,
            preserved_entries.join("、")
        )
    };

    Ok(serde_json::json!({
        "removedEntries": removed_entries,
        "preservedEntries": preserved_entries,
        "message": message,
    }))
}

/// 在系统文件管理器中打开数据目录
/// plugin-shell 的 open 对本地路径在部分平台不可靠，改用系统命令直接打开
#[tauri::command]
pub async fn open_data_dir(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };

    // 目录不存在时先创建，避免打开失败
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::Unknown(format!("创建数据目录失败: {e}")))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开数据目录失败: {e}")))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开数据目录失败: {e}")))?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开数据目录失败: {e}")))?;
    }

    Ok(())
}

/// 获取截图缩略图
#[tauri::command]
pub async fn get_screenshot_thumbnail(
    path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let full_path = state.data_dir.join(&path);
    state
        .screenshot_service
        .generate_thumbnail_base64(&full_path, 400)
}

/// 获取高分辨率截图（用于详情弹窗，1200px）
#[tauri::command]
pub async fn get_screenshot_full(
    path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let full_path = state.data_dir.join(&path);
    state
        .screenshot_service
        .generate_full_image_base64(&full_path)
}

/// 手动执行一次截屏
#[tauri::command]
pub async fn take_screenshot(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Activity, AppError> {
    let (
        screenshot_result,
        app_name,
        window_title,
        browser_url,
        category,
        semantic_category,
        semantic_confidence,
        relative_path,
        executable_path,
    ) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;

        // 获取当前活动窗口
        let active_window = crate::monitor::get_active_window().ok();

        #[cfg(target_os = "linux")]
        let active_window = if active_window.is_none()
            && !matches!(
                current_linux_desktop_session(),
                LinuxDesktopSession::Wayland
            ) {
            return Err(AppError::Unknown("获取当前活动窗口失败".to_string()));
        } else {
            active_window
        };

        #[cfg(not(target_os = "linux"))]
        let active_window = match active_window {
            Some(active_window) => Some(active_window),
            None => return Err(AppError::Unknown("获取当前活动窗口失败".to_string())),
        };

        // 检查隐私过滤
        if let Some(active_window) = active_window.as_ref() {
            if state.privacy_filter.check_privacy_full(
                &active_window.app_name,
                &active_window.window_title,
                active_window.browser_url.as_deref(),
            ) == crate::privacy::PrivacyAction::Skip
            {
                return Err(AppError::Privacy("当前窗口被隐私规则过滤".to_string()));
            }
        }

        // 执行截屏
        let result = state
            .screenshot_service
            .capture_for_window(active_window.as_ref())?;
        let relative_path = state.screenshot_service.get_relative_path(&result.path);
        let app_name = active_window
            .as_ref()
            .map(|window| window.app_name.clone())
            .unwrap_or_else(|| "Wayland Session".to_string());
        let window_title = active_window
            .as_ref()
            .map(|window| window.window_title.clone())
            .unwrap_or_else(|| "Wayland screenshot".to_string());
        let browser_url = active_window
            .as_ref()
            .and_then(|window| window.browser_url.clone());
        let executable_path = active_window
            .as_ref()
            .and_then(|window| window.executable_path.clone());
        let classification = crate::resolve_activity_classification(
            &state.config,
            &app_name,
            &window_title,
            browser_url.as_deref(),
        );

        (
            result,
            app_name,
            window_title,
            browser_url,
            classification.base_category,
            classification.semantic_category,
            classification.confidence,
            relative_path,
            executable_path,
        )
    };

    // 创建活动记录
    let activity = Activity {
        id: None,
        timestamp: screenshot_result.timestamp,
        app_name,
        window_title,
        screenshot_path: relative_path,
        ocr_text: None,
        category,
        duration: 30,
        browser_url,
        executable_path,
        semantic_category: Some(semantic_category),
        semantic_confidence: Some(i32::from(semantic_confidence)),
        ..Activity::default()
    };

    // 保存到数据库
    let insert_result = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.database.insert_activity(&activity)
    };

    if let Err(error) = insert_result {
        let _ = std::fs::remove_file(&screenshot_result.path);
        if let Some(temp_path) = screenshot_result
            .ocr_source_path
            .as_ref()
            .filter(|path| *path != &screenshot_result.path)
        {
            let _ = std::fs::remove_file(temp_path);
        }
        return Err(error);
    }

    if let Some(temp_path) = screenshot_result
        .ocr_source_path
        .as_ref()
        .filter(|path| *path != &screenshot_result.path)
    {
        let _ = std::fs::remove_file(temp_path);
    }

    Ok(activity)
}

/// 获取历史应用列表 —— 内部复用版
pub(crate) fn get_recent_apps_inner(state: &Arc<Mutex<AppState>>) -> Result<Vec<String>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.get_recent_apps(50)
}

/// 获取历史应用列表（从数据库）
#[tauri::command]
pub async fn get_recent_apps(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, AppError> {
    get_recent_apps_inner(state.inner())
}

/// 应用分类概览 —— 内部复用版
pub(crate) fn get_app_category_overview_inner(state: &Arc<Mutex<AppState>>) -> Result<Vec<AppCategoryOverviewItem>, AppError> {
    let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let overview = s.database.get_app_category_overview()?;

    Ok(overview
        .into_iter()
        .map(|item| {
            let override_category = crate::monitor::find_category_override(
                &s.config.app_category_rules,
                &item.app_name,
                &s.config.custom_categories,
            );
            let is_overridden = override_category.is_some();
            AppCategoryOverviewItem {
                app_name: item.app_name,
                category: override_category.unwrap_or(item.category),
                total_duration: item.total_duration,
                is_overridden,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn get_app_category_overview(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<AppCategoryOverviewItem>, AppError> {
    get_app_category_overview_inner(state.inner())
}

fn upsert_app_category_rule(config: &mut AppConfig, app_name: &str, category: &str) {
    let normalized_app_name = crate::monitor::normalize_display_app_name(app_name);
    let custom_keys: Vec<String> = config
        .custom_categories
        .iter()
        .map(|c| c.key.clone())
        .collect();
    let normalized_category = crate::config::normalize_category_key_private(category, &custom_keys);
    let match_key = normalized_app_name.to_lowercase();

    if let Some(rule) = config.app_category_rules.iter_mut().find(|rule| {
        crate::monitor::normalize_display_app_name(&rule.app_name).to_lowercase() == match_key
    }) {
        rule.app_name = normalized_app_name;
        rule.category = normalized_category;
        return;
    }

    config.app_category_rules.push(AppCategoryRule {
        app_name: normalized_app_name,
        category: normalized_category,
    });
}

fn reclassify_app_history_in_state(
    state: &AppState,
    app_name: &str,
    category: &str,
) -> Result<usize, AppError> {
    let custom_keys: Vec<String> = state
        .config
        .custom_categories
        .iter()
        .map(|c| c.key.clone())
        .collect();
    let target_category = crate::config::normalize_category_key_private(category, &custom_keys);
    let activities = state
        .database
        .get_activities_by_normalized_app_name(app_name)?;

    for activity in &activities {
        let classification = crate::activity_classifier::classify_activity_with_base_category(
            &activity.app_name,
            &activity.window_title,
            activity.browser_url.as_deref(),
            &target_category,
        );
        state.database.update_activity_classification(
            activity.id.expect("活动记录应包含主键"),
            &classification.base_category,
            Some(&classification.semantic_category),
            Some(i32::from(classification.confidence)),
        )?;
    }

    Ok(activities.len())
}

fn upsert_domain_semantic_rule(config: &mut AppConfig, domain: &str, semantic_category: &str) {
    let Some(normalized_domain) = crate::monitor::normalize_domain_rule(domain) else {
        return;
    };
    let normalized_semantic_category = semantic_category.trim().to_string();

    if let Some(rule) = config.website_semantic_rules.iter_mut().find(|rule| {
        crate::monitor::normalize_domain_rule(&rule.domain).as_deref()
            == Some(normalized_domain.as_str())
    }) {
        rule.domain = normalized_domain;
        rule.semantic_category = normalized_semantic_category;
        return;
    }

    config.website_semantic_rules.push(WebsiteSemanticRule {
        domain: normalized_domain,
        semantic_category: normalized_semantic_category,
    });
}

fn reclassify_domain_history_in_state(
    state: &AppState,
    domain: &str,
    semantic_category: &str,
) -> Result<usize, AppError> {
    let activities = state.database.get_activities_by_domain(domain)?;
    let semantic_category = semantic_category.trim();

    for activity in &activities {
        let next_base_category = crate::monitor::semantic_category_to_base_category(
            semantic_category,
            &activity.category,
        );
        state.database.update_activity_classification(
            activity.id.expect("活动记录应包含主键"),
            &next_base_category,
            Some(semantic_category),
            Some(100),
        )?;
    }

    Ok(activities.len())
}

#[tauri::command]
pub async fn set_app_category_rule(
    app_name: String,
    category: String,
    sync_history: bool,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let trimmed_app_name = app_name.trim();
    if trimmed_app_name.is_empty() {
        return Err(AppError::Unknown("应用名称不能为空".to_string()));
    }

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();
        upsert_app_category_rule(&mut next_config, trimmed_app_name, &category);
        next_config
    };

    persist_app_config(next_config, app, state.inner())?;

    if !sync_history {
        return Ok(0);
    }

    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    reclassify_app_history_in_state(&state, trimmed_app_name, &category)
}

#[tauri::command]
pub async fn reclassify_app_history(
    app_name: String,
    category: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    reclassify_app_history_in_state(&state, &app_name, &category)
}

/// 分类信息（前端展示用）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoryInfo {
    pub key: String,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub is_custom: bool,
}

/// 分类信息 —— 内部复用版
pub(crate) fn get_categories_inner(state: &Arc<Mutex<AppState>>) -> Result<Vec<CategoryInfo>, AppError> {
    let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let mut result = Vec::new();
    let builtins: Vec<(&str, &str, &str, &str)> = vec![
        ("development", "开发工具", "blue", "⚡"),
        ("browser", "浏览器", "green", "🌐"),
        ("communication", "通讯协作", "yellow", "💬"),
        ("office", "办公软件", "purple", "📝"),
        ("design", "设计工具", "pink", "🎨"),
        ("entertainment", "娱乐摸鱼", "red", "🎮"),
        ("other", "其他", "gray", "📁"),
    ];
    for (key, name, color, icon) in builtins {
        result.push(CategoryInfo {
            key: key.to_string(),
            name: name.to_string(),
            color: color.to_string(),
            icon: icon.to_string(),
            is_custom: false,
        });
    }
    for c in &s.config.custom_categories {
        result.push(CategoryInfo {
            key: c.key.clone(),
            name: c.name.clone(),
            color: c.color.clone(),
            icon: c.icon.clone(),
            is_custom: true,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_categories(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<CategoryInfo>, AppError> {
    get_categories_inner(state.inner())
}

#[tauri::command]
pub async fn save_custom_category(
    key: String,
    name: String,
    color: String,
    icon: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let key = key.trim().to_lowercase();
    let name = name.trim().to_string();
    let color = color.trim().to_string();
    let icon = icon.trim().to_string();

    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AppError::Unknown(
            "分类标识只能包含小写字母、数字和连字符".to_string(),
        ));
    }
    if name.is_empty() {
        return Err(AppError::Unknown("分类名称不能为空".to_string()));
    }
    if !color.starts_with('#') || color.len() != 7 {
        return Err(AppError::Unknown("颜色格式无效，需为 #RRGGBB".to_string()));
    }
    // 不允许覆盖预设分类
    if matches!(
        key.as_str(),
        "development"
            | "browser"
            | "communication"
            | "office"
            | "design"
            | "entertainment"
            | "other"
    ) {
        return Err(AppError::Unknown("不能覆盖预设分类".to_string()));
    }

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();

        let custom = crate::config::CustomCategory {
            key: key.clone(),
            name: name.clone(),
            color: color.clone(),
            icon: icon.clone(),
        };

        if let Some(existing) = next_config
            .custom_categories
            .iter_mut()
            .find(|c| c.key == key)
        {
            *existing = custom;
        } else {
            next_config.custom_categories.push(custom);
        }

        next_config
    };

    persist_app_config(next_config, app, state.inner())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_custom_category(
    key: String,
    reassign_to: Option<String>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let key = key.trim().to_lowercase();
    let fallback = reassign_to.unwrap_or_else(|| "other".to_string());

    let affected = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        // 统计引用该分类的规则数
        state
            .config
            .app_category_rules
            .iter()
            .filter(|r| r.category == key)
            .count()
    };

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();

        // 删除自定义分类
        next_config.custom_categories.retain(|c| c.key != key);

        // 重定向引用该分类的规则
        for rule in &mut next_config.app_category_rules {
            if rule.category == key {
                rule.category = fallback.clone();
            }
        }

        next_config
    };

    persist_app_config(next_config, app, state.inner())?;
    Ok(affected)
}

/// 语义分类信息（前端展示用）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SemanticCategoryInfo {
    pub key: String,
    pub name: String,
    pub is_custom: bool,
}

/// 语义分类信息 —— 内部复用版
pub(crate) fn get_semantic_categories_inner(state: &Arc<Mutex<AppState>>) -> Result<Vec<SemanticCategoryInfo>, AppError> {
    let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let mut result = Vec::new();
    let builtins: Vec<(&str, &str)> = vec![
        ("编码开发", "编码开发"),
        ("内容撰写", "内容撰写"),
        ("资料阅读", "资料阅读"),
        ("资料调研", "资料调研"),
        ("任务规划", "任务规划"),
        ("设计创作", "设计创作"),
        ("AI 协作", "AI 协作"),
        ("即时聊天", "即时聊天"),
        ("会议沟通", "会议沟通"),
        ("视频内容", "视频内容"),
        ("音乐音频", "音乐音频"),
        ("休息娱乐", "休息娱乐"),
        ("未知活动", "未知活动"),
    ];
    for (key, name) in builtins {
        result.push(SemanticCategoryInfo {
            key: key.to_string(),
            name: name.to_string(),
            is_custom: false,
        });
    }
    for c in &s.config.custom_semantic_categories {
        result.push(SemanticCategoryInfo {
            key: c.key.clone(),
            name: c.name.clone(),
            is_custom: true,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_semantic_categories(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SemanticCategoryInfo>, AppError> {
    get_semantic_categories_inner(state.inner())
}

#[tauri::command]
pub async fn save_custom_semantic_category(
    key: String,
    name: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let key = key.trim().to_lowercase();
    let name = name.trim().to_string();

    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AppError::Unknown(
            "分类标识只能包含小写字母、数字和连字符".to_string(),
        ));
    }
    if name.is_empty() {
        return Err(AppError::Unknown("分类名称不能为空".to_string()));
    }
    // 不允许覆盖预设语义分类
    if crate::config::is_valid_builtin_semantic_category(&name) {
        return Err(AppError::Unknown(
            "不能使用与预设分类相同的名称".to_string(),
        ));
    }

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();

        let custom = CustomSemanticCategory {
            key: key.clone(),
            name: name.clone(),
        };

        if let Some(existing) = next_config
            .custom_semantic_categories
            .iter_mut()
            .find(|c| c.key == key)
        {
            *existing = custom;
        } else {
            next_config.custom_semantic_categories.push(custom);
        }

        next_config
    };

    persist_app_config(next_config, app, state.inner())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_custom_semantic_category(
    key: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let key = key.trim().to_lowercase();

    let affected = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        // 统计引用该分类的规则数
        state
            .config
            .website_semantic_rules
            .iter()
            .filter(|r| r.semantic_category == key)
            .count()
    };

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();

        // 删除自定义语义分类
        next_config
            .custom_semantic_categories
            .retain(|c| c.key != key);

        // 重定向引用该分类的规则到"未知活动"
        for rule in &mut next_config.website_semantic_rules {
            if rule.semantic_category == key {
                rule.semantic_category = "未知活动".to_string();
            }
        }

        next_config
    };

    persist_app_config(next_config, app, state.inner())?;
    Ok(affected)
}

#[tauri::command]
pub async fn set_domain_semantic_rule(
    domain: String,
    semantic_category: String,
    sync_history: bool,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let normalized_domain = crate::monitor::normalize_domain_rule(&domain)
        .ok_or_else(|| AppError::Unknown("域名不能为空".to_string()))?;
    let trimmed_semantic_category = semantic_category.trim();
    if trimmed_semantic_category.is_empty() {
        return Err(AppError::Unknown("语义分类不能为空".to_string()));
    }

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();
        upsert_domain_semantic_rule(
            &mut next_config,
            &normalized_domain,
            trimmed_semantic_category,
        );
        next_config
    };

    persist_app_config(next_config, app, state.inner())?;

    if !sync_history {
        return Ok(0);
    }

    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    reclassify_domain_history_in_state(&state, &normalized_domain, trimmed_semantic_category)
}

/// 获取当前运行的应用列表
#[tauri::command]
pub async fn get_running_apps() -> Result<Vec<String>, AppError> {
    get_running_apps_impl()
}

/// macOS 实现
#[cfg(target_os = "macos")]
fn get_running_apps_impl() -> Result<Vec<String>, AppError> {
    use std::process::Command;

    // 使用 AppleScript 获取运行中的应用
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to get name of every process whose background only is false"#
        ])
        .output()
        .map_err(|e| AppError::Unknown(format!("执行 AppleScript 失败: {e}")))?;

    if output.status.success() {
        let apps_str = String::from_utf8_lossy(&output.stdout);
        let mut apps: Vec<String> = apps_str
            .split(", ")
            .map(|s| crate::monitor::normalize_display_app_name(s))
            .filter(|s| !s.is_empty())
            .collect();
        apps.sort();
        apps.dedup();
        Ok(apps)
    } else {
        Err(AppError::Unknown("获取应用列表失败".to_string()))
    }
}

/// Windows 实现
#[cfg(target_os = "windows")]
fn get_running_apps_impl() -> Result<Vec<String>, AppError> {
    use std::collections::HashSet;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::tlhelp32::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut apps = HashSet::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() {
            return Ok(vec![]);
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                // 获取进程名
                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = OsString::from_wide(&entry.szExeFile[..name_len])
                    .to_string_lossy()
                    .to_string();

                // 排除系统进程
                let name_lower = name.to_lowercase();
                if !name_lower.ends_with(".exe") {
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                    continue;
                }

                // 排除常见系统进程
                let excluded = [
                    "svchost.exe",
                    "csrss.exe",
                    "wininit.exe",
                    "services.exe",
                    "lsass.exe",
                    "smss.exe",
                    "winlogon.exe",
                    "dwm.exe",
                    "fontdrvhost.exe",
                    "sihost.exe",
                    "taskhostw.exe",
                    "runtimebroker.exe",
                    "searchhost.exe",
                    "startmenuexperiencehost.exe",
                    "textinputhost.exe",
                    "ctfmon.exe",
                    "conhost.exe",
                ];

                if !excluded.contains(&name_lower.as_str()) {
                    // 移除 .exe 后缀
                    let display_name = crate::monitor::normalize_display_app_name(&name);
                    apps.insert(display_name);
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    let mut result: Vec<String> = apps.into_iter().collect();
    result.sort();
    Ok(result)
}

/// 其他平台
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_running_apps_impl() -> Result<Vec<String>, AppError> {
    Ok(vec![])
}

/// 获取存储统计信息 —— 内部复用版
pub(crate) fn get_storage_stats_inner(state: &Arc<Mutex<AppState>>) -> Result<serde_json::Value, AppError> {
    let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let stats = s
        .storage_manager
        .get_stats()
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    Ok(serde_json::json!({
        "total_files": stats.total_files,
        "total_size_mb": format!("{:.1}", stats.total_size_mb),
        "storage_limit_mb": stats.storage_limit_mb,
        "retention_days": stats.retention_days,
    }))
}

/// 获取存储统计信息
#[tauri::command]
pub async fn get_storage_stats(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    get_storage_stats_inner(state.inner())
}

/// 获取指定日期的小时摘要 —— 内部复用版
pub(crate) fn get_hourly_summaries_inner(
    date: &str,
    state: &Arc<Mutex<AppState>>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let app_state = state.clone();

    for hour in 0..24 {
        crate::generate_and_save_summary(&app_state, date, hour);
    }

    let s = app_state
        .lock()
        .map_err(|e| AppError::Unknown(e.to_string()))?;
    let summaries = s.database.get_hourly_summaries(date)?;

    Ok(summaries
        .iter()
        .map(|s| {
            serde_json::json!({
                "hour": s.hour,
                "summary": s.summary,
                "main_apps": s.main_apps,
                "activity_count": s.activity_count,
                "total_duration": s.total_duration,
            })
        })
        .collect())
}

/// 获取指定日期的小时摘要
#[tauri::command]
pub async fn get_hourly_summaries(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<serde_json::Value>, AppError> {
    get_hourly_summaries_inner(&date, state.inner())
}

/// 清理今天之前的所有活动记录
#[tauri::command]
pub async fn clear_old_activities(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };

    // 获取要保留的日期（今天和昨天）
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let yesterday = (now - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let mut deleted_screenshots = 0;

    // 删除旧截图目录（保留今天和昨天）
    let screenshots_dir = data_dir.join("screenshots");
    if screenshots_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&screenshots_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // 保留今天和昨天的目录
                    if name != today && name != yesterday && entry.path().is_dir() {
                        if let Ok(dir_entries) = std::fs::read_dir(entry.path()) {
                            for file_entry in dir_entries.flatten() {
                                if file_entry.path().is_file() {
                                    deleted_screenshots += 1;
                                }
                            }
                        }
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }

    // 同步清理数据库中对应的旧记录
    {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if let Err(e) = state.database.delete_activities_before_date(&today) {
            log::warn!("清理旧活动记录失败: {e}");
        }
    }

    Ok(serde_json::json!({
        "deleted_screenshots": deleted_screenshots,
        "kept_dates": [today, yesterday],
        "message": format!("已清理 {} 张旧截图和对应活动记录，保留今天和昨天的数据", deleted_screenshots)
    }))
}

/// 获取指定日期的 OCR 日志
#[tauri::command]
pub async fn get_ocr_log(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let ocr_logger = crate::ocr_logger::OcrLogger::new(&state.data_dir);
    ocr_logger.read_log(&date)
}

/// 检查屏幕锁定状态
#[tauri::command]
pub async fn is_screen_locked() -> Result<bool, AppError> {
    let monitor = crate::screen_lock::ScreenLockMonitor::new();
    Ok(monitor.is_locked())
}

/// 检查 macOS 系统权限状态（屏幕录制 + 辅助功能）
/// Windows 上始终返回全部已授权
#[tauri::command]
pub async fn check_permissions() -> Result<serde_json::Value, AppError> {
    let screen_capture = crate::screenshot::has_screen_capture_permission();
    let accessibility = crate::screenshot::has_accessibility_permission(false);

    #[cfg(target_os = "linux")]
    let screenshot_supported = crate::screenshot::current_linux_screenshot_support().supported;
    #[cfg(not(target_os = "linux"))]
    let screenshot_supported = screen_capture;

    let all_granted = if cfg!(target_os = "macos") {
        screen_capture && accessibility
    } else {
        screenshot_supported
    };

    Ok(serde_json::json!({
        "screen_capture": screen_capture,
        "accessibility": accessibility,
        "screenshot_supported": screenshot_supported,
        "all_granted": all_granted,
        "platform": std::env::consts::OS,
    }))
}

#[cfg(target_os = "macos")]
fn macos_permission_settings_url(permission: &str) -> Option<&'static str> {
    match permission {
        "screen_capture" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        }
        "accessibility" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        }
        _ => None,
    }
}

/// 打开系统权限设置页
#[tauri::command]
pub async fn open_permission_settings(permission: String) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        match permission.as_str() {
            "screen_capture" => crate::screenshot::request_screen_capture_permission(),
            "accessibility" => {
                crate::screenshot::has_accessibility_permission(true);
            }
            _ => {}
        }

        let target = macos_permission_settings_url(&permission)
            .ok_or_else(|| AppError::Unknown(format!("不支持的权限类型: {}", permission)))?;

        std::process::Command::new("open")
            .arg(target)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开系统权限设置失败: {e}")))?;

        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = permission;
        Err(AppError::Unknown(
            "当前平台暂不支持直接跳转系统权限设置".to_string(),
        ))
    }
}

/// 检查是否在工作时间内
#[tauri::command]
pub async fn is_work_time(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let segments = state.config.effective_work_segments();
    Ok(crate::screen_lock::ScreenLockMonitor::is_work_time_in_segments(&segments))
}

/// 检查 PaddleOCR 是否可用
#[tauri::command]
pub async fn check_ocr_available() -> Result<serde_json::Value, AppError> {
    let paddle_available = crate::ocr::OcrService::check_paddle_available();

    Ok(serde_json::json!({
        "paddle_ocr_available": paddle_available,
        "install_command": crate::ocr::OcrService::get_paddle_install_command(),
        "platform": std::env::consts::OS,
    }))
}

/// 执行 OCR 识别
#[tauri::command]
pub async fn run_ocr(
    screenshot_path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let (data_dir, full_path) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let full_path = state.data_dir.join(&screenshot_path);
        (state.data_dir.clone(), full_path)
    };

    if !full_path.exists() {
        return Err(AppError::Unknown(format!("截图文件不存在: {full_path:?}")));
    }

    let ocr_service = crate::ocr::OcrService::new(&data_dir);

    match ocr_service.extract_text(&full_path) {
        Ok(Some(result)) => {
            // 过滤敏感信息
            let filtered_text = crate::ocr::filter_sensitive_text(&result.text);

            Ok(serde_json::json!({
                "success": true,
                "text": filtered_text,
                "raw_text": result.text,
                "confidence": result.confidence,
                "box_count": result.boxes.len(),
            }))
        }
        Ok(None) => Ok(serde_json::json!({
            "success": true,
            "text": "",
            "message": "未检测到文字",
        })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string(),
        })),
    }
}

/// 获取 OCR 安装指南
#[tauri::command]
pub async fn get_ocr_install_guide() -> Result<serde_json::Value, AppError> {
    let platform = std::env::consts::OS;

    let guide = match platform {
        "windows" => serde_json::json!({
            "platform": "Windows",
            "steps": [
                "1. 确保已安装 Python 3.8+",
                "2. 打开命令提示符或 PowerShell",
                "3. 运行以下命令安装 PaddleOCR：",
                "   pip install paddlepaddle paddleocr -i https://mirror.baidu.com/pypi/simple",
                "4. 等待安装完成（首次运行会自动下载模型）",
                "",
                "备选方案：使用 Windows 内置 OCR（无需安装，但识别效果较弱）"
            ],
            "install_command": "pip install paddlepaddle paddleocr -i https://mirror.baidu.com/pypi/simple",
            "has_builtin_fallback": true,
        }),
        "macos" => serde_json::json!({
            "platform": "macOS",
            "steps": [
                "macOS 使用系统内置的 Vision 框架进行 OCR，无需额外安装。",
                "",
                "如需使用 PaddleOCR（效果更好）：",
                "1. 确保已安装 Python 3.8+",
                "2. 运行以下命令：",
                "   pip install paddlepaddle paddleocr",
            ],
            "install_command": "pip install paddlepaddle paddleocr",
            "has_builtin_fallback": true,
        }),
        _ => serde_json::json!({
            "platform": platform,
            "steps": [
                "1. 确保已安装 Python 3.8+",
                "2. 运行以下命令安装 PaddleOCR：",
                "   pip install paddlepaddle paddleocr",
            ],
            "install_command": "pip install paddlepaddle paddleocr",
            "has_builtin_fallback": false,
        }),
    };

    Ok(guide)
}

/// 设置 Dock 图标可见性 (仅 macOS)
#[tauri::command]
#[allow(unused_variables)]
#[allow(unexpected_cfgs)]
pub fn set_dock_visibility(visible: bool) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        apply_dock_visibility(visible, true);
        log::info!("Dock 图标可见性已设置为: {visible}");
    }

    #[cfg(not(target_os = "macos"))]
    {
        log::warn!("set_dock_visibility 仅支持 macOS");
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn refresh_dock_icon(activate: bool) {
    use cocoa::appkit::{NSApp, NSImage};
    use cocoa::base::nil;
    use cocoa::foundation::NSString;
    use objc::runtime::Object;

    unsafe {
        let app: *mut Object = NSApp();

        // 使用 NSBundle.mainBundle 获取图标路径
        let bundle: *mut Object = objc::msg_send![objc::class!(NSBundle), mainBundle];
        let resource: *mut Object = objc::msg_send![
            bundle,
            pathForResource: NSString::alloc(nil).init_str("icon")
            ofType: NSString::alloc(nil).init_str("icns")
        ];

        // 如果 bundle 中找不到，尝试硬编码路径
        let path_to_use = if resource != nil {
            resource
        } else {
            NSString::alloc(nil)
                .init_str("/Applications/Work Review.app/Contents/Resources/icon.icns")
        };

        let image: *mut Object = NSImage::alloc(nil).initByReferencingFile_(path_to_use);
        if image != nil {
            let _: () = objc::msg_send![app, setApplicationIconImage: image];
            log::info!("已重新设置 Dock 图标");
        }

        if activate {
            let _: () = objc::msg_send![app, activateIgnoringOtherApps: true];
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn apply_dock_visibility(visible: bool, activate: bool) {
    use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
    use objc::runtime::Object;

    unsafe {
        let app: *mut Object = NSApp();

        if visible {
            // 显示 Dock 图标: 切换回 Regular 策略
            app.setActivationPolicy_(
                NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
            );

            // 切换 ActivationPolicy 后主动重载图标，避免启动后 Dock 残留旧图标缓存
            refresh_dock_icon(activate);
        } else {
            // 隐藏 Dock 图标: 切换到 Accessory 策略
            app.setActivationPolicy_(
                NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn apply_dock_visibility(_visible: bool, _activate: bool) {}

/// 获取应用图标（Base64 PNG）
/// 返回应用的图标，如果获取失败返回空字符串
#[tauri::command]
pub async fn get_app_icon(
    app_name: String,
    executable_path: Option<String>,
) -> Result<String, AppError> {
    get_app_icon_impl(&app_name, executable_path.as_deref()).await
}

#[cfg(any(target_os = "macos", test))]
fn normalize_macos_app_lookup_name(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(".app");
    let mut normalized = String::new();
    let mut last_was_space = false;

    for ch in trimmed.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_alphanumeric() {
            normalized.push(ch);
            last_was_space = false;
        } else if !last_was_space {
            normalized.push(' ');
            last_was_space = true;
        }
    }

    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(any(target_os = "macos", test))]
fn push_normalized_macos_lookup_name(target: &mut Vec<String>, value: &str) {
    let normalized = normalize_macos_app_lookup_name(value);
    if normalized.is_empty() || target.iter().any(|existing| existing == &normalized) {
        return;
    }
    target.push(normalized);
}

#[cfg(any(target_os = "macos", test))]
fn macos_lookup_name_variants(value: &str) -> Vec<String> {
    const ALIAS_GROUPS: &[&[&str]] = &[&["腾讯视频", "QQLive", "Tencent Video"]];

    let normalized = normalize_macos_app_lookup_name(value);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut variants = Vec::new();
    push_normalized_macos_lookup_name(&mut variants, value);

    for group in ALIAS_GROUPS {
        if !group
            .iter()
            .any(|alias| normalize_macos_app_lookup_name(alias) == normalized)
        {
            continue;
        }

        for alias in *group {
            push_normalized_macos_lookup_name(&mut variants, alias);
        }
    }

    variants
}

#[cfg(any(target_os = "macos", test))]
fn macos_significant_name_tokens(value: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &["app", "browser", "desktop", "helper", "tools"];

    let mut tokens = Vec::new();
    for token in normalize_macos_app_lookup_name(value).split_whitespace() {
        if token.len() < 2 || STOPWORDS.contains(&token) {
            continue;
        }
        if !tokens.iter().any(|existing| existing == token) {
            tokens.push(token.to_string());
        }
    }
    tokens
}

#[cfg(target_os = "macos")]
fn macos_bundle_path_from_executable(executable_path: &str) -> Option<PathBuf> {
    let path = Path::new(executable_path);
    for ancestor in path.ancestors() {
        let is_app_bundle = ancestor
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("app"))
            .unwrap_or(false);
        if is_app_bundle {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

#[cfg(any(target_os = "macos", test))]
fn score_normalized_macos_app_bundle_name(normalized_app: &str, normalized_bundle: &str) -> i32 {
    if normalized_app.is_empty() || normalized_bundle.is_empty() {
        return 0;
    }

    let mut score = 0;
    if normalized_app == normalized_bundle {
        score += 1000;
    } else if normalized_app.contains(&normalized_bundle)
        || normalized_bundle.contains(&normalized_app)
    {
        score += 500;
    }

    let app_tokens = macos_significant_name_tokens(&normalized_app);
    let bundle_tokens = macos_significant_name_tokens(&normalized_bundle);
    let overlap_count = bundle_tokens
        .iter()
        .filter(|token| app_tokens.iter().any(|candidate| candidate == *token))
        .count() as i32;
    score += overlap_count * 160;

    if let Some(first_token) = app_tokens.first() {
        if normalized_bundle.starts_with(first_token) {
            score += 80;
        }
    }

    score
}

#[cfg(any(target_os = "macos", test))]
fn macos_score_app_bundle_name(app_name: &str, bundle_name: &str) -> i32 {
    let app_variants = macos_lookup_name_variants(app_name);
    let bundle_variants = macos_lookup_name_variants(bundle_name);

    let mut best_score = 0;
    for normalized_app in &app_variants {
        for normalized_bundle in &bundle_variants {
            best_score = best_score.max(score_normalized_macos_app_bundle_name(
                normalized_app,
                normalized_bundle,
            ));
        }
    }

    best_score
}

#[cfg(target_os = "macos")]
fn collect_macos_app_bundles(root: &Path, depth: usize, bundles: &mut Vec<PathBuf>) {
    if depth == 0 || !root.exists() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let is_app_bundle = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("app"))
            .unwrap_or(false);
        if is_app_bundle {
            bundles.push(path);
            continue;
        }

        collect_macos_app_bundles(&path, depth.saturating_sub(1), bundles);
    }
}

#[cfg(target_os = "macos")]
fn macos_icon_app_path_candidates(app_name: &str, executable_path: Option<&str>) -> Vec<String> {
    let mut candidates: Vec<(i32, String)> = Vec::new();

    if let Some(path) = executable_path.and_then(macos_bundle_path_from_executable) {
        candidates.push((i32::MAX, path.to_string_lossy().to_string()));
    }

    let mut search_roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home_dir) = dirs::home_dir() {
        search_roots.push(home_dir.join("Applications"));
    }

    let mut bundles = Vec::new();
    for root in search_roots {
        collect_macos_app_bundles(&root, 3, &mut bundles);
    }

    for bundle in bundles {
        let Some(bundle_name) = bundle.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let score = macos_score_app_bundle_name(app_name, bundle_name);
        if score <= 0 {
            continue;
        }
        candidates.push((score, bundle.to_string_lossy().to_string()));
    }

    candidates.sort_by(|(score_a, path_a), (score_b, path_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| path_a.len().cmp(&path_b.len()))
            .then_with(|| path_a.cmp(path_b))
    });

    let mut deduped = Vec::new();
    for (_, path) in candidates {
        if deduped.iter().any(|existing| existing == &path) {
            continue;
        }
        deduped.push(path);
    }
    deduped
}

/// macOS 实现：使用 mdfind 获取应用图标（带磁盘缓存）
#[cfg(target_os = "macos")]
async fn get_app_icon_impl(
    app_name: &str,
    executable_path: Option<&str>,
) -> Result<String, AppError> {
    use std::path::Path;
    use std::process::Command;

    // 缓存目录：/tmp/work_review_icons/
    let cache_dir = Path::new("/tmp/work_review_icons");
    if !cache_dir.exists() {
        let _ = std::fs::create_dir_all(cache_dir);
    }

    // 安全文件名：将空格和特殊字符替换为下划线
    let safe_name: String = app_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let cache_file = cache_dir.join(format!("{safe_name}.b64"));

    // 检查缓存：如果缓存存在且非空，直接返回
    if cache_file.exists() {
        if let Ok(cached) = std::fs::read_to_string(&cache_file) {
            if !cached.is_empty() {
                log::debug!("从缓存读取图标: {app_name}");
                return Ok(cached);
            }
        }
    }

    let app_path = macos_icon_app_path_candidates(app_name, executable_path)
        .into_iter()
        .find(|candidate| Path::new(candidate).exists())
        .unwrap_or_default();

    if app_path.is_empty() {
        log::debug!("未找到应用路径: {app_name}");
        return Ok(String::new());
    }

    log::debug!("找到应用路径: {app_name} -> {app_path}");

    // 获取 Info.plist 中的图标文件名
    let info_plist = format!("{app_path}/Contents/Info.plist");
    let icon_name = if Path::new(&info_plist).exists() {
        // 使用 defaults read 读取 CFBundleIconFile
        let defaults_output = Command::new("defaults")
            .args(["read", &info_plist, "CFBundleIconFile"])
            .output();

        if let Ok(output) = defaults_output {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // 确保有 .icns 扩展名
                if name.ends_with(".icns") {
                    name
                } else {
                    format!("{name}.icns")
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // 构造图标文件路径
    let icns_path = if !icon_name.is_empty() {
        format!("{app_path}/Contents/Resources/{icon_name}")
    } else {
        // 尝试查找任何 .icns 文件
        let find_output = Command::new("find")
            .args([
                &format!("{app_path}/Contents/Resources"),
                "-name",
                "*.icns",
                "-maxdepth",
                "1",
            ])
            .output()
            .map_err(|e| AppError::Unknown(format!("查找图标失败: {e}")))?;

        String::from_utf8_lossy(&find_output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    };

    if icns_path.is_empty() || !Path::new(&icns_path).exists() {
        log::debug!("未找到图标文件: {app_name}");
        return Ok(String::new());
    }

    log::debug!("找到图标文件: {icns_path}");

    // 使用 sips 转换为 PNG
    let temp_png = format!(
        "/tmp/app_icon_{}_{}.png",
        app_name.replace(' ', "_"),
        std::process::id()
    );

    let sips_output = Command::new("sips")
        .args([
            "-s", "format", "png", "-Z", "128", &icns_path, "--out", &temp_png,
        ])
        .output();

    if let Ok(result) = sips_output {
        if result.status.success() {
            if let Ok(png_data) = std::fs::read(&temp_png) {
                let _ = std::fs::remove_file(&temp_png);
                let base64_str =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);
                // 保存到缓存
                let _ = std::fs::write(&cache_file, &base64_str);
                log::debug!("图标已缓存: {} ({} bytes)", app_name, base64_str.len());
                return Ok(base64_str);
            }
        } else {
            log::debug!("sips 转换失败: {}", String::from_utf8_lossy(&result.stderr));
        }
    }

    let _ = std::fs::remove_file(&temp_png);
    Ok(String::new())
}

/// Windows 实现：使用 Shell API 获取高清应用图标
/// 优先提取 256x256 (JUMBO) 图标，降级到 48x48 (EXTRALARGE)，最后回退到 32x32
/// 带磁盘缓存，避免重复启动 PowerShell
#[cfg(any(target_os = "windows", test))]
fn sanitize_icon_cache_name(value: &str) -> String {
    let safe_name: String = value
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();

    if safe_name.is_empty() {
        "icon".to_string()
    } else {
        safe_name
    }
}

#[cfg(any(target_os = "windows", test))]
fn build_windows_icon_cache_key(app_name: &str, executable_path: Option<&str>) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let safe_name = sanitize_icon_cache_name(app_name);
    let Some(path) = executable_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return safe_name;
    };

    let mut hasher = DefaultHasher::new();
    path.to_lowercase().hash(&mut hasher);
    format!("{safe_name}_{:016x}", hasher.finish())
}

#[cfg(any(target_os = "windows", test))]
fn merge_windows_icon_lookup_candidates(
    executable_path: Option<&str>,
    known_icon_paths: Vec<String>,
) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut push_candidate = |value: &str| {
        let candidate = value.trim().trim_matches('"').replace('/', "\\");
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };

    if let Some(path) = executable_path {
        push_candidate(path);
    }

    for path in known_icon_paths {
        push_candidate(&path);
    }

    candidates
}

#[cfg(target_os = "windows")]
fn windows_known_icon_paths(app_name: &str) -> Vec<String> {
    let trimmed = app_name
        .trim()
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .trim();
    let normalized = trimmed.to_lowercase();
    let compact = normalized
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();

    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let app_data = std::env::var("APPDATA").unwrap_or_default();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

    let mut paths = Vec::new();
    let mut push_path = |path: String| {
        if !path.is_empty() && !paths.contains(&path) {
            paths.push(path);
        }
    };

    match compact.as_str() {
        "explorer" | "fileexplorer" => {
            push_path(format!(r"{}\explorer.exe", windir));
        }
        "msedge" | "edge" | "microsoftedge" => {
            push_path(format!(
                r"{}\Microsoft\Edge\Application\msedge.exe",
                program_files_x86
            ));
            push_path(format!(
                r"{}\Microsoft\Edge\Application\msedge.exe",
                program_files
            ));
        }
        "chrome" | "googlechrome" => {
            push_path(format!(
                r"{}\Google\Chrome\Application\chrome.exe",
                program_files
            ));
            push_path(format!(
                r"{}\Google\Chrome\Application\chrome.exe",
                program_files_x86
            ));
        }
        "wechat" | "weixin" => {
            push_path(format!(r"{}\Tencent\WeChat\WeChat.exe", program_files_x86));
            push_path(format!(r"{}\Tencent\WeChat\WeChat.exe", program_files));
        }
        "wecom" | "wxwork" => {
            push_path(format!(r"{}\Tencent\WeCom\WXWork.exe", program_files_x86));
            push_path(format!(r"{}\Tencent\WeCom\WXWork.exe", program_files));
        }
        "obsidian" => {
            push_path(format!(
                r"{}\Programs\Obsidian\Obsidian.exe",
                local_app_data
            ));
        }
        "pixpin" => {
            push_path(format!(r"{}\PixPin\PixPin.exe", local_app_data));
        }
        "xshell" => {
            push_path(format!(
                r"{}\NetSarang Computer\7\Xshell.exe",
                program_files_x86
            ));
            push_path(format!(
                r"{}\NetSarang Computer\7\Xshell.exe",
                program_files
            ));
            push_path(format!(
                r"{}\NetSarang Computer\8\Xshell.exe",
                program_files_x86
            ));
            push_path(format!(
                r"{}\NetSarang Computer\8\Xshell.exe",
                program_files
            ));
        }
        "wechatappex" => {
            push_path(format!(
                r"{}\Tencent\WeChat\XPlugin\Plugins\WeChatAppEx\WeChatAppEx.exe",
                app_data
            ));
        }
        _ => {}
    }

    paths
}

#[cfg(target_os = "windows")]
fn encode_windows_icon_path(value: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
unsafe fn get_windows_icon_from_shell_image_list(
    path: &str,
    list_type: i32,
) -> Option<winapi::shared::windef::HICON> {
    use std::mem::zeroed;
    use std::ptr::null_mut;
    use winapi::ctypes::c_void;
    use winapi::um::commoncontrols::IImageList;
    use winapi::um::shellapi::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX};
    use winapi::Interface;

    let wide_path = encode_windows_icon_path(path);
    let mut file_info: SHFILEINFOW = zeroed();
    let lookup_result = SHGetFileInfoW(
        wide_path.as_ptr(),
        0,
        &mut file_info,
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_SYSICONINDEX,
    );
    if lookup_result == 0 {
        return None;
    }

    let mut image_list: *mut IImageList = null_mut();
    let hr = SHGetImageList(
        list_type,
        &IImageList::uuidof(),
        &mut image_list as *mut _ as *mut *mut c_void,
    );
    if hr < 0 || image_list.is_null() {
        return None;
    }

    let mut icon = null_mut();
    let hr = (*image_list).GetIcon(file_info.iIcon, 0, &mut icon);
    (*image_list).Release();

    if hr < 0 || icon.is_null() {
        None
    } else {
        Some(icon)
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_windows_associated_icon(path: &str) -> Option<winapi::shared::windef::HICON> {
    use std::ptr::null_mut;
    use winapi::shared::minwindef::WORD;
    use winapi::um::shellapi::ExtractAssociatedIconW;

    let mut wide_path = encode_windows_icon_path(path);
    if wide_path.len() < 260 {
        wide_path.resize(260, 0);
    }

    let mut icon_index: WORD = 0;
    let icon = ExtractAssociatedIconW(null_mut(), wide_path.as_mut_ptr(), &mut icon_index);
    if icon.is_null() {
        None
    } else {
        Some(icon)
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_windows_icon_pixels(
    icon: winapi::shared::windef::HICON,
) -> Option<(Vec<u8>, u32, u32)> {
    const DI_NORMAL: u32 = 0x0003;

    use std::mem::zeroed;
    use std::ptr::{copy_nonoverlapping, null_mut, write_bytes};
    use winapi::shared::minwindef::UINT;
    use winapi::shared::windef::HGDIOBJ;
    use winapi::um::wingdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetObjectW, SelectObject,
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use winapi::um::winuser::{DrawIconEx, GetDC, GetIconInfo, ReleaseDC, ICONINFO};

    let mut icon_info: ICONINFO = zeroed();
    if GetIconInfo(icon, &mut icon_info) == 0 {
        return None;
    }

    let rendered = (|| {
        let source_bitmap = if !icon_info.hbmColor.is_null() {
            icon_info.hbmColor
        } else {
            icon_info.hbmMask
        };
        if source_bitmap.is_null() {
            return None;
        }

        let mut bitmap: BITMAP = zeroed();
        let get_object_result = GetObjectW(
            source_bitmap as *mut _,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bitmap as *mut _ as *mut _,
        );
        if get_object_result == 0 {
            return None;
        }

        let width = bitmap.bmWidth.abs();
        let mut height = bitmap.bmHeight.abs();
        if icon_info.hbmColor.is_null() {
            height /= 2;
        }
        if width <= 0 || height <= 0 {
            return None;
        }

        let screen_dc = GetDC(null_mut());
        if screen_dc.is_null() {
            return None;
        }

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(null_mut(), screen_dc);
            return None;
        }

        let mut bitmap_info: BITMAPINFO = zeroed();
        bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = width;
        bitmap_info.bmiHeader.biHeight = -height;
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = BI_RGB;

        let mut dib_bits = null_mut();
        let dib = CreateDIBSection(
            screen_dc,
            &bitmap_info,
            DIB_RGB_COLORS as UINT,
            &mut dib_bits,
            null_mut(),
            0,
        );
        if dib.is_null() || dib_bits.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);
            return None;
        }

        let old_object = SelectObject(mem_dc, dib as HGDIOBJ);
        if old_object.is_null() {
            DeleteObject(dib as HGDIOBJ);
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);
            return None;
        }

        let pixel_len = width as usize * height as usize * 4;
        write_bytes(dib_bits as *mut u8, 0, pixel_len);

        let draw_result = DrawIconEx(mem_dc, 0, 0, icon, width, height, 0, null_mut(), DI_NORMAL);
        let mut pixels = None;
        if draw_result != 0 {
            let mut buffer = vec![0; pixel_len];
            copy_nonoverlapping(dib_bits as *const u8, buffer.as_mut_ptr(), pixel_len);
            pixels = Some((buffer, width as u32, height as u32));
        }

        SelectObject(mem_dc, old_object);
        DeleteObject(dib as HGDIOBJ);
        DeleteDC(mem_dc);
        ReleaseDC(null_mut(), screen_dc);
        pixels
    })();

    if !icon_info.hbmColor.is_null() {
        DeleteObject(icon_info.hbmColor as HGDIOBJ);
    }
    if !icon_info.hbmMask.is_null() {
        DeleteObject(icon_info.hbmMask as HGDIOBJ);
    }

    rendered
}

#[cfg(target_os = "windows")]
fn encode_windows_icon_base64(mut pixels: Vec<u8>, width: u32, height: u32) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }

    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    let image = image::RgbaImage::from_raw(width, height, pixels)?;
    let mut dynamic_image = image::DynamicImage::ImageRgba8(image);
    if width > 128 || height > 128 {
        dynamic_image = dynamic_image.resize_exact(128, 128, image::imageops::FilterType::Lanczos3);
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    dynamic_image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;

    Some(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        cursor.into_inner(),
    ))
}

#[cfg(target_os = "windows")]
fn convert_windows_icon_to_base64(icon: winapi::shared::windef::HICON) -> Option<(String, u32)> {
    use winapi::um::winuser::DestroyIcon;

    let rendered = unsafe { render_windows_icon_pixels(icon) };
    unsafe {
        DestroyIcon(icon);
    }

    let (pixels, width, height) = rendered?;
    let encoded = encode_windows_icon_base64(pixels, width, height)?;
    Some((encoded, width.max(height)))
}

#[cfg(target_os = "windows")]
fn extract_windows_icon_base64(path: &str) -> Option<String> {
    use winapi::um::shellapi::{SHIL_EXTRALARGE, SHIL_JUMBO};

    let mut jumbo_fallback = None;

    if let Some(icon) = unsafe { get_windows_icon_from_shell_image_list(path, SHIL_JUMBO as i32) } {
        if let Some((encoded, size)) = convert_windows_icon_to_base64(icon) {
            if size >= 48 {
                return Some(encoded);
            }
            jumbo_fallback = Some(encoded);
        }
    }

    let mut extra_large_fallback = None;
    if let Some(icon) =
        unsafe { get_windows_icon_from_shell_image_list(path, SHIL_EXTRALARGE as i32) }
    {
        if let Some((encoded, size)) = convert_windows_icon_to_base64(icon) {
            if size >= 32 {
                return Some(encoded);
            }
            extra_large_fallback = Some(encoded);
        }
    }

    if let Some(icon) = unsafe { get_windows_associated_icon(path) } {
        if let Some((encoded, _)) = convert_windows_icon_to_base64(icon) {
            return Some(encoded);
        }
    }

    extra_large_fallback.or(jumbo_fallback)
}

#[cfg(target_os = "windows")]
async fn get_app_icon_impl(
    app_name: &str,
    executable_path: Option<&str>,
) -> Result<String, AppError> {
    const WINDOWS_ICON_CACHE_VERSION: &str = "v5";

    // 磁盘缓存：检查是否已有缓存
    let cache_dir = std::env::temp_dir().join("work_review_icons");
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache_key = build_windows_icon_cache_key(
        &crate::monitor::normalize_display_app_name(app_name),
        executable_path,
    );
    let cache_file = cache_dir.join(format!("{cache_key}_{WINDOWS_ICON_CACHE_VERSION}.b64"));

    if cache_file.exists() {
        if let Ok(metadata) = std::fs::metadata(&cache_file) {
            // 缓存有效期 24 小时
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or_default().as_secs() < 86400 {
                    if let Ok(cached) = std::fs::read_to_string(&cache_file) {
                        if cached.len() > 100 {
                            return Ok(cached);
                        }
                    }
                }
            }
        }
    }

    let icon_lookup_candidates = merge_windows_icon_lookup_candidates(
        executable_path,
        windows_known_icon_paths(app_name)
            .into_iter()
            .filter(|path| std::path::Path::new(path).exists())
            .collect::<Vec<_>>(),
    );
    if icon_lookup_candidates.is_empty() {
        return Ok(String::new());
    }

    // 仅对明确的可执行路径提取图标，不扫描注册表、开始菜单快捷方式或全部运行进程。
    for candidate_path in icon_lookup_candidates {
        if !Path::new(&candidate_path).exists() {
            continue;
        }

        if let Some(base64_str) = extract_windows_icon_base64(&candidate_path) {
            if base64_str.len() > 100 {
                let _ = std::fs::write(&cache_file, &base64_str);
                return Ok(base64_str);
            }
        }
    }

    Ok(String::new())
}

/// 其他平台：返回空字符串
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn get_app_icon_impl(
    _app_name: &str,
    _executable_path: Option<&str>,
) -> Result<String, AppError> {
    Ok(String::new())
}

/// 保存背景图片（接收 base64 编码的图片数据）
#[tauri::command]
pub async fn save_background_image(
    data: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let (data_dir, config_path) = {
        let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (s.data_dir.clone(), s.config_path.clone())
    };

    let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
        .map_err(|e| AppError::Unknown(format!("base64 解码失败: {e}")))?;
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| AppError::Unknown(format!("图片解析失败: {e}")))?;
    let img = if img.width() > 1920 {
        img.resize(1920, 1920, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let bg_path = data_dir.join("background.jpg");
    img.save_with_format(&bg_path, image::ImageFormat::Jpeg)
        .map_err(|e| AppError::Unknown(format!("保存背景图失败: {e}")))?;

    let mut s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    s.config.background_image = Some("background.jpg".to_string());
    s.config.save(&config_path)?;
    Ok(())
}

/// 获取背景图片（返回 base64）
#[tauri::command]
pub async fn get_background_image(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<String>, AppError> {
    let (data_dir, bg_filename) = {
        let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (s.data_dir.clone(), s.config.background_image.clone())
    };

    let filename = match bg_filename {
        Some(f) if !f.is_empty() => f,
        _ => return Ok(None),
    };

    let bg_path = data_dir.join(&filename);
    if !bg_path.exists() {
        return Ok(None);
    }

    let bytes =
        std::fs::read(&bg_path).map_err(|e| AppError::Unknown(format!("读取背景图失败: {e}")))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(Some(b64))
}

/// 清除背景图片
#[tauri::command]
pub async fn clear_background_image(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let (data_dir, config_path) = {
        let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (s.data_dir.clone(), s.config_path.clone())
    };

    let bg_path = data_dir.join("background.jpg");
    if bg_path.exists() {
        let _ = std::fs::remove_file(&bg_path);
    }

    let mut s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    s.config.background_image = None;
    s.config.save(&config_path)?;
    Ok(())
}
