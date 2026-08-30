using System.Collections.Concurrent;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Graphics.Imaging;
using Windows.Storage.Streams;
using WorkReview.Engine;

namespace WorkReview.Services;

/// <summary>
/// 应用图标缓存：get_app_icon 的 base64 PNG → BitmapImage。
/// 与 Svelte 版 iconCache 相同的 key 规则：appName 或 appName::executablePath。
/// </summary>
public sealed class IconCache
{
    private const int Capacity = 256;
    private readonly ConcurrentDictionary<string, BitmapImage?> _cache = new();
    private readonly ConcurrentQueue<string> _order = new();
    private readonly object _gate = new();

    public static string CacheKey(string appName, string? executablePath) =>
        string.IsNullOrWhiteSpace(executablePath) ? appName : $"{appName}::{executablePath}";

    public bool TryGet(string key, out BitmapImage? icon)
    {
        icon = null;
        return _cache.TryGetValue(key, out icon) && icon is not null;
    }

    public async Task<BitmapImage?> GetAsync(string appName, string? executablePath)
    {
        var key = CacheKey(appName, executablePath);
        if (_cache.TryGetValue(key, out var cached))
        {
            return cached;
        }

        try
        {
            var base64 = await EngineApi.InvokeAsync<string?>("get_app_icon", new
            {
                app_name = appName,
                executable_path = executablePath,
            });

            if (string.IsNullOrWhiteSpace(base64) || base64.Length <= 100)
            {
                _cache[key] = null;
                return null;
            }

            var image = new BitmapImage();
            await image.SetSourceAsync(await Base64ToStream(base64));
            _cache[key] = image;
            Track(key);
            return image;
        }
        catch
        {
            _cache[key] = null;
            return null;
        }
    }

    private void Track(string key)
    {
        lock (_gate)
        {
            _order.Enqueue(key);
            while (_order.Count > Capacity && _order.TryDequeue(out var evicted))
            {
                _cache.TryRemove(evicted, out _);
            }
        }
    }

    private static async Task<InMemoryRandomAccessStream> Base64ToStream(string base64)
    {
        var bytes = Convert.FromBase64String(base64);
        var stream = new InMemoryRandomAccessStream();
        using var writer = new DataWriter(stream.GetOutputStreamAt(0));
        writer.WriteBytes(bytes);
        await writer.StoreAsync();
        await writer.FlushAsync();
        writer.DetachStream();
        return stream;
    }
}
