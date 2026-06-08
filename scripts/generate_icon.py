"""
Generate the Stargate app icon (PNG / ICO / ICNS) from assets/icon.svg.

assets/icon.svg is the single source of truth for the icon's colors and shape.
This script renders it to the raster formats the platform bundles need and
writes them to both assets/ and icons/ (the latter is what Dioxus.toml points
at for `dx bundle`). Re-run this whenever icon.svg changes.
"""
import cairosvg
from PIL import Image
import io
import os

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Render the canonical icon SVG to PNG at 1024x1024.
icon_svg_path = os.path.join(PROJECT_ROOT, "assets", "icon.svg")
png_data = cairosvg.svg2png(url=icon_svg_path, output_width=1024, output_height=1024)

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
