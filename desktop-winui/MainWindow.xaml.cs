using Microsoft.UI;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using WorkReview.Engine;
using WorkReview.Services;
using WorkReview.Views;

namespace WorkReview;

public sealed partial class MainWindow : Window
{
    private static MainWindow? _current;

    public static MainWindow Current => _current
        ?? throw new InvalidOperationException("MainWindow 尚未创建");

    private TimelinePage? _timelinePage;
    private SummaryPage? _summaryPage;
    private SettingsPage? _settingsPage;

    private bool _isRecording = true;
    private bool _isPaused = false;
    private bool _trayQuitRequested;

    private Microsoft.UI.Xaml.Controls.MenuFlyoutItem? TrayShowItem;
    private Microsoft.UI.Xaml.Controls.MenuFlyoutItem? TrayToggleItem;
    private Microsoft.UI.Xaml.Controls.MenuFlyoutItem? TrayQuitItem;

    public MainWindow()
    {
        _current = this;

        App.Stage("window-init-begin");
        InitializeComponent();
        App.Stage("window-init-done");
        AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "Assets", "app.ico"));
        AppWindow.ResizeClient(new Windows.Graphics.SizeInt32(1000, 700));

        Title = "Work Review";

        ConfigureTitleBar();

        // 真实 Windows 11 DWM Mica 背景（不支持的会话自动回退不透明深色层）
        SystemBackdrop = new MicaBackdrop()
        {
            Kind = Microsoft.UI.Composition.SystemBackdrops.MicaKind.Base,
        };

        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);

        EngineEventBridge.Instance.AttachDispatcher(DispatcherQueue);

        App.Stage("tray-begin");
        _ = StartEngineAsync();
        ConfigureTray();
        App.Stage("tray-done");
        BindEvents();
        ApplyLocale();

        Nav.SelectedItem = NavTimeline;
    }

    private void ConfigureTitleBar()
    {
        var titleBar = AppWindow.TitleBar;
        titleBar.BackgroundColor = Colors.Transparent;
        titleBar.InactiveBackgroundColor = Colors.Transparent;
        titleBar.ButtonBackgroundColor = Colors.Transparent;
        titleBar.ButtonInactiveBackgroundColor = Colors.Transparent;
        titleBar.ButtonForegroundColor = Colors.White;
        titleBar.ButtonHoverBackgroundColor = Argb(0x1A, 0xFF, 0xFF, 0xFF);
        titleBar.ButtonPressedBackgroundColor = Argb(0x2E, 0xFF, 0xFF, 0xFF);
    }

    private static Windows.UI.Color Argb(byte a, byte r, byte g, byte b) => new()
    { A = a, R = r, G = g, B = b };

    /// <summary>由时间线页跳转到汇总页并携带日期。</summary>
    public void NavigateToSummary(string? date)
    {
        Nav.SelectedItem = NavSummary;
        _summaryPage ??= new SummaryPage();
        PageHost.Content = _summaryPage;
        if (date is not null)
        {
            _summaryPage.SetDate(date);
        }
        _summaryPage.OnActivated();
    }

    // ---------- 引擎启动 ----------

    private async Task StartEngineAsync()
    {
        try
        {
            await Task.Run(() =>
            {
                // 触发引擎初始化（get_platform 是最轻的探测调用；engine_start 已在 App 中执行）
                EngineApi.Invoke("get_platform");
            });
            await ReloadRecordingStateAsync();
            await CategoryStoreSingleton.Instance.RefreshAsync();
        }
        catch (Exception e)
        {
            _ = DispatcherQueue.TryEnqueue(() =>
                ShowToast(InfoBarSeverity.Error, $"{I18n.T("toast.engineStartFailed")}: {e.Message}", autoClose: false));
        }
    }

    // ---------- 导航 ----------

    private void OnNavSelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (args.SelectedItem is not NavigationViewItem item || item.Tag is not string tag)
        {
            return;
        }

        switch (tag)
        {
            case "timeline":
                _timelinePage ??= new TimelinePage();
                PageHost.Content = _timelinePage;
                _timelinePage.OnActivated();
                break;
            case "summary":
                _summaryPage ??= new SummaryPage();
                PageHost.Content = _summaryPage;
                _summaryPage.OnActivated();
                break;
            case "settings":
                _settingsPage ??= new SettingsPage();
                PageHost.Content = _settingsPage;
                _settingsPage.OnActivated();
                break;
        }
    }

    // ---------- 事件 ----------

    private void BindEvents()
    {
        EngineEventBridge.Instance.ScreenshotTaken += OnScreenshotTaken;
        EngineEventBridge.Instance.RecordingStateChanged += OnRecordingStateChanged;
        EngineEventBridge.Instance.ConfigChanged += OnConfigChanged;
        I18n.LocaleChanged += OnLocaleChanged;

        Activated += (_, args) =>
        {
            if (args.WindowActivationState != WindowActivationState.Deactivated)
            {
                if (_timelinePage is not null && ReferenceEquals(PageHost.Content, _timelinePage))
                {
                    _timelinePage.OnWindowActivated();
                }
            }
        };

        Closed += (_, _) =>
        {
            // 与 Tauri 行为一致：关闭窗口仅隐藏到托盘，退出需托盘菜单
            if (!_trayQuitRequested)
            {
                HideToTray();
            }
        };
    }

    private void OnScreenshotTaken(Models.Activity activity)
    {
        _timelinePage?.OnActivityUpserted(activity);
    }

    private void OnRecordingStateChanged(bool isRecording, bool isPaused)
    {
        _isRecording = isRecording;
        _isPaused = isPaused;
        UpdateRecordingStatus();
    }

    private void OnConfigChanged()
    {
        _ = ReloadRecordingStateAsync();
    }

    private void OnLocaleChanged(string locale)
    {
        ApplyLocale();
    }

    public async Task ReloadRecordingStateAsync()
    {
        try
        {
            var tuple = await EngineApi.InvokeAsync<System.Text.Json.JsonElement>("get_recording_state");
            var arr = tuple.EnumerateArray().ToArray();
            _isRecording = arr.Length > 0 && arr[0].GetBoolean();
            _isPaused = arr.Length > 1 && arr[1].GetBoolean();
            _ = DispatcherQueue.TryEnqueue(UpdateRecordingStatus);
        }
        catch
        {
            // 引擎未就绪时忽略
        }
    }

    private void UpdateRecordingStatus()
    {
        if (_isRecording && !_isPaused)
        {
            RecordingDot.Fill = new SolidColorBrush(Argb(0xFF, 0x6C, 0xCB, 0x5F));
            RecordingStatusText.Text = I18n.T("sidebar.statusActive");
            RecordingToggleIcon.Glyph = "\uE769"; // Pause
        }
        else if (_isRecording && _isPaused)
        {
            RecordingDot.Fill = new SolidColorBrush(Argb(0xFF, 0xFA, 0xBD, 0x2C));
            RecordingStatusText.Text = I18n.T("sidebar.statusPaused");
            RecordingToggleIcon.Glyph = "\uE768"; // Play
        }
        else
        {
            RecordingDot.Fill = new SolidColorBrush(Argb(0xFF, 0x9D, 0x9D, 0x9D));
            RecordingStatusText.Text = I18n.T("sidebar.statusStopped");
            RecordingToggleIcon.Glyph = "\uE768";
        }

        if (TrayToggleItem is not null) TrayToggleItem.Text = _isRecording && !_isPaused
            ? I18n.T("tray.pause")
            : I18n.T("tray.resume");
    }

    private async void OnRecordingToggle(object sender, RoutedEventArgs e)
    {
        var method = _isRecording && !_isPaused ? "pause_recording" : "resume_recording";
        try
        {
            await EngineApi.InvokeAsync(method);
            await ReloadRecordingStateAsync();
        }
        catch (Exception ex)
        {
            ShowToast(InfoBarSeverity.Error, ex.Message);
        }
    }

    private void OnLocaleCycle(object sender, RoutedEventArgs e)
    {
        I18n.CycleLocale();
        AppPreferences.Set("locale", I18n.Locale);
    }

    private void ApplyLocale()
    {
        NavTimeline.Content = I18n.T("sidebar.nav.timeline");
        NavSummary.Content = I18n.T("timeline.periodSummary");
        NavSettings.Content = I18n.T("sidebar.nav.settings");
        LocaleButtonText.Text = I18n.ShortLabel + " · " + I18n.T("sidebar.localeButtonTitle");
        if (TrayShowItem is not null)
        {
            TrayShowItem.Text = I18n.T("tray.show");
        }
        if (TrayQuitItem is not null)
        {
            TrayQuitItem.Text = I18n.T("tray.quit");
        }
        UpdateRecordingStatus();
        _timelinePage?.ApplyLocale();
        _summaryPage?.ApplyLocale();
        _settingsPage?.ApplyLocale();
    }

    // ---------- Toast ----------

    private DispatcherQueueTimer? _toastTimer;

    public void ShowToast(InfoBarSeverity severity, string message, bool autoClose = true)
    {
        ToastInfoBar.Severity = severity;
        ToastInfoBar.Title = message;
        ToastInfoBar.Message = string.Empty;
        ToastLayer.Visibility = Visibility.Visible;
        ToastInfoBar.IsOpen = true;

        _toastTimer?.Stop();
        if (autoClose)
        {
            _toastTimer = DispatcherQueue.CreateTimer();
            _toastTimer.Interval = TimeSpan.FromSeconds(4);
            _toastTimer.Tick += (_, _) =>
            {
                ToastInfoBar.IsOpen = false;
                ToastLayer.Visibility = Visibility.Collapsed;
                _toastTimer?.Stop();
            };
            _toastTimer.Start();
        }
    }

    public static void Toast(InfoBarSeverity severity, string message, bool autoClose = true) =>
        _current?.ShowToast(severity, message, autoClose);

    // ---------- 托盘 ----------

    private void ConfigureTray()
    {
        try
        {
            TrayIcon.ToolTipText = "Work Review";
            TrayIcon.UpdateIcon(new System.Drawing.Icon(Path.Combine(AppContext.BaseDirectory, "Assets", "app.ico")));
            TrayIcon.ContextFlyout = BuildTrayMenu();
            TrayIcon.LeftClickCommand = new RelayCommand(ShowFromTray);
            TrayIcon.ForceCreate();
        }
        catch (Exception e)
        {
            System.Diagnostics.Debug.WriteLine($"托盘初始化失败: {e.Message}");
        }
    }

    private MenuFlyout BuildTrayMenu()
    {
        var menu = new MenuFlyout();
        TrayShowItem = new MenuFlyoutItem { Tag = "show" };
        TrayShowItem.Click += OnTrayShow;
        TrayToggleItem = new MenuFlyoutItem { Tag = "toggle" };
        TrayToggleItem.Click += OnTrayToggle;
        TrayQuitItem = new MenuFlyoutItem { Tag = "quit" };
        TrayQuitItem.Click += OnTrayQuit;
        menu.Items.Add(TrayShowItem);
        menu.Items.Add(new MenuFlyoutSeparator());
        menu.Items.Add(TrayToggleItem);
        menu.Items.Add(new MenuFlyoutSeparator());
        menu.Items.Add(TrayQuitItem);
        return menu;
    }

    private void OnTrayShow(object sender, RoutedEventArgs e) => ShowFromTray();

    private async void OnTrayToggle(object sender, RoutedEventArgs e)
    {
        try
        {
            await EngineApi.InvokeAsync(_isRecording && !_isPaused ? "pause_recording" : "resume_recording");
            await ReloadRecordingStateAsync();
        }
        catch (Exception ex)
        {
            ShowToast(InfoBarSeverity.Error, ex.Message);
        }
    }

    private void OnTrayQuit(object sender, RoutedEventArgs e)
    {
        _trayQuitRequested = true;
        TrayIcon.Dispose();
        Close();
        Environment.Exit(0);
    }

    public void ShowFromTray()
    {
        AppWindow.Show();
        Activate();
    }

    public void HideToTray()
    {
        AppWindow.Hide();
    }
}

/// <summary>轻量命令（托盘左键显示窗口）。</summary>
internal sealed class RelayCommand : System.Windows.Input.ICommand
{
    private readonly Action _action;

    public RelayCommand(Action action)
    {
        _action = action;
    }

    public event EventHandler? CanExecuteChanged
    {
        add { }
        remove { }
    }

    public bool CanExecute(object? parameter) => true;

    public void Execute(object? parameter) => _action();
}

public static class CategoryStoreSingleton
{
    public static CategoryStore Instance { get; } = new();
}
