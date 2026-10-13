# PhotoCraft for Android

Native ARM64 tablet build, Android 10/API 29 or newer. App label: **PhotoCraft**. Based on PhotoCraft 0.7.0 at upstream commit `c1bd296dcf8429ef084cc357113c08b56caead67`. This update uses version code 7 and keeps the package and development signing identity from the earlier previews so it can update them in place.

The image editor, UI, gestures, pen input, filters and animation engine are Rust/egui/wgpu. A small Java NativeActivity subclass supplies Android's document picker and Share sheet. There is no WebView or browser editor.

## Tablet workspace

Android settings are in **Edit → Preferences → Android & tablet**: pen-only drawing, palm rejection, touch-control size, customizable floating quick actions, reference panel and manual workspace recovery. The preview status strip, mark beside File, Discord link and moon theme button are removed. Theme choices remain in ordinary Interface preferences. Panel toggles remain under Window; the quick-actions strip can include Layers and Colour toggles.

The layer panel has icon actions for clipping below, transparent-pixel lock, whole-layer lock, reference designation, image import and hue/saturation/lightness. New/Delete/Group/Adjustment/Mask controls are directly below this row. HSL edits the active layer or its selected area. Reference designation is saved in `.pcraft` projects; the reference panel shows these layers and can pin another open document beside the drawing. It does not make a reference layer automatically immune to editing or an automatic fill-reference source.

The movable selection launcher appears near a selection and adds feathering and selection-outline-only transformation to deselect/invert/expand/shrink/delete/cut/copy and selected-pixel transforms. Drag its handle to reposition it. Canvas transform handles use the pen when pen-only mode is enabled.

## S Pen and gestures

Pressure, tilt/orientation, hover and eraser/barrel-button erasing use the native input feed. The initial brush is Hard Round Pressure Size. Pressure-enabled brush dynamics and Preferences → Tools → Use Tablet Pressure must be enabled. Bluetooth Air Actions and barrel rotation are not implemented.

Two fingers pan, pinch to zoom and twist to rotate the view. Gesture contacts and any remaining finger cannot paint. Two-finger taps undo and three-finger taps redo after all fingers lift, within 300 ms and 10 UI points of movement. Palm rejection cancels a held finger when a pen arrives; navigation may continue with a hovering pen but not while the pen is down. Pen-only mode blocks finger/mouse marks on the canvas while allowing menus and controls.

## Drawing animation

Open **Window → Timeline**, choose **Create drawing animation**, and draw the first frame. The default is 12 fps. Blank frame, Duplicate frame and Delete frame edit the active drawing layer. A normal layer stays visible throughout the animation. Add drawing layer creates another animation layer. Scrub or step frames, change fps, play, and enable red/blue onion skins for adjacent frames. Onion previews are downsampled and the cache retains at most two textures.

**Export animation…** opens Android Files and writes an animated GIF on a background worker. Native `.pcraft` saves retain the animation frame stack and frame edits, including the currently displayed frame. Drawing-animation MP4 export, audio synchronization and a full exposure-sheet UI are not included. Drawing-progress timelapse MP4 is a separate feature described below.

## Filters

**Filter → Artistic** adds Bloom, Cross Filter, Noise, Chromatic Aberration (Zooming) and Chromatic Aberration (Moving). Controls include highlight threshold/brightness, glow or ray radius, ray count/direction, and channel-shift distance/centre/direction. Implementations are original and inspired by the requested behaviours; they do not claim pixel-identical ibisPaint results. They obey selections, support the editor's colour modes/bit depths, and produce undoable edits.

## Gallery and files (update 7)

Open/Import → **Gallery / Photos** uses Android's photo picker, with an older-device image chooser fallback. **Browse folders** and **Android Files / projects** remain available for projects and other formats. Save/Export → **Save to Gallery** writes supported image formats into **Pictures/PhotoCraft**; Gallery publishes the image only after the write completes. Projects use Files or the folder browser. Previously granted folders are still remembered.

Imports are retained in unique app-owned folders and opened by path, so Open Recent now has stable files and survives preference restoration. Saving an imported `.pcraft` writes back to its selected URI when permitted. Read-only sources require Save As. Imported photo saves create a Gallery copy. App-owned backups remain available after a failed destination write; an unsuccessful save is reported rather than marked successful. Cancelling a picker makes no editor change.

The JNI bridge retrieves the live PhotoCraft Activity through its application class loader. It no longer casts Application to Activity, the source of `Invalid Object type`.

