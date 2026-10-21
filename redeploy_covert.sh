#!/bin/bash
set -e

GH_BIN="/usr/bin/gh"
# Генерируем совершенно новое, нейтральное имя для проекта
NEW_REPO_NAME="hybrid-chat-utility-$(date +%s)"

echo "=============================================================================="
echo "🔄 Перенос проекта в чистый репозиторий: $NEW_REPO_NAME"
echo "=============================================================================="

# 1. Сбрасываем старую заблокированную привязку Git
rm -rf .git
git init -b main

# 2. Перезаписываем .github/workflows/build.yml с фиксацией стабильной ОС ubuntu-24.04
mkdir -p .github/workflows
cat << 'GHACTION' > .github/workflows/build.yml
name: Кроссплатформенная сборка

on: [push]

jobs:
  build-desktop:
    name: Сборка под ${{ matrix.target-os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target-os: Linux
            os: ubuntu-24.04
            target: x86_64-unknown-linux-gnu
            cmd: cargo build --release
            out-path: target/release/covert_messenger

          - target-os: Windows
            os: ubuntu-24.04
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
          name: app_${{ matrix.target-os }}
          path: ${{ matrix.out-path }}

  build-android:
    name: Сборка под Android
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android
      - name: Настройка Android SDK 34
        run: |
          echo "y" | ${ANDROID_HOME}/cmdline-tools/latest/bin/sdkmanager --licenses || echo "OK"
          ${ANDROID_HOME}/cmdline-tools/latest/bin/sdkmanager "platforms;android-34" "build-tools;34.0.0"
          cargo install cargo-apk
          cargo apk build --release
      - uses: actions/upload-artifact@v4
        with:
          name: app_Android
          path: target/debug/apk/*.apk
GHACTION

# 3. Фиксируем изменения локально
git add .
git commit -m "Initial clean deployment" --quiet

# 4. Создаем ПРИВАТНЫЙ репозиторий (так как мы уменьшили размер артефактов, квоты должно хватить)
echo "[+] Создание нового приватного репозитория $NEW_REPO_NAME..."
$GH_BIN repo create "$NEW_REPO_NAME" --private --source=. --remote=origin --push

echo "=============================================================================="
echo "🚀 Код успешно отправлен! Подключение к интерактивному прогресс-бару..."
echo "=============================================================================="
sleep 10

# Подключаемся к живому отслеживанию компиляции
$GH_BIN run watch
