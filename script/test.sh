#!/usr/bin/env bash
set -e

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

cd "$(dirname "${BASH_SOURCE[0]}")/.."

try_build_dev

${CUBE_DOCKER} run --rm \
    -v "$(pwd):/code" \
    -e VCPKGRS_DYNAMIC=0 \
    -e OPENCV_LINKAGE=static \
    "${CUBE_IMAGE_DEV}" ./script/rust/test.sh
