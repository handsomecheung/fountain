FROM rust:1.91.1

# Install system build dependencies
# vcpkg requires git, curl, zip, unzip, tar, and standard build tools
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    git \
    curl \
    zip \
    unzip \
    tar \
    bison \
    flex \
    ninja-build \
    linux-libc-dev \
    libx11-dev \
    libxext-dev \
    libxrender-dev \
    libxrandr-dev \
    libxinerama-dev \
    libxcursor-dev \
    libxi-dev \
    libxtst-dev \
    libgl1-mesa-dev \
    libglu1-mesa-dev \
    autoconf \
    autoconf-archive \
    automake \
    libtool \
    nasm \
    yasm \
    python3-venv \
    python3-dev \
    clang \
    libclang-dev \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

# Set up vcpkg
ENV VCPKG_ROOT=/opt/vcpkg
RUN git clone https://github.com/microsoft/vcpkg.git $VCPKG_ROOT && \
    $VCPKG_ROOT/bootstrap-vcpkg.sh

# Install OpenCV via vcpkg with static linking
# This will take a while as it compiles OpenCV and all dependencies from source
# I select 'x64-linux' which defaults to static libraries in modern vcpkg for linux dynamic linkage,
# BUT to ensure static linkage into the binary, I need to handle the triplet carefully.
# However, standard x64-linux in vcpkg produces .a files for libraries usually.
# Let's verify and use a custom triplet or configuration if needed.
# Actually, simply installing `opencv` with proper env vars for the rust crate usually works if pkg-config finds the static libs.
# Better: use the specific static triplet if available or configure standard one.
# For simplicity and robustness, I rely on vcpkg's default behavior but ensure I link statically in Rust.

# Installing minimal opencv features to save build time
RUN $VCPKG_ROOT/vcpkg install "opencv[png,jpeg,tiff,webp,ffmpeg]:x64-linux" --recurse

WORKDIR /code