## Hover and colour picking

Hover a pen or mouse over an icon to see its tool name. Hold one finger still for 550 ms on the canvas or the main pinned reference image to pick the visible colour. Moving more than 8 UI points, adding a second finger, or palm suppression cancels picking. A colour-picking hold never draws. Ordinary short taps and two/three-finger history gestures remain available.

## Timelapse video

**File → Export → Timelapse video…** (also in Export As) offers **10, 30, 60, 120, 300 seconds**. MP4/AVC output is **512 × 512 at 24 fps**, with the artwork fitted over white without cropping, saved to **Gallery → PhotoCraft**. A supported AVC encoder is required; unsupported encoders report an error.

Recording starts with this update. Completed changed document states are captured at most once every three seconds, with a bounded one-item work queue. Captures and video export run sequentially on a worker. Export captures the current completed state before encoding. Per-document recordings are thinned at 300 frames or approximately 48 MiB; one decoded video frame is kept in memory. Recordings are local app data, separate from `.pcraft`, and older edits cannot be reconstructed. The video shows sampled progress, not a stroke-by-stroke replay. Disable recording in the timelapse dialog if desired. Keep the app open during generation; interrupted exports are not resumed.

## Compatibility

ARM64 Android 10/API 29 and newer; 32-bit ARM and x86 are not included. Portrait and resizable tablet windows are enabled. Gallery and folder grants replace the former broad-storage permission. Existing render/cache/large-file limits remain. Hovers and pressure depend on the device's pen hardware. Physical Adreno/Mali/PowerVR, Android 10–16, portrait rotation, process death, SAF provider and AVC encoder testing remains necessary before a stable release.

## Files and recovery

Open/Import and Save/Export use Android's system file picker. Select a Gallery/Files provider or destination in that picker. The floating Share action exports a PNG and opens Android's Share sheet. No all-files permission is needed for these actions. Optional direct shared-folder access remains in Android & tablet preferences. Cancel returns to the editor; access and write failures show errors. Import bytes are staged and read on workers, and app-owned saves use temporary files, fsync and rename. A failed external-provider save retains the app-owned encoded backup; provider writes themselves are not guaranteed atomic.

Incremental background checkpoints include open documents, completed edits, original save paths and canvas views. Returning to a living process retains the session. After process death, a restore prompt offers the saved workspace. The recovery indicator distinguishes saving, the last completed checkpoint and errors. Recovery restores the last completed checkpoint; undo history and temporary uncommitted tool operations are not serialized. Keep manual `.pcraft` copies outside the app folder before uninstalling or clearing app data.

Android safety limits: import staging 128 MiB, 8192 pixels per side / 24 megapixels for ordinary decoded images, PSDs and newly created canvases; PSD import at most 128 layers; native-project manifest 32 MiB, individual blob 256 MiB, aggregate decoded blobs 768 MiB. Drawing layers are limited to 4096 frames; GIF export to 64 million total frame-pixels. Initial budgets are 512 MiB memory, 24 history states, 64 MiB effect cache and 512-pixel tiles. The render/filter pool has two workers. These guards reduce allocation spikes; they do not establish crash-free operation on a physical device.

## Build

Install Rust 1.95+, target `aarch64-linux-android`, JDK 17+, Android SDK platform/build-tools 35/35.0.0 and NDK r29. In the repository root:

```sh
export ANDROID_NDK_ROOT=/path/to/android-ndk-r29
export ANDROID_SDK_ROOT=/path/to/android-sdk
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
cargo build -p photocraft-android --target aarch64-linux-android --profile android-preview -j2
PHOTOCRAFT_SIGNING_KEY="$PWD/apps/photocraft-android/preview.keystore" bash apps/photocraft-android/package.sh
```

The APK uses the earlier preview development identity for in-place updates. No signing key is included in the public source archive. Use your own private release key for public distribution; see RELEASE-GUIDE.md. Packaging compiles the platform classes and dex, aligns and signs the APK. NDK 29 uses 16 KiB-compatible ELF segment alignment.

PhotoCraft is MIT OR Apache-2.0. Vendored winit 0.30.13 retains Apache-2.0 and the existing safe Android motion observation changes. Original launcher artwork and tablet action icons have licence/attribution entries. Proprietary ArtCraft brand artwork is excluded from the delivered source archive.

Physical Samsung Android 16 / S Pen, Gallery providers, Share targets, locking and OS process-kill verification remain required. See VALIDATION.md for automated evidence and remaining checks.
