"""
Generate a Stargate app icon using the exact same SVG logo as in the app.
Uses the same paths from src/components/icon.rs (IconName::Logo).
Rendered with blue-400 (#60a5fa) on a dark slate background.
"""
import cairosvg
from PIL import Image
import io
import os

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The exact same SVG as used in the app (src/components/icon.rs IconName::Logo)
# Color: text-blue-400 = #60a5fa, background: slate-900 = #0f172a
SVG = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" width="1024" height="1024">
  <!-- Background -->
  <rect x="0" y="0" width="24" height="24" rx="3.5" fill="#0f172a"/>

  <!-- Outer ring -->
  <circle cx="12" cy="12" r="10" stroke="#60a5fa" stroke-width="1.5" fill="none"/>

  <!-- Inner ring -->
  <circle cx="12" cy="12" r="6.5" stroke="#60a5fa" stroke-width="1" fill="none"/>

  <!-- 9 Chevron marks around the gate -->
  <path d="M12 2.5L13.2 4.5H10.8L12 2.5Z" fill="#60a5fa"/>
  <path d="M17.5 4.5L17 6.8L15.2 5.5L17.5 4.5Z" fill="#60a5fa"/>
  <path d="M20.5 9.5L18.2 9.8L18.8 7.8L20.5 9.5Z" fill="#60a5fa"/>
  <path d="M20.5 14.5L18.8 16.2L18.2 14.2L20.5 14.5Z" fill="#60a5fa"/>
  <path d="M17.5 19.5L15.2 18.5L17 17.2L17.5 19.5Z" fill="#60a5fa"/>
  <path d="M6.5 19.5L7 17.2L8.8 18.5L6.5 19.5Z" fill="#60a5fa"/>
  <path d="M3.5 14.5L5.8 14.2L5.2 16.2L3.5 14.5Z" fill="#60a5fa"/>
  <path d="M3.5 9.5L5.2 7.8L5.8 9.8L3.5 9.5Z" fill="#60a5fa"/>
  <path d="M6.5 4.5L8.8 5.5L7 6.8L6.5 4.5Z" fill="#60a5fa"/>

  <!-- Center glow dot -->
  <circle cx="12" cy="12" r="2" fill="#60a5fa" opacity="0.6"/>
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

print(f"Icons generated: {icon_png_path}, {ico_path}")
print(f"Also copied to: {icons_dir}/")
