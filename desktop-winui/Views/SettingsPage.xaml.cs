using System.Text.Json.Nodes;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.Storage.Pickers;
using WorkReview.Engine;
using WorkReview.Models;
using WorkReview.Services;

namespace WorkReview.Views;

/// <summary>设置页：常规 / 隐私 / 存储 三个横向标签 + 单一阅读窗格。</summary>
public sealed partial class SettingsPage : UserControl
{
    private ConfigDocument? _config;
    private StorageStats? _stats;
    private string _dataDir = string.Empty;
    private string _databasePath = string.Empty;
    private string _defaultDataDir = string.Empty;
    private bool _autostartEnabled;
    private bool _loading;
    private bool _saving;

    // 控件引用（按标签构建一次）
    private TextBox _idleThresholdBox = null!;
    private TextBox _reportTimeBox = null!;
    private ToggleSwitch _autoStartSwitch = null!;
    private ComboBox _launchModeBox = null!;
    private ToggleSwitch _screenshotsEnabledSwitch = null!;
    private TextBox _intervalBox = null!;
    private Slider _intervalSlider = null!;
    private CheckBox _keepForeverCheck = null!;
    private TextBox _retentionBox = null!;
    private Slider _retentionSlider = null!;
    private ComboBox _displayModeBox = null!;
    private ComboBox _widthModeBox = null!;
    private TextBox _maxWidthBox = null!;
    private TextBlock _statsText = null!;
    private TextBlock _dataDirText = null!;
    private TextBlock _dbPathText = null!;
    private AutoSuggestBox _appRuleInput = null!;
    private ComboBox _appRuleLevelBox = null!;
    private StackPanel _appRulesHost = null!;
    private StackPanel _recentAppsHost = null!;
    private TextBox _keywordInput = null!;
    private StackPanel _keywordsHost = null!;
    private TextBox _domainInput = null!;
    private StackPanel _domainsHost = null!;
    private TextBlock _exportDirText = null!;
    private ToggleSwitch _autoExportSwitch = null!;

    private List<string> _runningApps = new();
    private List<string> _recentApps = new();

    public SettingsPage()
    {
        InitializeComponent();
        ApplyLocale();
    }

    public void OnActivated()
    {
        if (!_loading && _config is null)
        {
            _ = LoadAsync();
        }
    }

    public void ApplyLocale()
    {
        PageTitle.Text = I18n.T("settings.title");
        PageSubtitle.Text = I18n.T("settings.subtitle");
        TabGeneral.Text = I18n.T("settings.tabs.general");
        TabPrivacy.Text = I18n.T("settings.tabs.privacy");
        TabStorage.Text = I18n.T("settings.tabs.storage");
        SaveButton.Content = _saving ? I18n.T("settings.saving") : I18n.T("settings.save");
        LoadErrorTitle.Text = I18n.T("settings.loadError");
        RetryButton.Content = I18n.T("settings.retry");
    }

    // ---------- 数据加载 ----------

    private async void OnLoadConfig(object sender, RoutedEventArgs e) => await LoadAsync();

    private async Task LoadAsync()
    {
        _loading = true;
        LoadingRing.Visibility = Visibility.Visible;
        LoadingRing.IsActive = true;
        ErrorBanner.Visibility = Visibility.Collapsed;
        ContentScroll.Visibility = Visibility.Collapsed;

        try
        {
            var configTask = ConfigDocument.LoadAsync();
            var statsTask = EngineApi.InvokeAsync<StorageStats>("get_storage_stats");
            var dataDirTask = EngineApi.InvokeAsync<string>("get_data_dir");
            var dbPathTask = EngineApi.InvokeAsync<string>("get_database_path");
            var defaultDirTask = EngineApi.InvokeAsync<string>("get_default_data_dir");
            var autostartTask = EngineApi.InvokeAsync<bool>("is_autostart_enabled");
            var runningTask = EngineApi.InvokeAsync<List<string>>("get_running_apps");
            var recentTask = EngineApi.InvokeAsync<List<string>>("get_recent_apps");

            await Task.WhenAll(configTask, statsTask, dataDirTask, dbPathTask, defaultDirTask, autostartTask, runningTask, recentTask);

            _config = configTask.Result;
            _stats = statsTask.Result;
            _dataDir = dataDirTask.Result;
            _databasePath = dbPathTask.Result;
            _defaultDataDir = defaultDirTask.Result;
            _autostartEnabled = autostartTask.Result;
            _runningApps = runningTask.Result ?? new();
            _recentApps = recentTask.Result ?? new();

            BuildTabs();
            ContentScroll.Visibility = Visibility.Visible;
        }
        catch (Exception e)
        {
            App.Crash($"settings-load failed: {e.GetType().FullName}: {e.Message}\n{e.StackTrace}");
            LoadErrorText.Text = e.Message;
            ErrorBanner.Visibility = Visibility.Visible;
        }
        finally
        {
            _loading = false;
            LoadingRing.IsActive = false;
            LoadingRing.Visibility = Visibility.Collapsed;
        }
    }

