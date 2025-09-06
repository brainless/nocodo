#!/bin/bash
set -e

echo "🧪 Testing Self-contained Manager Build"
echo "======================================"

# Check if we're in the right directory
if [ ! -f "manager/Cargo.toml" ] || [ ! -f "manager-web/package.json" ]; then
    echo "❌ Error: Run this script from the project root directory"
    exit 1
fi

echo "📦 Step 1: Building manager-web assets..."
cd manager-web

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "📥 Installing Node.js dependencies..."
    npm ci
fi

# Build the web app
echo "🏗️  Building web application..."
npm run build

# Verify build output
if [ ! -d "dist" ]; then
    echo "❌ Error: manager-web build failed - dist directory not found"
    exit 1
fi

echo "✅ Web build completed. Assets in manager-web/dist:"
ls -la dist/

cd ..

echo "🦀 Step 2: Building Rust manager with embedded assets..."
cd manager

# Build the manager binary
echo "🔨 Compiling manager binary..."
cargo build --release

# Check if binary was created
if [ ! -f "target/release/nocodo-manager" ]; then
    echo "❌ Error: Manager binary not found"
    exit 1
fi

echo "✅ Manager binary built successfully:"
ls -lh target/release/nocodo-manager

# Run basic validation tests
echo "🔬 Step 3: Running validation tests..."
cargo test embedded_assets_exist
cargo test asset_validation

echo "🚀 Step 4: Testing binary startup (5 second timeout)..."
cd ..

# Test binary startup with a timeout
timeout 5 ./manager/target/release/nocodo-manager --no-browser || {
    exit_code=$?
    if [ $exit_code -eq 124 ]; then
        echo "✅ Binary started successfully (timed out as expected)"
    else
        echo "❌ Binary failed to start (exit code: $exit_code)"
        exit 1
    fi
}

echo ""
echo "🎉 Self-contained manager build test completed successfully!"
echo ""
echo "📋 Summary:"
echo "   📁 Web assets embedded in binary"
echo "   🔗 Binary size: $(du -h manager/target/release/nocodo-manager | cut -f1)"
echo "   🌐 Browser auto-launch available"
echo "   📊 Ready for distribution"
echo ""
echo "💡 Usage:"
echo "   ./manager/target/release/nocodo-manager          # Auto-launch browser"  
echo "   ./manager/target/release/nocodo-manager --no-browser  # Manual browser"
echo ""