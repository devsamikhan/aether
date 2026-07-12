#!/bin/bash
# Register .aether file association on Linux/macOS

echo "Registering .aether file association..."

# Create MIME type definition
mkdir -p ~/.local/share/mime/packages
cat > ~/.local/share/mime/packages/aether.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="text/x-aether">
    <comment>AETHER Source File</comment>
    <glob pattern="*.aether"/>
    <icon name="aether-file-icon"/>
  </mime-type>
</mime-info>
EOF

# Update MIME database
update-mime-database ~/.local/share/mime 2>/dev/null || true

# Copy icon to icon theme
mkdir -p ~/.local/share/icons/hicolor/256x256/mimetypes
cp logo/aether-file-icon.png ~/.local/share/icons/hicolor/256x256/mimetypes/text-x-aether.png 2>/dev/null || true

# Update icon cache
gtk-update-icon-cache ~/.local/share/icons/hicolor 2>/dev/null || true

echo "✅ File association registered!"
echo "Note: You may need to restart your file manager or log out/in for changes to take effect."
