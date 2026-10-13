#!/usr/bin/env bash
set -euo pipefail
project_root=$(cd "$(dirname "$0")/../.." && pwd)
android_source="$project_root/apps/photocraft-android"
: "${ANDROID_SDK_ROOT:?Set ANDROID_SDK_ROOT}"
: "${PHOTOCRAFT_SIGNING_KEY:?Set PHOTOCRAFT_SIGNING_KEY}"
output_dir="$project_root/target/android-apk"
mkdir -p "$output_dir/lib/arm64-v8a"
if [[ -n "${PHOTOCRAFT_BASE_APK:-}" ]]; then
  : "${PHOTOCRAFT_BASE_NATIVE_SHA256:?Set the validated base native-library SHA-256}"
  python3 - "$PHOTOCRAFT_BASE_APK" "$output_dir/lib/arm64-v8a/libphotocraft_android.so" "$PHOTOCRAFT_BASE_NATIVE_SHA256" <<'PYTHON'
import hashlib,sys,zipfile
with zipfile.ZipFile(sys.argv[1]) as z:
    data=z.read("lib/arm64-v8a/libphotocraft_android.so")
if hashlib.sha256(data).hexdigest()!=sys.argv[3]:
    raise SystemExit("Base native-library checksum mismatch")
with open(sys.argv[2],"wb") as f:f.write(data)
PYTHON
else
  cp "${CARGO_TARGET_DIR:-$project_root/target}/aarch64-linux-android/android-preview/libphotocraft_android.so" "$output_dir/lib/arm64-v8a/"
  "${ANDROID_NDK_ROOT:-$ANDROID_SDK_ROOT/ndk/29.0.14206865}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip" --strip-unneeded "$output_dir/lib/arm64-v8a/libphotocraft_android.so"
fi
build_tools="$ANDROID_SDK_ROOT/build-tools/35.0.0"
"$build_tools/aapt" package -f -M "$android_source/AndroidManifest.xml" -S "$android_source/res" -I "$ANDROID_SDK_ROOT/platforms/android-35/android.jar" -F "$output_dir/resources.apk"
mkdir -p "$output_dir/classes" "$output_dir/dex"
java --module jdk.compiler/com.sun.tools.javac.Main -source 8 -target 8 -classpath "$ANDROID_SDK_ROOT/platforms/android-35/android.jar" -d "$output_dir/classes" "$android_source"/platform/com/mayamba/photocraftpreview/*.java
"$build_tools/d8" --lib "$ANDROID_SDK_ROOT/platforms/android-35/android.jar" --min-api 29 --output "$output_dir/dex" $(find "$output_dir/classes" -name '*.class')
cp "$output_dir/dex/classes.dex" "$output_dir/classes.dex"
(cd "$output_dir" && zip -q resources.apk lib/arm64-v8a/libphotocraft_android.so classes.dex)
"$build_tools/zipalign" -f -p 4 "$output_dir/resources.apk" "$output_dir/aligned.apk"
"$build_tools/apksigner" sign --ks "$PHOTOCRAFT_SIGNING_KEY" --ks-key-alias "${PHOTOCRAFT_SIGNING_ALIAS:-preview}" --ks-pass "${PHOTOCRAFT_STORE_PASS_SPEC:-pass:android}" --key-pass "${PHOTOCRAFT_KEY_PASS_SPEC:-pass:android}" --out "$output_dir/PhotoCraft.apk" "$output_dir/aligned.apk"
"$build_tools/apksigner" verify --verbose "$output_dir/PhotoCraft.apk"
"$build_tools/aapt" dump badging "$output_dir/PhotoCraft.apk"
