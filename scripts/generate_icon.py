"""
Generate a Stargate app icon using the exact same SVG logo as in the app.
Uses the Lucide "orbit" icon (src/components/icon.rs IconName::Logo).
Rendered with blue-400 (#60a5fa) on a dark slate background.
"""
import cairosvg
from PIL import Image
import io
import os

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The exact same icon as used in the app (src/components/icon.rs IconName::Logo)
# This is the Lucide "orbit" icon, a stroke-based 24x24 glyph.
# Color: text-blue-400 = #60a5fa, background: slate-900 = #0f172a
# The 24x24 glyph is centered within the 24x24 viewport with padding via transform.
SVG = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="1024" height="1024">
  <!-- Background -->
  <rect x="0" y="0" width="24" height="24" rx="3.5" fill="#0f172a"/>

  <!-- Lucide "orbit" icon, scaled down and centered to leave padding -->
  <g transform="translate(3 3) scale(0.75)"
     fill="none" stroke="#60a5fa" stroke-width="2"
     stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="3"/>
    <circle cx="19" cy="5" r="2"/>
    <circle cx="5" cy="19" r="2"/>
    <path d="M10.4 21.9a10 10 0 0 0 9.941-15.416"/>
    <path d="M13.5 2.1a10 10 0 0 0-9.841 15.416"/>
  </g>
</svg>
"""

# Render SVG to PNG at 1024x1024
png_data = cairosvg.svg2png(bytestring=SVG.encode(), output_width=1024, output_height=1024)

# Save as PNG
icon_png_path = os.path.join(PROJECT_ROOT, "assets", "icon.png")
with open(icon_png_path, "wb") as f:
    f.write(png_data)

# Also save to icons/ directory
icons_dir = os.path.join(PROJECT_ROOT, "icons")
os.makedirs(icons_dir, exist_ok=True)
with open(os.path.join(icons_dir, "icon.png"), "wb") as f:
    f.write(png_data)

# Create ICO with multiple sizes
img = Image.open(io.BytesIO(png_data))
ico_path = os.path.join(PROJECT_ROOT, "assets", "icon.ico")
img.save(ico_path, format="ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

# Also save to icons/
img.save(os.path.join(icons_dir, "icon.ico"), format="ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

# Create ICNS (macOS app bundle icon)
def write_icns(dst_path):
    icns_img = Image.open(io.BytesIO(png_data)).convert("RGBA")
    try:
        icns_img.save(dst_path, format="ICNS")
        return True
    except Exception as exc:  # pragma: no cover - environment dependent
        print(f"Warning: could not write {dst_path}: {exc}")
        return False

for icns_dir in (os.path.join(PROJECT_ROOT, "assets"), icons_dir):
    write_icns(os.path.join(icns_dir, "icon.icns"))

print(f"Icons generated: {icon_png_path}, {ico_path}")
print(f"Also copied to: {icons_dir}/")
