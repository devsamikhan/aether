import math
from PIL import Image, ImageDraw, ImageFilter

def create_modern_aether_logo():
    # Supersampling canvas: 2048 x 2048 for pristine anti-aliasing
    SIZE = 2048
    canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    
    # We will draw a breathtaking modern "Æ" monogram with radiant gradients
    # Background soft neon ambient glow
    glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw_glow = ImageDraw.Draw(glow)
    
    # Outer circular glow
    center = SIZE // 2
    r_glow = int(SIZE * 0.42)
    for r in range(r_glow, r_glow - 300, -10):
        alpha = int(25 * (1.0 - (r_glow - r) / 300.0))
        draw_glow.ellipse(
            [center - r, center - r, center + r, center + r],
            outline=(139, 92, 246, alpha),
            width=12
        )
    
    glow = glow.filter(ImageFilter.GaussianBlur(radius=40))
    canvas = Image.alpha_composite(canvas, glow)

    # 1. Background Shield / Hexagonal Squircle
    card = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw_card = ImageDraw.Draw(card)
    
    # Rounded squircle base
    pad = 140
    # Draw deep obsidian badge
    corner_radius = 280
    draw_card.rounded_rectangle(
        [pad, pad, SIZE - pad, SIZE - pad],
        radius=corner_radius,
        fill=(11, 17, 32, 240), # #0b1120 deep space obsidian
        outline=(139, 92, 246, 180), # Neon violet border
        width=16
    )
    
    # Subtle inner border for glassy depth
    draw_card.rounded_rectangle(
        [pad + 24, pad + 24, SIZE - pad - 24, SIZE - pad - 24],
        radius=corner_radius - 20,
        outline=(34, 211, 238, 70), # Subtle cyan inner rim
        width=6
    )
    canvas = Image.alpha_composite(canvas, card)

    # 2. Draw the Modern "Æ" Monogram Glyph with Multi-stop Gradient
    glyph_layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw_glyph = ImageDraw.Draw(glyph_layer)

    # Coordinate mapping for bold, geometric "Æ" (fusion of A and E)
    # Apex of A: (760, 480)
    # Left base of A: (480, 1540)
    # Right base of A: (980, 1540)
    # Connecting crossbar and E bars:
    
    # Geometric polygon for Left Leg of A
    left_leg = [
        (920, 460),    # Apex top
        (1040, 460),   # Apex right
        (620, 1540),   # Bottom left outer
        (460, 1540),   # Bottom left tip
    ]
    
    # Geometric polygon for Right Leg / Central Spine of Æ
    center_spine = [
        (980, 460),
        (1140, 460),
        (1140, 1540),
        (980, 1540),
    ]

    # Three horizontal wings of 'E'
    # Top bar
    top_bar = [
        (1080, 460),
        (1600, 460),
        (1500, 620),
        (1080, 620),
    ]

    # Middle bar (Crossbar connecting A and E)
    mid_bar = [
        (680, 940),
        (1480, 940),
        (1400, 1100),
        (740, 1100),
    ]

    # Bottom bar
    bot_bar = [
        (1080, 1380),
        (1600, 1380),
        (1500, 1540),
        (1080, 1540),
    ]

    # Central triangular negative space of A
    # A's inner triangle:
    inner_triangle = [
        (880, 680),
        (760, 940),
        (1000, 940)
    ]

    # Draw strokes into a mask for gradient painting
    mask = Image.new("L", (SIZE, SIZE), 0)
    draw_mask = ImageDraw.Draw(mask)

    draw_mask.polygon(left_leg, fill=255)
    draw_mask.polygon(center_spine, fill=255)
    draw_mask.polygon(top_bar, fill=255)
    draw_mask.polygon(mid_bar, fill=255)
    draw_mask.polygon(bot_bar, fill=255)
    draw_mask.polygon(inner_triangle, fill=0) # Cutout counter-form

    # Create Radiant Linear Gradient from Neon Cyan (#22d3ee) to Electric Violet (#8b5cf6)
    gradient = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    for y in range(SIZE):
        t = y / float(SIZE)
        # Interpolate between Cyan (34, 211, 238) at top and Violet (139, 92, 246) at bottom
        r = int(34 * (1 - t) + 168 * t)
        g = int(211 * (1 - t) + 85 * t)
        b = int(238 * (1 - t) + 247 * t)
        draw_glyph.line([(0, y), (SIZE, y)], fill=(r, g, b, 255))
    
    # Apply mask
    gradient.putalpha(mask)

    # Add soft inner shadow/glow to glyph
    canvas = Image.alpha_composite(canvas, gradient)

    # 3. Add accent energy dots / vertices (Speed Nodes)
    nodes = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw_nodes = ImageDraw.Draw(nodes)
    
    # Apex node
    draw_nodes.ellipse([950, 430, 1010, 490], fill=(255, 255, 255, 255))
    # E top wing tip node
    draw_nodes.ellipse([1560, 430, 1620, 490], fill=(34, 211, 238, 255))
    # E mid wing tip node
    draw_nodes.ellipse([1440, 910, 1500, 970], fill=(168, 85, 247, 255))
    # E bot wing tip node
    draw_nodes.ellipse([1560, 1350, 1620, 1410], fill=(224, 231, 255, 255))

    nodes_glow = nodes.filter(ImageFilter.GaussianBlur(radius=15))
    canvas = Image.alpha_composite(canvas, nodes_glow)
    canvas = Image.alpha_composite(canvas, nodes)

    # Downsample with highest quality Lanczos filter to target resolutions
    sizes = [
        (512, "logo/aether-logo.png"),
        (256, "logo/aether-logo-256.png"),
        (128, "logo/aether-logo-128.png"),
        (64,  "logo/aether-logo-64.png"),
        (32,  "logo/aether-logo-32.png")
    ]

    for dim, path in sizes:
        resized = canvas.resize((dim, dim), Image.Resampling.LANCZOS)
        resized.save(path, format="PNG")
        print(f"Generated PNG: {path} ({dim}x{dim})")

    # Generate multi-resolution ICO file
    ico_img = canvas.resize((256, 256), Image.Resampling.LANCZOS)
    ico_img.save(
        "logo/aether-logo.ico",
        format="ICO",
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    )
    print("Generated Multi-Resolution ICO: logo/aether-logo.ico")

    # Also generate the pristine, matching SVG vector
    svg_content = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Background Badge Gradient -->
    <linearGradient id="cardGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0b1120" stop-opacity="0.95"/>
      <stop offset="100%" stop-color="#060913" stop-opacity="0.98"/>
    </linearGradient>

    <!-- Primary Radiant Energy Gradient: Neon Cyan to Electric Violet -->
    <linearGradient id="aeGradient" x1="15%" y1="0%" x2="85%" y2="100%">
      <stop offset="0%" stop-color="#22d3ee"/>
      <stop offset="50%" stop-color="#818cf8"/>
      <stop offset="100%" stop-color="#a855f7"/>
    </linearGradient>

    <!-- Subtle Glow Filter -->
    <filter id="aeGlow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="8" result="blur"/>
      <feComposite in="SourceGraphic" in2="blur" operator="over"/>
    </filter>
  </defs>

  <!-- Squircle Base Container -->
  <rect x="35" y="35" width="442" height="442" rx="70" ry="70" 
        fill="url(#cardGrad)" stroke="#8b5cf6" stroke-width="3.5" stroke-opacity="0.45"/>
  <rect x="41" y="41" width="430" height="430" rx="64" ry="64" 
        fill="none" stroke="#22d3ee" stroke-width="1.5" stroke-opacity="0.2"/>

  <!-- Modern Geometric "Æ" Monogram -->
  <g filter="url(#aeGlow)">
    <!-- Left Angled Leg of A -->
    <polygon points="230,115 260,115 155,385 115,385" fill="url(#aeGradient)"/>
    
    <!-- Central Vertical Spine -->
    <rect x="245" y="115" width="40" height="270" fill="url(#aeGradient)"/>

    <!-- E Top Wing -->
    <polygon points="270,115 400,115 375,155 270,155" fill="url(#aeGradient)"/>

    <!-- A-E Unified Crossbar -->
    <polygon points="170,235 370,235 350,275 185,275" fill="url(#aeGradient)"/>

    <!-- E Bottom Wing -->
    <polygon points="270,345 400,345 375,385 270,385" fill="url(#aeGradient)"/>

    <!-- Inner Triangular Negative Space -->
    <polygon points="220,170 190,235 250,235" fill="#0b1120"/>
    
    <!-- Accent Energy Nodes -->
    <circle cx="245" cy="115" r="7" fill="#ffffff"/>
    <circle cx="395" cy="115" r="5" fill="#22d3ee"/>
    <circle cx="365" cy="235" r="5" fill="#818cf8"/>
    <circle cx="395" cy="345" r="5" fill="#a855f7"/>
  </g>
</svg>
'''
    with open("logo/aether-logo.svg", "w", encoding="utf-8") as f:
        f.write(svg_content)
    print("Generated SVG: logo/aether-logo.svg")

if __name__ == "__main__":
    create_modern_aether_logo()
