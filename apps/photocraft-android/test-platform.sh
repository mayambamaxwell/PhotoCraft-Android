#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
classes="$root/target/platform-tests"
mkdir -p "$classes"
java --module jdk.compiler/com.sun.tools.javac.Main -d "$classes" "$root/apps/photocraft-android/platform/com/mayamba/photocraftpreview/BrowserLogic.java" "$root/apps/photocraft-android/tests/platform/BrowserLogicTest.java"
java -cp "$classes" com.mayamba.photocraftpreview.BrowserLogicTest

# Optional Android SDK-backed video pixel tests (does not exercise a hardware codec).
if [[ -n "${ANDROID_SDK_ROOT:-}" ]]; then
  java --module jdk.compiler/com.sun.tools.javac.Main -classpath "$ANDROID_SDK_ROOT/platforms/android-35/android.jar" -d "$classes" "$root/apps/photocraft-android/platform/com/mayamba/photocraftpreview/TimelapseVideo.java" "$root/apps/photocraft-android/tests/platform/VideoLogicTest.java"
  java -cp "$classes:$ANDROID_SDK_ROOT/platforms/android-35/android.jar" com.mayamba.photocraftpreview.VideoLogicTest
fi
