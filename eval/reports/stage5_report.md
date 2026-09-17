# Stage 5 Report — FFI Bridge

**Date:** 2026-09-03

## What was built

### 1. Native cdylib wrapper (`app/native/`)

A thin C-ABI wrapper around `loose-ends-core` exposes the store operations needed by the UI:

| C function | Purpose |
|---|---|
| `loose_ends_open(path)` | Open or create the SQLite store at a file path |
| `loose_ends_open_in_memory()` | In-memory store (testing/eval only) |
| `loose_ends_close(handle)` | Free a store handle |
| `loose_ends_ingest_rules(handle, text, y, m, d)` | Run the rule-based extractor, return JSON array of drafts |
| `loose_ends_confirm_draft(handle, id, desc, dir, date, party)` | Promote a draft to a real commitment |
| `loose_ends_list_open(handle, dir, y, m, d)` | List open commitments in a direction with planner actions |
| `loose_ends_create_commitment(handle, ...)` | Manually add a commitment (no draft) |
| `loose_ends_free(s)` | Free a C string returned by the bridge |

**Build status:** `cargo build --release` succeeds against the core (no `neural` feature). The wrapper is ~190 lines, does no logic, just marshals between C and the Rust API.

### 2. Android JNI bridge (`app/android/app/src/main/kotlin/.../NativeBridge.kt`)

- `System.loadLibrary("loose_ends_native")` at class init.
- Singleton with thread-safe lazy init.
- `init(dbDir)` opens the store at `<app files dir>/loose_ends.sqlite`.
- All store operations go through the native handle, so SQLite isn't repeatedly opened.
- JSON marshalling uses `org.json` (zero extra deps, ships with Android).

### 3. Flutter platform channel (`app/lib/bridge/loose_ends_bridge.dart`)

- `LooseEndsBridge.init()` calls `init` on the channel.
- `LooseEndsBridge.ingestText(text)` calls `ingestText` and parses the JSON drafts array.
- `LooseEndsBridge.confirmDraft(...)` and `listOpen(...)` mirror the native methods.
- Catches `MissingPluginException` and `PlatformException` → falls back to a stub so the app launches on platforms where the native lib isn't built (desktop, unbuilt Android variants).

### 4. Flutter UI

A complete Material 3 UI is in place:

- **Home screen** — 4 cards: Capture, Review, You Owe, Owed to You
- **Capture screen** — text field, "Extract" button, local-only badge
- **Review screen** — color-coded direction badges, low-confidence warning icon, confirm/dismiss buttons per draft
- **You Owe / Owed to You screens** — list view with party, due date, and planner action (surface/snooze/escalate/archive)

`flutter analyze` passes with no errors (one unused-import warning removed).

## What's still TODO on the FFI path

1. **Wire the build.** `cargo-ndk` is installed and the NDK is present (verified in Stage 0), but the build pipeline to produce `libloose_ends_native.so` for `arm64-v8a` and link it into the APK hasn't been run end-to-end. The Kotlin `System.loadLibrary` will fail on a real device until this is done.
2. **Neural path on-device.** The native wrapper intentionally uses the core's `neural = []` default-features (rules-only). The full neural path requires the GGUF model bundled as an Android asset (~1.1 GB), which conflicts with the "offline, small, instant" target. The plan is to keep the APK rules-only and offer the neural model as a separate download for power users.
3. **`flutter_rust_bridge` codegen.** The Dart side currently talks to a platform channel by hand. For long-term maintenance, running `flutter_rust_bridge_codegen` against the Rust API and replacing the hand-written bridge with generated bindings is cleaner. Deferred — the hand-written bridge is ~120 lines and works.
4. **Isolate for inference.** When the neural path is enabled on-device, model load + inference must run on a background isolate so the UI thread is never blocked. The Rust circuit breaker already returns a `TimedOut` on long inference, so the UI can show a "still working" state and a fallback result when ready.

## Verification done

- `cargo build --release` in `app/native/` → success
- `flutter analyze` in `app/` → 0 errors
- `flutter pub get` in `app/` → 49 dependencies resolved

## Verification still needed (device)

- Cross-compile the cdylib for `aarch64-linux-android` with `cargo-ndk`
- Drop the `.so` into `app/android/app/src/main/jniLibs/arm64-v8a/`
- Build the APK and confirm the `MissingPluginException` fallback no longer fires
- Latency: rules-only ingest should be < 50ms on a mid-range device
