import os
import sys
import io
import resvg_py
from PIL import Image

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TAURI_ICONS = os.path.join(PROJECT_ROOT, "src-tauri", "icons")
PUBLIC_DIR = os.path.join(PROJECT_ROOT, "public")
PUBLIC_ICONS = os.path.join(PUBLIC_DIR, "icons")

# Fluent-inspired Dark Current App Icon SVG (1024x1024)
MASTER_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <defs>
    <!-- Background Gradient -->
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#161D28"/>
      <stop offset="100%" stop-color="#0A0D12"/>
    </linearGradient>

    <!-- Outer Rim Stroke -->
    <linearGradient id="rimGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#3B485C" stop-opacity="0.8"/>
      <stop offset="100%" stop-color="#1C2330" stop-opacity="0.4"/>
    </linearGradient>

    <!-- Signal Arc Gradient -->
    <linearGradient id="signalGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#38BDF8"/>
      <stop offset="50%" stop-color="#7AA2F7"/>
      <stop offset="100%" stop-color="#6366F1"/>
    </linearGradient>

    <!-- Bar 1 Gradient -->
    <linearGradient id="bar1Grad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#7AA2F7"/>
      <stop offset="100%" stop-color="#A5B4FC"/>
    </linearGradient>

    <!-- Bar 2 Gradient -->
    <linearGradient id="bar2Grad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#38BDF8"/>
      <stop offset="100%" stop-color="#7DD3FC"/>
    </linearGradient>

    <!-- Bar 3 Gradient -->
    <linearGradient id="bar3Grad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#818CF8"/>
      <stop offset="100%" stop-color="#C084FC"/>
    </linearGradient>

    <!-- Drop Shadow for foreground -->
    <filter id="fluentShadow" x="-10%" y="-10%" width="120%" height="120%">
      <feDropShadow dx="0" dy="16" stdDeviation="24" flood-color="#000000" flood-opacity="0.5"/>
    </filter>

    <filter id="glowMint" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur stdDeviation="16" result="blur"/>
      <feComposite in="SourceGraphic" in2="blur" operator="over"/>
    </filter>
  </defs>

  <!-- Squircle Base Canvas -->
  <rect x="32" y="32" width="960" height="960" rx="224" ry="224" fill="url(#bgGrad)" stroke="url(#rimGrad)" stroke-width="8"/>

  <!-- Inner Plate with Subtle Highlight -->
  <rect x="72" y="72" width="880" height="880" rx="184" ry="184" fill="#10151E" fill-opacity="0.6"/>

  <g filter="url(#fluentShadow)">
    <!-- 1. Background Inactive Clock Track (Subtle Slate Arc) -->
    <circle cx="430" cy="512" r="280" fill="none" stroke="#252F3F" stroke-width="52" stroke-linecap="round"/>

    <!-- 2. Active Timeline Flow Arc (From 10 o'clock to 4 o'clock) -->
    <path d="M 215 332 A 280 280 0 0 1 628 709" fill="none" stroke="url(#signalGrad)" stroke-width="52" stroke-linecap="round"/>

    <!-- 3. Center Hub & Clock Hands -->
    <!-- Center Pivot -->
    <circle cx="430" cy="512" r="32" fill="#F3F6FA"/>
    <!-- Hour Hand (pointing toward 2 o'clock) -->
    <path d="M 430 512 L 535 440" stroke="#F3F6FA" stroke-width="36" stroke-linecap="round"/>
    <!-- Minute Hand (pointing toward 12 o'clock) -->
    <path d="M 430 512 L 430 330" stroke="#FFFFFF" stroke-width="36" stroke-linecap="round"/>

    <!-- 4. Review / Activity Stream Ledger Bars -->
    <!-- Activity Bar 1 -->
    <rect x="520" y="340" width="310" height="56" rx="28" fill="url(#bar1Grad)"/>
    <circle cx="550" cy="368" r="14" fill="#FFFFFF"/>

    <!-- Activity Bar 2 -->
    <rect x="520" y="440" width="240" height="56" rx="28" fill="url(#bar2Grad)"/>
    <circle cx="550" cy="468" r="14" fill="#FFFFFF"/>

    <!-- Activity Bar 3 -->
    <rect x="520" y="540" width="280" height="56" rx="28" fill="url(#bar3Grad)"/>
    <circle cx="550" cy="568" r="14" fill="#FFFFFF"/>

    <!-- 5. Active Recording Pulse Point (Mint Beacon) -->
    <g filter="url(#glowMint)">
      <circle cx="215" cy="332" r="36" fill="#43D6A2"/>
      <circle cx="215" cy="332" r="20" fill="#FFFFFF"/>
    </g>
  </g>
</svg>"""

# Monochrome Tray Template SVG (64x64)
TRAY_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <circle cx="28" cy="32" r="18" fill="none" stroke="#FFFFFF" stroke-width="4.5" stroke-linecap="round"/>
  <path d="M 28 32 L 28 20" stroke="#FFFFFF" stroke-width="4" stroke-linecap="round"/>
  <path d="M 28 32 L 36 27" stroke="#FFFFFF" stroke-width="4" stroke-linecap="round"/>
  <!-- Activity bars -->
  <rect x="36" y="20" width="18" height="4" rx="2" fill="#FFFFFF"/>
  <rect x="36" y="28" width="14" height="4" rx="2" fill="#FFFFFF"/>
  <rect x="36" y="36" width="16" height="4" rx="2" fill="#FFFFFF"/>
  <circle cx="15" cy="20" r="3.5" fill="#43D6A2"/>
</svg>"""

def render_svg(svg_str, size):
    png_bytes = resvg_py.svg_to_bytes(svg_str, width=size, height=size)
    return png_bytes

def main():
    os.makedirs(TAURI_ICONS, exist_ok=True)
    os.makedirs(PUBLIC_ICONS, exist_ok=True)

    # 1. Master PNGs
    sizes = [16, 32, 48, 64, 128, 256, 512, 1024]

    for s in sizes:
        png_data = render_svg(MASTER_SVG, s)
        out_path = os.path.join(TAURI_ICONS, f"{s}x{s}.png")
        with open(out_path, "wb") as f:
            f.write(png_data)
        
        if s == 256:
            # 128x128@2x.png
            with open(os.path.join(TAURI_ICONS, "128x128@2x.png"), "wb") as f:
                f.write(png_data)
            # public/icons/256x256.png
            with open(os.path.join(PUBLIC_ICONS, "256x256.png"), "wb") as f:
                f.write(png_data)
        elif s == 128:
            # public/icons/128x128.png
            with open(os.path.join(PUBLIC_ICONS, "128x128.png"), "wb") as f:
                f.write(png_data)
            # public/favicon.png
            with open(os.path.join(PUBLIC_DIR, "favicon.png"), "wb") as f:
                f.write(png_data)
        elif s == 512:
            # icon.png
            with open(os.path.join(TAURI_ICONS, "icon.png"), "wb") as f:
                f.write(png_data)
            with open(os.path.join(PUBLIC_DIR, "icon.png"), "wb") as f:
                f.write(png_data)
        elif s == 1024:
            with open(os.path.join(TAURI_ICONS, "windows-icon.png"), "wb") as f:
                f.write(png_data)

    # 2. Tray icons
    tray_png = render_svg(TRAY_SVG, 64)
    with open(os.path.join(TAURI_ICONS, "tray-icon.png"), "wb") as f:
        f.write(tray_png)
    with open(os.path.join(TAURI_ICONS, "tray-template.png"), "wb") as f:
        f.write(tray_png)

    # 3. Create Windows ICO using Pillow
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    pil_images = []
    for (w, h) in ico_sizes:
        png_data = render_svg(MASTER_SVG, w)
        img = Image.open(io.BytesIO(png_data))
        pil_images.append(img)
    
    ico_dest = os.path.join(TAURI_ICONS, "icon.ico")
    pil_images[0].save(ico_dest, format="ICO", sizes=ico_sizes, append_images=pil_images[1:])
    print(f"Generated all icons successfully in {TAURI_ICONS}")

if __name__ == "__main__":
    main()
