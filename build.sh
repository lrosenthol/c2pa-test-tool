#!/bin/bash
# Build script for crTool workspace

set -e  # Exit on error

echo "================================"
echo "Building crTool Workspace"
echo "================================"
echo ""

# Parse command line arguments
BUILD_CLI=true
BUILD_GUI=false
RELEASE_MODE=""
MAC_APP=false

for arg in "$@"; do
    case $arg in
        --gui-only)
            BUILD_CLI=false
            BUILD_GUI=true
            shift
            ;;
        --cli-only)
            BUILD_CLI=true
            BUILD_GUI=false
            shift
            ;;
        --all)
            BUILD_CLI=true
            BUILD_GUI=true
            shift
            ;;
        --release)
            RELEASE_MODE="--release"
            shift
            ;;
        --mac-app)
            BUILD_CLI=false
            BUILD_GUI=true
            RELEASE_MODE="--release"
            MAC_APP=true
            shift
            ;;
        --help)
            echo "Usage: ./build.sh [options]"
            echo ""
            echo "Options:"
            echo "  --cli-only    Build only the CLI tool (default)"
            echo "  --gui-only    Build only the GUI tool"
            echo "  --all         Build both CLI and GUI"
            echo "  --release     Build in release mode"
            echo "  --mac-app     Build GUI in release and create macOS crTool.app bundle (macOS only)"
            echo "  --help        Show this help message"
            echo ""
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Show build configuration
if [ "$BUILD_CLI" = true ] && [ "$BUILD_GUI" = true ]; then
    echo "Building: CLI + GUI"
elif [ "$BUILD_CLI" = true ]; then
    echo "Building: CLI only"
else
    echo "Building: GUI only"
fi

if [ -n "$RELEASE_MODE" ]; then
    echo "Mode: Release"
else
    echo "Mode: Debug"
fi
echo ""

# Build CLI
if [ "$BUILD_CLI" = true ]; then
    echo "→ Building CLI tool..."
    cargo build $RELEASE_MODE -p crTool
    echo "✓ CLI build complete"
    echo ""
fi

# Build GUI
if [ "$BUILD_GUI" = true ]; then
    echo "→ Building GUI tool..."
    cargo build $RELEASE_MODE -p crTool-gui
    echo "✓ GUI build complete"
    echo ""
fi

# Create macOS .app bundle (macOS only)
if [ "$MAC_APP" = true ]; then
    if [ "$(uname)" != "Darwin" ]; then
        echo "Error: --mac-app is only supported on macOS."
        exit 1
    fi
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    APP_NAME="crTool.app"
    APP_DIR="$SCRIPT_DIR/$APP_NAME"
    CONTENTS="$APP_DIR/Contents"
    MACOS_DIR="$CONTENTS/MacOS"
    RESOURCES_DIR="$CONTENTS/Resources"
    RELEASE_BIN="$SCRIPT_DIR/target/release/crTool-gui"
    INFO_PLIST_SRC="$SCRIPT_DIR/crtool-gui/macos/Info.plist"
    ICON_SRC="$SCRIPT_DIR/crtool-gui/macos/AppIcon.icns"

    echo "→ Creating macOS app bundle: $APP_NAME"
    rm -rf "$APP_DIR"
    mkdir -p "$MACOS_DIR"
    mkdir -p "$RESOURCES_DIR"

    cp "$RELEASE_BIN" "$MACOS_DIR/crTool"
    chmod +x "$MACOS_DIR/crTool"
    cp "$INFO_PLIST_SRC" "$CONTENTS/Info.plist"
    if [ -f "$ICON_SRC" ]; then
        cp "$ICON_SRC" "$RESOURCES_DIR/AppIcon.icns"
    fi

    ENTITLEMENTS_SRC="$SCRIPT_DIR/crtool-gui/macos/crTool.entitlements"
    if [ -f "$ENTITLEMENTS_SRC" ]; then
        cp "$ENTITLEMENTS_SRC" "$CONTENTS/entitlements.plist"
    fi

    echo "✓ $APP_NAME created at $APP_DIR"
    echo ""
fi

# Show output locations
echo "================================"
echo "Build Complete!"
echo "================================"
echo ""

if [ -n "$RELEASE_MODE" ]; then
    BUILD_DIR="target/release"
else
    BUILD_DIR="target/debug"
fi

if [ "$BUILD_CLI" = true ]; then
    echo "CLI binary: $BUILD_DIR/crTool"
fi

if [ "$BUILD_GUI" = true ]; then
    echo "GUI binary: $BUILD_DIR/crTool-gui"
fi
if [ "$MAC_APP" = true ]; then
    echo "macOS app:   crTool.app"
fi

echo ""
echo "To run the CLI: ./$BUILD_DIR/crTool --help"
if [ "$BUILD_GUI" = true ]; then
    echo "To run the GUI: ./$BUILD_DIR/crTool-gui"
fi
if [ "$MAC_APP" = true ]; then
    echo "To run the app: open crTool.app"
fi
