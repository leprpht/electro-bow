#!/usr/bin/env sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
juce_version=${JUCE_VERSION:-9.0.2}
juce_dir=${JUCE_PATH:-$project_dir/JUCE}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "setup: required command '$1' was not found in PATH." >&2
        exit 1
    fi
    echo "setup: found $1 ($(command -v "$1"))"
}

require_any_command() {
    for command_name in "$@"; do
        if command -v "$command_name" >/dev/null 2>&1; then
            echo "setup: found $command_name ($(command -v "$command_name"))"
            return 0
        fi
    done
    echo "setup: none of these commands was found in PATH: $*" >&2
    exit 1
}

if ! command -v clang-format >/dev/null 2>&1; then
    for llvm_bin in /opt/homebrew/opt/llvm/bin /usr/local/opt/llvm/bin; do
        if [ -x "$llvm_bin/clang-format" ]; then
            PATH="$llvm_bin:$PATH"
            export PATH
            break
        fi
    done
fi

require_command git
require_command node
require_command npm
require_command python3
require_command cmake
require_any_command clang++ c++ g++
require_command clang-format

if [ ! -f "$juce_dir/CMakeLists.txt" ]; then
    require_command curl
    require_command tar
    archive=$(mktemp "${TMPDIR:-/tmp}/juce.XXXXXX.tar.gz")
    extract_dir=$(mktemp -d "${TMPDIR:-/tmp}/juce-extract.XXXXXX")
    trap 'rm -f "$archive"; rm -rf "$extract_dir"' EXIT HUP INT TERM

    echo "Downloading JUCE $juce_version..."
    curl --fail --location --silent --show-error \
        "https://github.com/juce-framework/JUCE/archive/refs/tags/$juce_version.tar.gz" \
        --output "$archive"
    tar -xzf "$archive" -C "$extract_dir"
    extracted_dir="$extract_dir/JUCE-$juce_version"
    if [ ! -f "$extracted_dir/CMakeLists.txt" ]; then
        echo "setup: downloaded JUCE archive has an unexpected layout." >&2
        exit 1
    fi
    if [ -e "$juce_dir" ]; then
        echo "setup: JUCE_PATH exists but is not a JUCE checkout: $juce_dir" >&2
        echo "Remove it or set JUCE_PATH to another location, then retry." >&2
        exit 1
    fi
    mkdir -p "$(dirname "$juce_dir")"
    mv "$extracted_dir" "$juce_dir"
    echo "setup: downloaded JUCE $juce_version to $juce_dir"
else
    echo "setup: found JUCE checkout at $juce_dir"
fi

echo "Installing UI dependencies..."
npm ci --prefix "$project_dir/UI"

echo "Enabling repository Git hooks..."
git -C "$project_dir" config core.hooksPath .githooks

echo "Setup complete. JUCE: $juce_dir"
echo "Run './build.py \"$juce_dir\"' to configure and build the plugin."
