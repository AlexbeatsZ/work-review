using System.Collections.ObjectModel;
using System.Text.Json.Nodes;
using WorkReview.Engine;
using WorkReview.Models;

namespace WorkReview.Services;

/// <summary>分类缓存（get_categories / get_semantic_categories）。</summary>
public sealed class CategoryStore
{
    public ObservableCollection<CategoryInfo> Categories { get; } = new();
    public ObservableCollection<SemanticCategoryInfo> SemanticCategories { get; } = new();

    public event Action? CategoriesChanged;

    public async Task RefreshAsync()
    {
        try
        {
            var cats = await EngineApi.InvokeAsync<List<CategoryInfo>>("get_categories");
            var semantic = await EngineApi.InvokeAsync<List<SemanticCategoryInfo>>("get_semantic_categories");

            Categories.Clear();
            foreach (var c in cats)
            {
                Categories.Add(c);
            }
            SemanticCategories.Clear();
            foreach (var s in semantic)
            {
                SemanticCategories.Add(s);
            }
            CategoriesChanged?.Invoke();
        }
        catch (Exception e)
        {
            System.Diagnostics.Debug.WriteLine($"获取分类列表失败: {e.Message}");
        }
    }

    public (string Color, string Icon, string Name, bool IsCustom) GetCategoryMeta(string? key)
    {
        var actual = string.IsNullOrWhiteSpace(key) ? "other" : key!;
        var found = Categories.FirstOrDefault(c => c.Key == actual);
        if (found is not null)
        {
            return found.IsCustom
                ? (found.Color, found.Icon, found.Name, true)
                : (found.Color, found.Icon, I18n.TranslateCategoryLabel(found.Key), false);
        }
        return ("#64748B", "📁", I18n.TranslateCategoryLabel(actual), false);
    }

    public string GetSemanticDisplayName(string key)
    {
        var found = SemanticCategories.FirstOrDefault(s => s.Key == key);
        if (found is not null)
        {
            return found.IsCustom ? found.Name : I18n.TranslateSemanticCategoryLabel(found.Key);
        }
        return I18n.TranslateSemanticCategoryLabel(key) is { } translated && translated != key
            ? translated
            : key;
    }

    public static Windows.UI.Color ParseColor(string hex)
    {
        try
        {
            if (hex is { Length: 7 } && hex[0] == '#')
            {
                var r = Convert.ToByte(hex[1..3], 16);
                var g = Convert.ToByte(hex[3..5], 16);
                var b = Convert.ToByte(hex[5..7], 16);
                return Windows.UI.Color.FromArgb(0xFF, r, g, b);
            }
        }
        catch
        {
            // fall through
        }
        return Windows.UI.Color.FromArgb(0xFF, 0x64, 0x74, 0x8B);
    }
}
