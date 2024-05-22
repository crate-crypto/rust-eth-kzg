#!/bin/bash


# Function to display usage information
usage() {
    echo "Usage: $0 [OS] [ARCH] [LIB_NAME] [LIB_TYPE] [OUT_DIR]"
    echo "Compile the project for the specified OS, architecture, library name, library type, and output directory."
    echo "If no OS and ARCH are provided, it defaults to the current system's OS and architecture."
    echo "If no LIB_NAME is provided, it defaults to 'c_peerdas_kzg'."
    echo "If no LIB_TYPE is provided, it defaults to 'both'."
    echo "If no OUT_DIR is provided, it defaults to './bindings/c/build'."
    echo
    echo "Arguments:"
    echo "  OS        Operating system (e.g., Linux, Darwin, MINGW64_NT)"
    echo "  ARCH      Architecture (e.g., x86_64, arm64)"
    echo "  LIB_NAME  Library name (e.g., c_peerdas_kzg)"
    echo "  LIB_TYPE  Library type to copy (static, dynamic, or both)"
    echo "  OUT_DIR   Output directory for the compiled libraries"
    echo
    echo "Examples:"
    echo "  $0                                        # Uses the system's OS and architecture, copies both libraries to the default directory with the default library name"
    echo "  $0 Linux x86_64 my_lib static             # Compiles for Linux on x86_64 and copies only static libraries to the default directory with the library name 'my_lib'"
    echo "  $0 Darwin arm64 my_lib dynamic ./out/dir  # Compiles for macOS on ARM (Apple Silicon) and copies only dynamic libraries to './out/dir' with the library name 'my_lib'"
    exit 1
}


# Check for help flag
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    usage
fi

# Determine the operating system, architecture, library name, library type, and output directory if not provided
OS="${1:-$(uname)}"
ARCH="${2:-$(uname -m)}"
LIB_NAME="${3:-c_peerdas_kzg}"
LIB_TYPE="${4:-both}"
OUT_DIR="${5:-./bindings/c/build}"
echo "Detected/Provided OS: $OS"
echo "Detected/Provided architecture: $ARCH"
echo "Library name: $LIB_NAME"
echo "Library type to copy: $LIB_TYPE"
echo "Output directory: $OUT_DIR"

STATIC_LIB_NAME=""
DYNAMIC_LIB_NAME=""
TARGET_NAME=""

# Check for Windows OS and ensure ARCH is x86_64
# We don't support 32-bit Windows builds -- nothing technical
# just a simplification to avoid dealing with 32-bit builds
# plus languages like java want you to package all possible
# dlls for all possible architectures which means we are saving 
# some space by not supporting 32-bit builds
if [[ "$OS" == "MINGW64_NT" || "$OS" == "CYGWIN_NT" ]]; then
    if [[ "$ARCH" != "x86_64" ]]; then
        echo "Error: On Windows, the architecture must be x86_64."
        exit 1
    fi
fi

case "$OS" in
    "Darwin")
        case "$ARCH" in
            "arm64")
                # Copy static and shared libraries for macOS ARM
                TARGET_NAME="aarch64-apple-darwin"
                STATIC_LIB_NAME="lib${LIB_NAME}.a"
                DYNAMIC_LIB_NAME="lib${LIB_NAME}.dylib"
                ;;
            "x86_64")
                # Copy static and shared libraries for macOS Intel
                TARGET_NAME="x86_64-apple-darwin"
                STATIC_LIB_NAME="lib${LIB_NAME}.a"
                DYNAMIC_LIB_NAME="lib${LIB_NAME}.dylib"
                ;;
            *)
                echo "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    "Linux")
        case "$ARCH" in
            "arm64")
                # Copy static and shared libraries for Linux ARM
                TARGET_NAME="aarch64-unknown-linux-gnu"
                STATIC_LIB_NAME="lib${LIB_NAME}.a"
                DYNAMIC_LIB_NAME="lib${LIB_NAME}.so"
                ;;
            "x86_64")
                # Copy static and shared libraries for Linux Intel
                TARGET_NAME="x86_64-unknown-linux-gnu"
                STATIC_LIB_NAME="lib${LIB_NAME}.a"
                DYNAMIC_LIB_NAME="lib${LIB_NAME}.so"
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    "MINGW64_NT"|"CYGWIN_NT"|"Windows")
        TARGET_NAME="x86_64-pc-windows-gnu"
        STATIC_LIB_NAME="lib${LIB_NAME}.a"
        DYNAMIC_LIB_NAME="${LIB_NAME}.dll"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Compiling for target: $TARGET_NAME"
./scripts/check_if_rustup_target_installed.sh $TARGET_NAME

# Check the exit code 
if [ $? -eq 0 ]; then
  echo "The default Rust target is installed."
else
  echo "The default Rust target is not installed."
  exit 1
fi

cargo build --release --target=$TARGET_NAME

# Create the output directory if it doesn't exist
mkdir -p "$OUT_DIR/$TARGET_NAME"

# Copy the libraries to the specified output directory
if [ "$LIB_TYPE" == "static" ] || [ "$LIB_TYPE" == "both" ]; then
    cp -R target/$TARGET_NAME/release/$STATIC_LIB_NAME "$OUT_DIR/$TARGET_NAME/"
fi

if [ "$LIB_TYPE" == "dynamic" ] || [ "$LIB_TYPE" == "both" ]; then
    cp -R target/$TARGET_NAME/release/$DYNAMIC_LIB_NAME "$OUT_DIR/$TARGET_NAME/"
fi