    // ---------- 标签构建 ----------

    private void OnTabChanged(SelectorBar sender, SelectorBarSelectionChangedEventArgs args)
    {
        var item = sender.SelectedItem;
        if (item is SelectorBarItem selected && selected.Tag is string tag)
        {
            SelectTab(tag);
        }
    }

    private void SelectTab(string tag)
    {
        if (_config is null)
        {
            return;
        }
        ContentHost.Children.Clear();
        switch (tag)
        {
            case "general":
                BuildGeneralTab(ContentHost);
                break;
            case "privacy":
                BuildPrivacyTab(ContentHost);
                break;
            case "storage":
                BuildStorageTab(ContentHost);
                break;
        }
    }

    private void BuildTabs()
    {
        SelectTab("general");
    }

    // ---------- 控件构建辅助 ----------

    private static Border Card(string title, Action<StackPanel> fill)
    {
        var panel = new StackPanel { Spacing = 12 };
        panel.Children.Add(new TextBlock
        {
            Text = title,
            Style = (Style)Application.Current.Resources["SettingsCardTitle"],
        });
        fill(panel);
        return new Border
        {
            Style = (Style)Application.Current.Resources["SettingsCard"],
            Child = panel,
        };
    }

    // 直接把标题插入既有 body 面板，避免子树重挂载（重挂载会触发 REGDB_E_CLASSNOTREGISTERED）
    private static Border Card(string title, StackPanel body)
    {
        body.Children.Insert(0, new TextBlock
        {
            Text = title,
            Style = (Style)Application.Current.Resources["SettingsCardTitle"],
        });
        return new Border
        {
            Style = (Style)Application.Current.Resources["SettingsCard"],
            Child = body,
        };
    }

    private static Grid Row(string label, string? hint, FrameworkElement control, double controlMinWidth = 220)
    {
        var grid = new Grid { ColumnSpacing = 16 };
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(grid, $"row:{label}");
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var textStack = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center };
        textStack.Children.Add(new TextBlock { Text = label, Style = (Style)Application.Current.Resources["RowLabel"] });
        if (!string.IsNullOrEmpty(hint))
        {
            textStack.Children.Add(new TextBlock { Text = hint, Style = (Style)Application.Current.Resources["RowHint"] });
        }
        grid.Children.Add(textStack);

