#!/bin/bash

echo "🚀 Starting SecurePass Development Environment"
echo "============================================"

# 检查依赖
echo "📦 Checking dependencies..."

# 检查Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# 检查Node.js
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js from https://nodejs.org/"
    exit 1
fi

# 检查npm
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed. Please install npm from https://nodejs.org/"
    exit 1
fi

# 安装前端依赖
echo "📦 Installing frontend dependencies..."
cd frontend
npm install
cd ..

# 创建必要的目录
echo "📁 Creating necessary directories..."
mkdir -p icons
mkdir -p assets

# 创建简单的图标
echo "🎨 Creating placeholder icons..."
convert -size 32x32 xc:blue icons/32x32.png
convert -size 128x128 xc:blue icons/128x128.png
convert -size 256x256 xc:blue icons/256x256.png

echo "✅ Development environment setup complete!"
echo ""
echo "🔧 Available commands:"
echo "  npm run dev    - Start development server"
echo "  npm run build  - Build for production"
echo "  npm run tauri:dev    - Start Tauri development"
echo "  npm run tauri:build  - Build Tauri application"
echo ""
echo "🎯 Quick test commands:"
echo "  cargo test --bin securepass_cli --no-default-features   - 核心单元测试（无头环境）"
echo "  cargo tauri dev   - 启动 Tauri 桌面应用（需 webkit2gtk-4.1 / libdbus-1-dev）"
echo ""
echo "🌐 Frontend will be available at: http://localhost:3000"
echo "🖥️  Tauri app will be available once started"