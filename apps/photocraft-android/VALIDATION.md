# Update 7 validation (2026-10-13)

Implemented: Activity/Application JNI type correction; persistent import paths for Recent; Gallery import/save; hover labels; finger-hold sampling; bounded sampled-progress timelapse MP4; welcome credit; portrait/resizable window support; removal of broad storage permission.

Verified before packaging:
- Android Rust build completed successfully with Rust 1.95 / NDK29.
- 19 Android-host unit/regression tests passed, including a layered project write/reopen with pixel equality and recent-path preference persistence.
- 8 offscreen Android-shell integration tests passed, including portrait timelapse duration controls. Landscape welcome and portrait screenshots inspected.
- Java platform compilation passed, 10 browser rule groups and 4 AVC colour-plane conversion cases passed.
- Dependency layering passed for all 30 crates; shared UI WASM check, scoped strict Android-host clippy and regenerated menu parity report passed. Shared UI scoped lint passed with two existing upstream lint categories allowed (`collapsible_match`, `nonminimal_bool`); new findings were corrected.
- APK version7 signature, ZIP CRC, zip alignment and native16KiB LOAD alignment passed.

The Android media codec, Gallery/SAF permissions and the corrected JNI calls have not run on a physical device or Android emulator. Compile and host tests are not device validation. The final packaging checks and portrait test results are recorded in BUILD-INFO.json. Historical checks below are not rerun counts for update 7.

# Update 6: folder-browser validation — 2026-10-13

Android platform sources compile against SDK 35 and are packaged as DEX. Ten host regression groups pass for folder-first/name/date sorting, case-insensitive search, image/project filtering including extension fallback, filename validation, bounded preview sampling and provider listing/depth limits. These tests exercise the pure browser rules, not Android views or a live DocumentsProvider.

The APK is signed with the existing development certificate, version code 6, minimum API 29/target 35. Signature, ZIP CRC and alignment checks pass. The native library is verified byte-for-byte against update 5; Rust/editor sources are unchanged. JNI Activity entry-point descriptors remain compatible. The package includes FolderBrowser and BrowserLogic.

No Android device/emulator was available; the new browser has not been rendered or interacted with on Android. Folder grants, provider-specific thumbnails, permission revocation, cancelling a picker, navigation and confirmed overwrite require SM-X900 verification. The 951 tests below are the prior update-5 results for the reused engine and are not reported as rerun for this platform-only change.

Manual check: Open → Browse folders → grant Pictures or a test subfolder; enter/leave two subfolders, search, toggle the image filter and newest sorting, reopen to verify the selected folder is remembered, cancel, then open a PNG/project. Save a copy into a subfolder; try replacing a disposable test file and cancel the confirmation first. Check System Files for Downloads/cloud sources and a revoked folder grant. Drawing should retain its session throughout.

---

# PhotoCraft Android update validation

ARM64 build, version code 5 / `0.7.0-android-preview.5`, completed on 2026-10-12. This is a native editor; Android picker/Share callbacks use the small platform Activity bridge.

## Automated results

951 host tests passed; 19 pre-existing opt-in tests remained ignored.

| Suite | Passed | Evidence |
|---|---:|---|
| Android unit tests | 14 | Gestures, pen/mouse distinction, palms, resampling, tilt, bounded reads, atomic-write failure, checkpoint restart |
| Android wrapper offscreen tests | 7 | Editor startup, native finger taps, camera pan/zoom/rotation without painting, undo/redo, layer-icon layout, drawing timeline, recovery deferral, selections/feather/outline transform |
| New engine feature tests | 5 | Effects across depths/models, selection boundaries, undo, invalid parameters, bloom spread, visible channel shifts, seeded noise, current drawing-frame persistence and reference metadata |
| Existing video command tests | 16 | Frame operations, scrubbing, edited/source frames, GIF transparency, rendering, progress and cancellation |
| Adversarial command test | 1 | `panic_hunt --ignored`: no command panic/hang |
| Algorithm library | 466 | Existing synthetic image tests |
| Codec library | 38 | Existing synthetic codec tests |
| Document library | 57 | Existing document tests |
| Native-format library | 33 | Existing round-trip/corruption tests |
| I/O library | 128 | Existing synthetic import/export tests |
| PSD library | 186 | Existing parser, writer and malformed-input tests |

Commands used the `android-smoke` profile and targeted libraries/integration files. The six core libraries ran their complete default unit suites. The entire upstream workspace, external real-file corpora and full performance benchmarks were not run; no corpus or performance claims are made.

The offscreen tablet and selection PNGs were visually inspected at 1691×977. The new layer icon row sits above New/Delete controls; the preview strip, mark beside File, Discord and moon button are absent. Timeline frame controls and the selection launcher are visible. A separate offscreen check feathers a selection and transforms only its outline while exported drawing pixels remain identical.

## Build and package checks

- `cargo fmt --all -- --check`: passed.
- `cargo xtask layers`: passed, 30 crates.
- `cargo xtask wasm`: shared L0–L6 crates and optional HEIF check passed.
- Menu parity document regenerated with the warmed `android-smoke` example. This establishes command/menu coverage, not Photoshop feature fidelity.
- Android host strict scoped lint: `cargo clippy -p photocraft-android --target aarch64-linux-android --profile android-preview --lib --no-deps -- -D warnings`: passed.
- Shared changed libraries were linted with `-D warnings`, allowing `clippy::collapsible_match` and `clippy::nonminimal_bool` for four existing style findings in analysis_ui.rs, canvas.rs and algo/transform.rs. The new timeline/type-complexity findings were fixed. All-target upstream lint was not run.
- Java platform classes compile against Android SDK 35 and dex is packaged. Picker/save/result/error method names are present in both DEX and the native library.
- APK signature verified; certificate SHA-256 `aeb79082931e4a287c927c5a8fdc88cc98eb0e6fb9731959b933c952851cc80b`, matching preview.4.
- Package `com.mayamba.photocraftpreview`, app label PhotoCraft, API 29 minimum / 35 target, ARM64 only.
- ZIP CRC and zip alignment checks pass; native ELF LOAD segments have 16 KiB alignment.

## Physical-device verification still required

No Samsung tablet, S Pen or Android emulator was connected. These automated results do not establish device pressure feel, provider-specific SAF/Share behaviour, locking/switching apps, process-kill recovery or crash-free large projects on Android 16. Recovery preserves the last completed background checkpoint, not undo history or uncommitted tool operations. GIF export is bounded; MP4/audio animation is not implemented. The new filters are original implementations, not pixel-identical copies of ibisPaint.

Suggested SM-X900 checks: update in place, make pen strokes and two/three-finger history taps, draw and scrub two animation frames, export GIF through Files, share PNG, then switch apps/lock/relaunch. Check the recovery indicator completes before deliberately stopping the process, and verify restored pixels and views. Also try cancelling imports, an unsupported file, an over-limit image and an unwritable destination; preserve manual project copies while testing.
