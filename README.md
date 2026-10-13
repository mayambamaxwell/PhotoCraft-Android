# PhotoCraft

Native Android drawing and image editing, ported by [mayamba](https://mayambamaxwell.com).

An independent Android port of [PhotoCraft by the ArtCraft Team and contributors](https://github.com/storytold/photocraft). The editor uses native Rust/egui/wgpu with a small Java bridge for Android services.

## Download the Android beta

[**PhotoCraft Android Preview 7 — APK and full source**](https://github.com/mayambamaxwell/PhotoCraft-Android/releases/tag/v0.7.0-android-preview.7)

On the release page, expand **Assets** and choose:

- **PhotoCraft.apk** to install. Android 10+; ARM64 devices.
- **PhotoCraft-Android-source.zip** for the complete source used for this APK, including the Android host, shared editor changes, patched winit, build instructions, validation notes and licenses.

**Use the named Android source ZIP to build the Android app.** The repository checkout and GitHub-generated “Source code” ZIP/tarball currently contain upstream code and this download page. Android changes are distributed in the attached full source snapshot until they can be committed to the repository.

## Features added for Android

- S Pen pressure and tilt, palm rejection and pen-only drawing.
- Two-finger pan, zoom and rotation; two-finger undo and three-finger redo.
- Selection launcher: deselect, invert, expand, shrink, feather, delete, copy/cut to a layer and free transform.
- Layer clipping, transparency lock, reference-layer designation and hue/saturation/lightness controls.
- Reference image panel and finger-hold colour picking on the canvas or reference.
- Drawing animation timeline, onion skins and GIF export.
- Bloom, Cross Filter, Noise and chromatic aberration filters.
- Gallery image import/export, project folder browsing, recovery checkpoints and bounded caches.
- Hover tooltips, portrait/resizable layouts and MP4 timelapse export in 10, 30, 60, 120 or 300 seconds.

## Build the Android source

Download and unzip **PhotoCraft-Android-source.zip** from the release above. Start with **apps/photocraft-android/README.md** inside the extracted **PhotoCraft-Android-Source** folder. Build metadata is in **BUILD-INFO.json**, test evidence in **VALIDATION.md**, and release signing guidance in **RELEASE-GUIDE.md** beside that README.

The Android snapshot is based on upstream commit **c1bd296dcf8429ef084cc357113c08b56caead67**. Its dependency revisions and build requirements are documented in the snapshot. No signing keys are included.

## Beta status

19 Android-host tests and 8 offscreen UI tests passed; the optimized ARM64 APK, signature and 16KB native alignment were checked. Physical-device compatibility testing remains necessary. Back up important artwork and report issues with the device model, Android version and steps to reproduce.

Timelapse captures completed states from this update onward, at 512×512 and 24fps; it cannot reconstruct previous strokes. Keep the app open during export. Recording data is separate from project files.

This beta preserves the development signing identity so existing previews can update. That key was distributed with older source bundles and is not a private production key. A production release requires a new private signing identity; export projects before changing signing identity or uninstalling.

## Credits and licenses

Original PhotoCraft: **ArtCraft Team and PhotoCraft contributors**. Android port: **mayamba**, [mayambamaxwell.com](https://mayambamaxwell.com).

PhotoCraft code is dual-licensed **MIT OR Apache-2.0**, at your option. Preserve [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), [NOTICE](NOTICE) and [ATTRIBUTION.md](ATTRIBUTION.md). Bundled assets and vendored dependencies retain their own licenses and notices; the Android source ZIP includes its additional notices. This port is not an official ArtCraft release.
