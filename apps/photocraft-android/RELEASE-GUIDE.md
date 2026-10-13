# Sharing PhotoCraft for Android — mayamba

This is an independent native Android port of [storytold/photocraft](https://github.com/storytold/photocraft), based on commit `c1bd296dcf8429ef084cc357113c08b56caead67`. Android port credit: [mayamba](https://mayambamaxwell.com). Describe it as a beta until physical-device checks below pass.

## Open source

1. Fork the upstream repository on GitHub. Create an `android` branch based on the pinned commit, preserving upstream history.
2. Overlay this source archive, excluding `target/`, build outputs, credentials and signing keys. Review the diff, then commit the Android changes. Do not add the proprietary `docs/brand/` artwork. This archive excludes that artwork and signing keys.
3. Keep `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md` and asset-specific notices. The source declares MIT OR Apache-2.0. Credit the original project and identify mayamba's Android port rather than presenting the upstream editor as newly authored.
4. Put the Android build instructions, feature limits, tested-device table and issue-report template in the fork README. Keep `Cargo.lock` and the pinned `craft-libs` dependency.
5. Publish a GitHub pre-release with the matching source tag, APK, SHA-256 checksum and release notes. Link your website to that release, so downloads have a stable destination. No repository or release has been published by this task.

GitHub instructions: https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository

## Signing and public releases

The supplied APK keeps the earlier development signing identity so your current installation can update without deleting data. That identity was distributed with earlier preview sources and is not suitable as a private production key. Before wider distribution, create your own private release keystore and back it up outside the repository. A differently signed APK cannot update the existing package directly: export your `.pcraft` projects first, or use a separate application ID for the public build.

The packaging script accepts `PHOTOCRAFT_SIGNING_KEY`, `PHOTOCRAFT_SIGNING_ALIAS`, `PHOTOCRAFT_STORE_PASS_SPEC` and `PHOTOCRAFT_KEY_PASS_SPEC`. The password specifications support apksigner's `env:NAME` form; store the corresponding values in CI secrets. Never upload a release keystore or password to GitHub. The default alias/passwords remain development-only for existing preview builders.

## Device checks before stable sharing

Supported build target: ARM64, Android 10/API29+, target35. Do not claim all Android tablets are compatible. Begin with a small beta on the Tab S8 Ultra and one Mali-based tablet; add a lower-memory tablet and Android 10/11 device. Test:

- Gallery import, cancelled import, cloud image, Files `.pcraft`, read-only URI, overwrite/Save As, full-storage failure and Gallery export.
- Make visible edits, save a layered project, close/reopen it, restart the app and use Open Recent. Verify pixels/layers, not just a success message.
- S Pen/mouse hover labels; finger hold on the canvas and reference; moving/second-finger cancellation; pressure/palm rejection.
- All five timelapse durations, playback in Gallery/another player, correct final artwork, encoder error, interruption and available storage. This is sampled progress at 512px, not full-resolution stroke replay.
- Portrait/landscape/split-screen at multiple densities; switching apps, screen lock, OS process kill and recovery of the last completed checkpoint.
- Import/layers/export near the documented image limits, with memory/ANR monitoring. Keep a device/version/renderer result table.

For issue reports request device model, Android version, app version, exact steps, image dimensions/layer count, the error text and a non-private sample. Do not request a user's private artwork by default.

## Tweet draft (use when the release link is live)

I wanted to draw on my Android tablet, so I ported PhotoCraft into a native app. S Pen support, gestures, layers, selections, references, animation and timelapse export. Built on the open-source PhotoCraft editor. Android beta + source: [release link]

No tweet has been posted.

## Supplied beta APK checksum

Version code 7, ARM64. SHA-256:

```text
58145e288ec8a1be91789370c51187a8282599ebde8cb267a4bd3ad76ae109a5  PhotoCraft.apk
```
