#!/usr/bin/env bash
set -e

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

cd "$(dirname "${BASH_SOURCE[0]}")/.."

try_build_dev

# VCPKGRS_DYNAMIC=0 forces static linking for vcpkg dependencies
# OPENCV_LINKAGE=static tells the opencv crate to link statically
${FOUNTAIN_DOCKER} run --rm \
    -v "$(pwd):/code" \
    -e VCPKGRS_DYNAMIC=0 \
    -e OPENCV_LINKAGE=static \
    "${FOUNTAIN_IMAGE_DEV}" ./script/rust/compile.sh