        control.MinWidth = controlMinWidth;
        control.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(control, 1);
        grid.Children.Add(control);
        return grid;
    }


    // NumberBox 在自包含未打包应用中触发 REGDB_E_CLASSNOTREGISTERED（WASDK 1.7 已知问题），
    // 数字输入退化为 TextBox + 失焦解析。
    private static TextBox NumBox(double initial, double min, double max)
    {
        var box = new TextBox
        {
            Text = initial.ToString("0"),
            CornerRadius = new CornerRadius(4),
        };
        box.LostFocus += (_, _) =>
        {
            if (!double.TryParse(box.Text, out var value))
            {
                value = initial;
            }
            value = Math.Clamp(value, min, max);
            box.Text = value.ToString("0");
        };
        return box;
    }

    private static double NumValue(TextBox box) =>
        double.TryParse(box.Text, out var value) ? value : 0;

    private static ComboBox Combo(params string[] items)
    {
        var combo = new ComboBox { CornerRadius = new CornerRadius(4) };
        foreach (var item in items)
        {
            combo.Items.Add(item);
        }
        return combo;
    }

    // ---------- 常规 ----------

    private void BuildGeneralTab(StackPanel host)
    {
        var config = _config!;

        _idleThresholdBox = NumBox(config.IdleThresholdMinutes, 1, 120);
        _idleThresholdBox.LostFocus += (_, _) =>
        {
            config.IdleThresholdMinutes = Math.Max(1, (long)NumValue(_idleThresholdBox));
        };

        _reportTimeBox = new TextBox
        {
            PlaceholderText = "HH:MM",
            Text = config.Get<string?>("daily_report_auto_generate_time", null) ?? string.Empty,
            CornerRadius = new CornerRadius(4),
            Width = 120,
        };
        _reportTimeBox.TextChanged += (_, _) => config.Set("daily_report_auto_generate_time", NormalizeTime(_reportTimeBox.Text));

        _autoStartSwitch = new ToggleSwitch
        {
            IsOn = _autostartEnabled,
            Margin = new Thickness(0, -6, 0, -8),
        };
        _autoStartSwitch.Toggled += async (_, _) => await ToggleAutoStartAsync(_autoStartSwitch.IsOn);

        _launchModeBox = Combo(
            I18n.T("settingsGeneral.autoStartLaunchShow"),
            I18n.T("settingsGeneral.autoStartLaunchSilent"));
        _launchModeBox.SelectedIndex = config.AutoStartSilent ? 1 : 0;
        _launchModeBox.SelectionChanged += (_, _) =>
        {
            config.AutoStartSilent = _launchModeBox.SelectedIndex == 1;
            if (_autostartEnabled)
            {
                _ = SyncAutoStartRegistrationAsync(true, config.AutoStartSilent);
            }
        };

        host.Children.Add(Card(I18n.T("settingsGeneral.title"), body =>
        {
            body.Children.Add(Row(
                I18n.T("settingsGeneral.idleThreshold"),
                I18n.T("settingsGeneral.idleThresholdHint"),
                _idleThresholdBox, 120));
            body.Children.Add(Row(
                I18n.T("settingsGeneral.reportAutoGenerateTime"),
                I18n.T("settingsGeneral.reportAutoGenerateTimeHint"),
                _reportTimeBox, 120));
            body.Children.Add(Row(
                I18n.T("settingsGeneral.autoStart"),
                null,
                _autoStartSwitch, 0));
            body.Children.Add(Row(
                I18n.T("settingsGeneral.autoStartLaunchMode"),
                null,
                _launchModeBox, 200));
        }));
    }

    private static string? NormalizeTime(string input)
    {
        var text = input.Trim();
        if (text.Length == 0)
        {
            return null;
        }
        if (System.Text.RegularExpressions.Regex.IsMatch(text, @"^([01]?\d|2[0-3]):[0-5]\d$"))
        {
            return text;
        }
        return null;
    }

    private async Task ToggleAutoStartAsync(bool enable)
    {
        try
        {
            var silent = _config?.AutoStartSilent ?? true;
            await SyncAutoStartRegistrationAsync(enable, silent);
            if (_config is not null)
            {
                _config.AutoStart = enable;
                await _config.SaveAsync();
            }
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, e.Message);
            if (_autoStartSwitch is not null)
            {
                _autoStartSwitch.IsOn = !enable;
            }
        }
    }

    private async Task SyncAutoStartRegistrationAsync(bool enable, bool silent)
    {
        try
        {
            if (enable)
            {
                await EngineApi.InvokeAsync("enable_autostart", new { silent });
            }
            else
            {
                await EngineApi.InvokeAsync("disable_autostart");
            }
            _autostartEnabled = enable;
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, e.Message);
        }
    }

    // ---------- 隐私 ----------

    private void BuildPrivacyTab(StackPanel host)
    {
        var config = _config!;

        // 应用规则
        _appRuleInput = new AutoSuggestBox
        {
            PlaceholderText = I18n.T("settingsPrivacy.appPlaceholder"),
            CornerRadius = new CornerRadius(4),
            Width = 240,
        };
        UpdateAppSuggestions(string.Empty);
        _appRuleInput.TextChanged += (_, args) => UpdateAppSuggestions(_appRuleInput.Text);

        _appRuleLevelBox = Combo(
            I18n.T("settingsPrivacy.full"),
            I18n.T("settingsPrivacy.anonymized"),
            I18n.T("settingsPrivacy.ignored"));
        _appRuleLevelBox.SelectedIndex = 0;

        var addRuleButton = new Button
        {
            Content = I18n.T("settingsPrivacy.addRuleAction"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        addRuleButton.Click += (_, _) => AddAppRule();

        var inputRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        inputRow.Children.Add(_appRuleInput);
        inputRow.Children.Add(_appRuleLevelBox);
        inputRow.Children.Add(addRuleButton);

        _recentAppsHost = new StackPanel { Spacing = 6 };
        var batchTitle = new TextBlock
        {
            Text = I18n.T("settingsPrivacy.runningApps"),
            Style = (Style)Application.Current.Resources["RowHint"],
        };

        _appRulesHost = new StackPanel { Spacing = 6 };

        var rulesBody = new StackPanel { Spacing = 10 };
        rulesBody.Children.Add(new TextBlock
        {
            Text = I18n.T("settingsPrivacy.appRulesHint"),
            Style = (Style)Application.Current.Resources["RowHint"],
        });
        rulesBody.Children.Add(inputRow);
        rulesBody.Children.Add(batchTitle);
        rulesBody.Children.Add(BuildBatchChips());
        rulesBody.Children.Add(_appRulesHost);

        host.Children.Add(Card(I18n.T("settingsPrivacy.title") + " · " + I18n.T("settingsPrivacy.appRules"), rulesBody));

        // 关键词
        _keywordInput = new TextBox
        {
            PlaceholderText = I18n.T("settingsPrivacy.keywordPlaceholder"),
            CornerRadius = new CornerRadius(4),
            Width = 240,
        };
        var addKeywordButton = new Button
        {
            Content = I18n.T("settingsPrivacy.add"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        addKeywordButton.Click += (_, _) => AddKeyword();

        _keywordsHost = new StackPanel { Spacing = 6 };

        var keywordBody = new StackPanel { Spacing = 10 };
        keywordBody.Children.Add(new TextBlock
        {
            Text = I18n.T("settingsPrivacy.contentFilterDesc"),
            Style = (Style)Application.Current.Resources["RowHint"],
        });
        var keywordRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        keywordRow.Children.Add(_keywordInput);
        keywordRow.Children.Add(addKeywordButton);
        keywordBody.Children.Add(keywordRow);
        keywordBody.Children.Add(_keywordsHost);
        host.Children.Add(Card(I18n.T("settingsPrivacy.contentFilter"), keywordBody));

        // 域名黑名单
        _domainInput = new TextBox
        {
            PlaceholderText = "example.com",
            CornerRadius = new CornerRadius(4),
            Width = 240,
        };
        var addDomainButton = new Button
        {
            Content = I18n.T("settingsPrivacy.add"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        addDomainButton.Click += (_, _) => AddDomain();

        _domainsHost = new StackPanel { Spacing = 6 };

        var domainBody = new StackPanel { Spacing = 10 };
        domainBody.Children.Add(new TextBlock
        {
            Text = I18n.T("settingsPrivacy.domainsHint"),
            Style = (Style)Application.Current.Resources["RowHint"],
        });
        var domainRow = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        domainRow.Children.Add(_domainInput);
        domainRow.Children.Add(addDomainButton);
        domainBody.Children.Add(domainRow);
        domainBody.Children.Add(_domainsHost);
        host.Children.Add(Card(I18n.T("settingsPrivacy.domains"), domainBody));

        RefreshPrivacyLists();
    }

    private void UpdateAppSuggestions(string query)
    {
        var candidates = _runningApps.Concat(_recentApps)
            .Where(a => string.IsNullOrWhiteSpace(query) ||
                        a.Contains(query, StringComparison.OrdinalIgnoreCase))
            .Distinct()
            .Take(8)
            .ToList();
        _appRuleInput.ItemsSource = candidates;
    }

    private StackPanel BuildBatchChips()
    {
        var wrapHost = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        foreach (var app in _runningApps.Take(6))
        {
            wrapHost.Children.Add(BuildBatchChip(app));
        }
        foreach (var app in _recentApps.Take(4))
        {
            wrapHost.Children.Add(BuildBatchChip(app));
        }
        return wrapHost;
    }

    private Button BuildBatchChip(string app)
    {
        var chip = new Button
        {
            Content = app,
            Padding = new Thickness(10, 3, 10, 4),
            CornerRadius = new CornerRadius(4),
            Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
            BorderBrush = (Brush)Application.Current.Resources["WaStrokeBrush"],
            FontSize = 12,
        };
        chip.Click += (_, _) =>
        {
            _appRuleInput.Text = app;
            UpdateAppSuggestions(app);
        };
        return chip;
    }

    private void AddAppRule()
    {
        var appName = _appRuleInput.Text.Trim();
        if (appName.Length == 0 || _config is null)
        {
            return;
        }
        var level = _appRuleLevelBox.SelectedIndex switch
        {
            1 => "anonymized",
            2 => "ignored",
            _ => "full",
        };

        var rules = _config.AppPrivacyRules ?? new JsonArray();
        rules.Add(new JsonObject
        {
            ["app_name"] = appName,
            ["level"] = level,
        });
        if (_config.AppPrivacyRules is null)
        {
            _config.Set("privacy.app_rules", rules);
        }
        _appRuleInput.Text = string.Empty;
        RefreshPrivacyLists();
    }

    private void AddKeyword()
    {
        var keyword = _keywordInput.Text.Trim();
        if (keyword.Length == 0 || _config is null)
        {
            return;
        }
        _config.ExcludedKeywords.Add(keyword);
        _keywordInput.Text = string.Empty;
        RefreshPrivacyLists();
    }

    private void AddDomain()
    {
        var domain = _domainInput.Text.Trim().TrimStart('.', '@');
        if (domain.Length == 0 || _config is null)
        {
            return;
        }
        _config.ExcludedDomains.Add(domain);
        _domainInput.Text = string.Empty;
        RefreshPrivacyLists();
    }

    private void RefreshPrivacyLists()
    {
        var config = _config!;
        _appRulesHost.Children.Clear();
        _keywordsHost.Children.Clear();
        _domainsHost.Children.Clear();

        foreach (var ruleNode in config.AppPrivacyRules?.OfType<JsonObject>().ToList() ?? new List<JsonObject>())
        {
            var appName = ruleNode["app_name"]?.GetValue<string>() ?? string.Empty;
            var level = ruleNode["level"]?.GetValue<string>() ?? "full";
            var levelLabel = level switch
            {
                "anonymized" => I18n.T("settingsPrivacy.anonymized"),
                "ignored" => I18n.T("settingsPrivacy.ignored"),
                _ => I18n.T("settingsPrivacy.full"),
            };
            _appRulesHost.Children.Add(BuildListRow(appName, levelLabel, () =>
            {
                config.AppPrivacyRules?.Remove(ruleNode);
                RefreshPrivacyLists();
            }));
        }

        if ((config.AppPrivacyRules?.Count ?? 0) == 0)
        {
            _appRulesHost.Children.Add(new TextBlock
            {
                Text = I18n.T("settingsPrivacy.noRules"),
                Style = (Style)Application.Current.Resources["RowHint"],
            });
        }

        foreach (var keywordNode in config.ExcludedKeywords.ToList())
        {
            var keyword = keywordNode?.GetValue<string>() ?? string.Empty;
            _keywordsHost.Children.Add(BuildListRow(keyword, null, () =>
            {
                config.ExcludedKeywords.Remove(keyword);
                RefreshPrivacyLists();
            }));
        }

        foreach (var domainNode in config.ExcludedDomains.ToList())
        {
            var domain = domainNode?.GetValue<string>() ?? string.Empty;
            _domainsHost.Children.Add(BuildListRow(domain, null, () =>
            {
                config.ExcludedDomains.Remove(domain);
                RefreshPrivacyLists();
            }));
        }
    }

    private Grid BuildListRow(string text, string? chip, Action remove)
    {
        var grid = new Grid { ColumnSpacing = 10 };
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var label = new TextBlock
        {
            Text = text,
            FontSize = 13,
            VerticalAlignment = VerticalAlignment.Center,
            TextTrimming = TextTrimming.CharacterEllipsis,
            Foreground = (Brush)Application.Current.Resources["WaTextPrimaryBrush"],
        };
        grid.Children.Add(label);

        if (chip is not null)
        {
            var chipBorder = new Border
            {
                Background = (Brush)Application.Current.Resources["WaLayerAltBrush"],
                CornerRadius = new CornerRadius(4),
                Padding = new Thickness(8, 2, 8, 3),
                Child = new TextBlock
                {
                    Text = chip,
                    FontSize = 11,
                    Foreground = (Brush)Application.Current.Resources["WaAccentTextBrush"],
                },
            };
            Grid.SetColumn(chipBorder, 1);
            grid.Children.Add(chipBorder);
        }

        var deleteButton = new Button
        {
            Content = I18n.T("settingsPrivacy.delete"),
            Padding = new Thickness(8, 2, 8, 4),
            CornerRadius = new CornerRadius(4),
            FontSize = 12,
        };
        deleteButton.Click += (_, _) => remove();
        Grid.SetColumn(deleteButton, 2);
        grid.Children.Add(deleteButton);
        return grid;
    }

    // ---------- 存储 ----------

    private void BuildStorageTab(StackPanel host)
    {
        var config = _config!;

        // 截图卡片
        _screenshotsEnabledSwitch = new ToggleSwitch
        {
            IsOn = config.ScreenshotsEnabled,
            Margin = new Thickness(0, -6, 0, -8),
        };
        _screenshotsEnabledSwitch.Toggled += (_, _) => config.ScreenshotsEnabled = _screenshotsEnabledSwitch.IsOn;

        _intervalBox = NumBox(config.ScreenshotInterval, 5, 600);
        _intervalSlider = new Slider
        {
            Minimum = 10,
            Maximum = 120,
            StepFrequency = 1,
            Value = Math.Clamp(config.ScreenshotInterval, 10, 120),
            Width = 220,
        };
        _intervalBox.LostFocus += (_, _) =>
        {
            config.ScreenshotInterval = Math.Max(5, (long)NumValue(_intervalBox));
            if (_intervalSlider.Value != Math.Clamp(config.ScreenshotInterval, 10, 120))
            {
                _intervalSlider.Value = Math.Clamp(config.ScreenshotInterval, 10, 120);
            }
        };
        _intervalSlider.ValueChanged += (_, _) =>
        {
            var sliderValue = (long)_intervalSlider.Value;
            if (NumValue(_intervalBox) != sliderValue)
            {
                _intervalBox.Text = sliderValue.ToString("0");
            }
        };

        var intervalStack = new StackPanel { Spacing = 6 };
        intervalStack.Children.Add(_intervalBox);
        intervalStack.Children.Add(_intervalSlider);

        _keepForeverCheck = new CheckBox
        {
            Content = I18n.T("settingsStorage.keepForever"),
            IsChecked = config.ScreenshotRetentionDays >= 9999,
        };
        _retentionBox = NumBox(config.ScreenshotRetentionDays, 1, 9999);
        _retentionBox.IsEnabled = config.ScreenshotRetentionDays < 9999;
        _retentionSlider = new Slider
        {
            Minimum = 1,
            Maximum = 90,
            StepFrequency = 1,
            Value = Math.Clamp(config.ScreenshotRetentionDays, 1, 90),
            IsEnabled = config.ScreenshotRetentionDays < 9999,
            Width = 220,
        };
        _keepForeverCheck.Checked += (_, _) => SetKeepForever(true);
        _keepForeverCheck.Unchecked += (_, _) => SetKeepForever(false);
        _retentionBox.LostFocus += (_, _) =>
        {
            if (NumValue(_retentionBox) >= 1)
            {
                config.ScreenshotRetentionDays = (long)NumValue(_retentionBox);
            }
            if (_retentionSlider.Value != Math.Clamp(config.ScreenshotRetentionDays, 1, 90))
            {
                _retentionSlider.Value = Math.Clamp(config.ScreenshotRetentionDays, 1, 90);
            }
        };
        _retentionSlider.ValueChanged += (_, _) =>
        {
            var retentionSliderValue = (long)_retentionSlider.Value;
            if (NumValue(_retentionBox) != retentionSliderValue)
            {
                _retentionBox.Text = retentionSliderValue.ToString("0");
            }
        };

        var retentionStack = new StackPanel { Spacing = 6 };
        retentionStack.Children.Add(_keepForeverCheck);
        retentionStack.Children.Add(_retentionBox);
        retentionStack.Children.Add(_retentionSlider);

        _displayModeBox = Combo(
            I18n.T("settingsStorage.screenshotModeActive"),
            I18n.T("settingsStorage.screenshotModeAll"));
        _displayModeBox.SelectedIndex = config.ScreenshotDisplayMode == "all" ? 1 : 0;
        _displayModeBox.SelectionChanged += (_, _) =>
            config.ScreenshotDisplayMode = _displayModeBox.SelectedIndex == 1 ? "all" : "active_window";

        _widthModeBox = Combo(
            I18n.T("settingsStorage.widthModeAuto"),
            I18n.T("settingsStorage.widthModeFixed"));
        _widthModeBox.SelectedIndex = config.ScreenshotWidthMode == "fixed" ? 1 : 0;
        _maxWidthBox = NumBox(config.MaxImageWidth, 640, 3840);
        _maxWidthBox.IsEnabled = config.ScreenshotWidthMode == "fixed";
        _widthModeBox.SelectionChanged += (_, _) =>
        {
            var fixedMode = _widthModeBox.SelectedIndex == 1;
            config.ScreenshotWidthMode = fixedMode ? "fixed" : "auto";
            _maxWidthBox.IsEnabled = fixedMode;
        };
        _maxWidthBox.LostFocus += (_, _) =>
        {
            if (NumValue(_maxWidthBox) >= 640)
            {
                config.MaxImageWidth = (long)NumValue(_maxWidthBox);
            }
        };

        var widthStack = new StackPanel { Spacing = 8 };
        widthStack.Children.Add(_widthModeBox);
        widthStack.Children.Add(Row(I18n.T("settingsStorage.maxWidth"), null, _maxWidthBox, 120));

        var screenshotBody = new StackPanel { Spacing = 14 };
        screenshotBody.Children.Add(Row(
            I18n.T("settingsStorage.screenshotsEnabled"),
            I18n.T("settingsStorage.screenshotsEnabledHint"),
            _screenshotsEnabledSwitch, 0));
        screenshotBody.Children.Add(Row(
            I18n.T("settingsStorage.pollingInterval"),
            I18n.T("settingsStorage.precise") + " ←→ " + I18n.T("settingsStorage.powerSave"),
            intervalStack, 220));
        screenshotBody.Children.Add(Row(
            I18n.T("settingsStorage.retentionDays"),
            I18n.T("settingsStorage.retentionMin") + " ←→ " + I18n.T("settingsStorage.retentionMax"),
            retentionStack, 220));
        screenshotBody.Children.Add(Row(
            I18n.T("settingsStorage.screenshotMode"),
            null,
            _displayModeBox, 200));
        screenshotBody.Children.Add(Row(
            I18n.T("settingsStorage.widthMode"),
            null,
            widthStack, 200));
        host.Children.Add(Card(I18n.T("settingsStorage.screenshotCardTitle"), screenshotBody));

        // 存储统计
        _statsText = new TextBlock
        {
            Style = (Style)Application.Current.Resources["WaMono"],
            TextWrapping = TextWrapping.Wrap,
            FontSize = 12,
        };
        var clearHistoryButton = new Button
        {
            Content = I18n.T("settingsStorage.clearHistoryAction"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        clearHistoryButton.Click += (_, _) => _ = ClearHistoryAsync();
        var statsBody = new StackPanel { Spacing = 10 };
        statsBody.Children.Add(_statsText);
        statsBody.Children.Add(clearHistoryButton);
        host.Children.Add(Card(I18n.T("settingsStorage.statsTitle"), statsBody));

        // 数据位置
        _dataDirText = new TextBlock
        {
            Style = (Style)Application.Current.Resources["WaMono"],
            FontSize = 12,
            TextWrapping = TextWrapping.Wrap,
        };
        _dbPathText = new TextBlock
        {
            Style = (Style)Application.Current.Resources["WaMono"],
            FontSize = 12,
            TextWrapping = TextWrapping.Wrap,
        };

        var openDirButton = new Button
        {
            Content = I18n.T("settingsStorage.openDir"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        openDirButton.Click += (_, _) => _ = EngineApi.InvokeAsync("open_data_dir");
        var changeDirButton = new Button
        {
            Content = I18n.T("settingsStorage.changeDir"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        changeDirButton.Click += (_, _) => _ = ChangeDataDirAsync();
        var changeDbButton = new Button
        {
            Content = I18n.T("settingsStorage.changeDatabase"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        changeDbButton.Click += (_, _) => _ = ChangeDatabasePathAsync();

        var locationBody = new StackPanel { Spacing = 12 };
        locationBody.Children.Add(new StackPanel
        {
            Spacing = 6,
            Children =
            {
                new TextBlock { Text = I18n.T("settingsStorage.dataDir"), Style = (Style)Application.Current.Resources["RowLabel"] },
                _dataDirText,
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 8,
                    Children = { openDirButton, changeDirButton },
                },
            },
        });
        locationBody.Children.Add(new StackPanel
        {
            Spacing = 6,
            Children =
            {
                new TextBlock { Text = I18n.T("settingsStorage.databasePath"), Style = (Style)Application.Current.Resources["RowLabel"] },
                _dbPathText,
                changeDbButton,
            },
        });
        host.Children.Add(Card(I18n.T("settingsStorage.locationTitle"), locationBody));

        // 日报导出
        _exportDirText = new TextBlock
        {
            Style = (Style)Application.Current.Resources["WaMono"],
            FontSize = 12,
            TextWrapping = TextWrapping.Wrap,
        };
        _autoExportSwitch = new ToggleSwitch
        {
            IsOn = config.Get("daily_report_auto_export", false),
            Margin = new Thickness(0, -6, 0, -8),
        };
        _autoExportSwitch.Toggled += (_, _) => config.Set("daily_report_auto_export", _autoExportSwitch.IsOn);

        var chooseExportButton = new Button
        {
            Content = I18n.T("settingsStorage.chooseDir"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        chooseExportButton.Click += (_, _) => _ = ChooseExportDirAsync();
        var clearExportButton = new Button
        {
            Content = I18n.T("settingsStorage.clearDir"),
            Padding = new Thickness(12, 6, 12, 6),
            CornerRadius = new CornerRadius(4),
        };
        clearExportButton.Click += (_, _) =>
        {
            _config?.Set("daily_report_export_dir", (string?)null);
            _exportDirText.Text = I18n.T("settingsStorage.notSet");
        };

        var exportBody = new StackPanel { Spacing = 12 };
        exportBody.Children.Add(new StackPanel
        {
            Spacing = 6,
            Children =
            {
                new TextBlock { Text = I18n.T("settingsStorage.exportDir"), Style = (Style)Application.Current.Resources["RowLabel"] },
                _exportDirText,
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 8,
                    Children = { chooseExportButton, clearExportButton },
                },
            },
        });
        exportBody.Children.Add(Row(I18n.T("settingsStorage.autoExport"), null, _autoExportSwitch, 0));
        host.Children.Add(Card(I18n.T("settingsStorage.exportTitle"), exportBody));

        RefreshStorageInfo();
    }

    private void SetKeepForever(bool forever)
    {
        var config = _config!;
        if (forever)
        {
            config.ScreenshotRetentionDays = 9999;
            _retentionBox.Text = "9999";
        }
        else
        {
            config.ScreenshotRetentionDays = 30;
            _retentionBox.Text = "30";
            _retentionSlider.Value = 30;
        }
        _retentionBox.IsEnabled = !forever;
        _retentionSlider.IsEnabled = !forever;
    }

    private void RefreshStorageInfo()
    {
        var config = _config!;
        if (_stats is not null)
        {
            _statsText.Text = string.Join("\n",
                $"{I18n.T("settingsStorage.totalSize")}: {_stats.TotalSizeMb} MB",
                $"{I18n.T("settingsStorage.fileCount")}: {_stats.TotalFiles}",
                $"{I18n.T("settingsStorage.retentionDaysLabel")}: {_stats.RetentionDays}",
                $"{I18n.T("settingsStorage.storageLimit")}: {_stats.StorageLimitMb} MB");
        }

        _dataDirText.Text = _dataDir;
        _dbPathText.Text = _databasePath;
        _exportDirText.Text = config.Get<string?>("daily_report_export_dir", null) ?? I18n.T("settingsStorage.notSet");
    }

    // ---------- 存储操作 ----------

    private async Task ClearHistoryAsync()
    {
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = (Style)Application.Current.Resources["WaContentDialog"],
            Title = I18n.T("settingsStorage.clearHistoryConfirmTitle"),
            Content = I18n.T("settingsStorage.clearHistoryConfirmMessage"),
            PrimaryButtonText = I18n.T("settingsStorage.clearHistoryAction"),
            CloseButtonText = I18n.T("common.cancel"),
            DefaultButton = ContentDialogButton.Close,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary)
        {
            return;
        }
        try
        {
            await EngineApi.InvokeAsync("clear_old_activities");
            MainWindow.Toast(InfoBarSeverity.Success, I18n.T("settingsStorage.clearDone"));
            await LoadAsync();
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, I18n.T("settingsStorage.clearFailed", ("error", e.Message)));
        }
    }

    private async Task<string?> PickFolderAsync()
    {
        var picker = new FolderPicker();
        WinRT.Interop.InitializeWithWindow.Initialize(picker, AppWindowHandle());
        picker.FileTypeFilter.Add("*");
        var folder = await picker.PickSingleFolderAsync();
        return folder?.Path;
    }

    private async Task ChangeDataDirAsync()
    {
        var nextDir = await PickFolderAsync();
        if (string.IsNullOrEmpty(nextDir) || _config is null)
        {
            return;
        }
        if (string.Equals(Path.GetFullPath(nextDir).TrimEnd('\\'), _dataDir.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))
        {
            MainWindow.Toast(InfoBarSeverity.Informational, I18n.T("settingsStorage.alreadyCurrentDir"));
            return;
        }

        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = (Style)Application.Current.Resources["WaContentDialog"],
            Title = I18n.T("settingsStorage.migrateConfirmTitle"),
            Content = I18n.T("settingsStorage.migrateConfirmMessage", ("dir", nextDir)),
            PrimaryButtonText = I18n.T("common.confirm"),
            CloseButtonText = I18n.T("common.cancel"),
            DefaultButton = ContentDialogButton.Primary,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary)
        {
            return;
        }

        try
        {
            await EngineApi.InvokeAsync("change_data_dir", new { target_dir = nextDir });
            MainWindow.Toast(InfoBarSeverity.Success, I18n.T("settingsStorage.migrated"));
            await LoadAsync();
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, I18n.T("settingsStorage.migrateFailed", ("error", e.Message)));
        }
    }

    private async Task ChangeDatabasePathAsync()
    {
        var picker = new FileOpenPicker();
        WinRT.Interop.InitializeWithWindow.Initialize(picker, AppWindowHandle());
        picker.FileTypeFilter.Add(".db");
        picker.FileTypeFilter.Add(".sqlite");
        picker.FileTypeFilter.Add(".sqlite3");
        var file = await picker.PickSingleFileAsync();
        if (file is null)
        {
            return;
        }

        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = (Style)Application.Current.Resources["WaContentDialog"],
            Title = I18n.T("settingsStorage.migrateConfirmTitle"),
            Content = I18n.T("settingsStorage.migrateDatabaseMessage", ("path", file.Path)),
            PrimaryButtonText = I18n.T("common.confirm"),
            CloseButtonText = I18n.T("common.cancel"),
            DefaultButton = ContentDialogButton.Primary,
        };
        if (await dialog.ShowAsync() != ContentDialogResult.Primary)
        {
            return;
        }

        try
        {
            await EngineApi.InvokeAsync("change_database_path", new { target_path = file.Path });
            MainWindow.Toast(InfoBarSeverity.Success, I18n.T("settingsStorage.migrated"));
            await LoadAsync();
        }
        catch (Exception e)
        {
            MainWindow.Toast(InfoBarSeverity.Error, I18n.T("settingsStorage.migrateFailed", ("error", e.Message)));
        }
    }

    private async Task ChooseExportDirAsync()
    {
        var dir = await PickFolderAsync();
        if (string.IsNullOrEmpty(dir) || _config is null)
        {
            return;
        }
        _config.Set("daily_report_export_dir", dir);
        _exportDirText.Text = dir;
    }

    private static IntPtr AppWindowHandle()
    {
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(MainWindow.Current);
        return hwnd;
    }

    // ---------- 保存 ----------

    private async void OnSaveClicked(object sender, RoutedEventArgs e)
    {
        if (_config is null || _saving)
        {
            return;
        }
        _saving = true;
        SaveButton.IsEnabled = false;
        SaveButton.Content = I18n.T("settings.saving");
        try
        {
            await _config.SaveAsync();
            MainWindow.Toast(InfoBarSeverity.Success, I18n.T("settings.saveSuccessToast"));
        }
        catch (Exception ex)
        {
            MainWindow.Toast(InfoBarSeverity.Error, ex.Message);
        }
        finally
        {
            _saving = false;
            SaveButton.IsEnabled = true;
            SaveButton.Content = I18n.T("settings.save");
        }
    }
}
