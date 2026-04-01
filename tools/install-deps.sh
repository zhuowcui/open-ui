#!/bin/bash
# Install system dependencies for building Open UI on Ubuntu 22.04+
set -euo pipefail

echo "==> Installing build dependencies..."

sudo apt-get update
sudo apt-get install -y \
  build-essential \
  clang \
  lld \
  ninja-build \
  python3 \
  git \
  curl \
  pkg-config \
  gperf \
  bison \
  flex

echo "==> Installing graphics/display dependencies..."

sudo apt-get install -y \
  libfontconfig-dev \
  libfreetype-dev \
  libvulkan-dev \
  vulkan-tools \
  libegl-dev \
  libgl-dev \
  libdrm-dev \
  libgbm-dev

echo "==> Installing windowing dependencies..."

sudo apt-get install -y \
  libx11-xcb-dev \
  libxcb1-dev \
  libxcb-shm0-dev \
  libwayland-dev \
  wayland-protocols

echo "==> Installing library dependencies..."

sudo apt-get install -y \
  libglib2.0-dev \
  libpng-dev \
  libjpeg-dev \
  zlib1g-dev

echo "==> All dependencies installed."
