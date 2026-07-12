#!/bin/bash
# This script converts SVG logos to PNG
# Requires: ImageMagick (install with: brew install imagemagick or apt install imagemagick)

echo "Converting SVG logos to PNG..."

# Check if ImageMagick is installed
if ! command -v convert &> /dev/null; then
    echo "Error: ImageMagick not found. Please install it first:"
    echo "  macOS: brew install imagemagick"
    echo "  Ubuntu: sudo apt install imagemagick"
    echo "  Windows: choco install imagemagick"
    exit 1
fi

# Convert main logo
convert -background none -size 512x512 logo/aether-logo.svg logo/aether-logo.png
convert -background none -size 256x256 logo/aether-logo.svg logo/aether-logo-256.png
convert -background none -size 128x128 logo/aether-logo.svg logo/aether-logo-128.png
convert -background none -size 64x64 logo/aether-logo.svg logo/aether-logo-64.png
convert -background none -size 32x32 logo/aether-logo.svg logo/aether-logo-32.png

# Convert file icon
convert -background none -size 256x320 logo/aether-file-icon.svg logo/aether-file-icon.png
convert -background none -size 128x160 logo/aether-file-icon.svg logo/aether-file-icon-128.png
convert -background none -size 64x80 logo/aether-file-icon.svg logo/aether-file-icon-64.png

echo "✅ PNG logos generated successfully!"
ls -lh logo/*.png
