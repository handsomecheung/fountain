#!/usr/bin/env bash
set -e

DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

export CUBE_DOCKER="${DOCKER:-docker}"
export CUBE_IMAGE_DEV=cube-rust-dev
export CUBE_CONTAINER_CAMERA_WEB=cube-decode-camera-web

try_build_dev() {
    if [[ "$(${CUBE_DOCKER} images -q ${CUBE_IMAGE_DEV} 2>/dev/null)" == "" ]]; then
        echo "not builder image, building it ..."
        # This build will take significantly longer due to vcpkg compiling OpenCV
        cd "${DIR}"
        ${CUBE_DOCKER} build -t ${CUBE_IMAGE_DEV} .
        cd -
    fi
}
