#!/bin/bash
set -e
echo "[+] 2/3 Создание файла автоматизации .github/workflows/build.yml..."
mkdir -p .github/workflows

cat << 'GHACTION' > .github/workflows/build.yml
name: Кроссплатформенная сборка мессенджера
on: [push]
jobs:
  build-desktop:
    name: Сборка под ${{ matrix.target-os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target-os: Ubuntu (Linux)
            os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            cmd: cargo build --release
            out-path: target/release/covert_messenger
          - target-os: Windows
            os: ubuntu-latest
            target: x86_64-pc-windows-gnu
            cmd: |
              sudo apt-get update && sudo apt-get install -y mingw-w64
              rustup target add x86_64-pc-windows-gnu
              cargo build --release --target x86_64-pc-windows-gnu
            out-path: target/x86_64-pc-windows-gnu/release/covert_messenger.exe
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: ${{ matrix.cmd }}
      - uses: actions/upload-artifact@v4
        with:
          name: covert_messenger_${{ matrix.target-os }}
          path: ${{ matrix.out-path }}
  build-android:
    name: Сборка под Android (.apk)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android
      - run: |
          cargo install cargo-apk
          cargo apk build --release
      - uses: actions/upload-artifact@v4
        with:
          name: covert_messenger_Android
          path: target/debug/apk/*.apk
GHACTION

echo "[✓] Шаг 2/3 завершен: Пайплайн компиляции настроен."
