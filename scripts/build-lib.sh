#!/bin/bash
set -e

echo "🔨 Building TomChat Core Library"
echo "================================"

# Create output directory
mkdir -p dist/lib
mkdir -p dist/include

# Build the shared library
echo "📦 Building shared library..."
LIBCLANG_PATH="/usr/lib/x86_64-linux-gnu" cargo build --release --lib

# Copy library files
echo "📋 Copying library files..."
cp target/release/libtomchat_core.so dist/lib/
cp target/release/libtomchat_core.rlib dist/lib/

# Copy header file
cp include/tomchat.h dist/include/

# Generate pkg-config file
echo "🔧 Generating pkg-config file..."
cat > dist/lib/tomchat.pc << EOF
prefix=\${pcfiledir}/../..
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: TomChat Core
Description: Speech-to-text engine library
Version: 0.1.0
Libs: -L\${libdir} -ltomchat_core
Cflags: -I\${includedir}
EOF

echo ""
echo "✅ Build complete!"
echo "Library: dist/lib/libtomchat_core.so"
echo "Header:  dist/include/tomchat.h"
echo "Pkg-config: dist/lib/tomchat.pc"
echo ""
echo "🚀 Ready for app integration!"