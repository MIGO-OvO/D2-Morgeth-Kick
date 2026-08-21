# Product

<!-- impeccable:product-schema 1 -->

## Platform

Windows desktop application with a public web portal

## Users

The primary user runs D2 Morgath Kick beside Destiny 2 on Windows 10 or 11. They want to check the active setup, tune a deterministic keyboard-and-mouse sequence, start it without returning to a terminal, and stop it immediately if the run is wrong.

## Product Purpose

D2 Morgath Kick is a compact desktop calibration console for one fixed Morgath Kick action sequence. It gives the operator a visible place to confirm resolution and sensitivity, adjust aiming offsets and timing, follow the current phase, and abort the sequence safely.

Success means the operator can understand the current configuration at a glance, make a small correction without editing a file, and trust the configured stop hotkey to stop the sequence and release held inputs.

## Positioning

The product turns a hard-coded replay script into an inspectable timing instrument. A lightweight control window owns calibration and execution while a separate click-through overlay shows only the live state needed during play.

## Operating Context

- Windows 10 or 11 with Destiny 2 running in borderless or windowed mode.
- The main window is used before or between attempts. The always-on-top overlay remains visible during play.
- F8 starts the sequence and F10 aborts it globally by default; both hotkeys are configurable.
- The reference setup is 1920 x 1080, field of view 100, look sensitivity 15, and ADS modifier 1.0.
- The GitHub Pages portal introduces the tool, links to the source repository, and downloads the latest Windows release.

## Capabilities and Constraints

- Rust owns the keyboard and mouse input sequence.
- Non-movement gameplay keys are configurable; W/A/S/D and right-mouse ADS remain fixed.
- First launch shows the required armor, subclass, FOV, and in-game positioning checklist and keeps it available from the main window.
- The interface detects the Destiny 2 client size and also accepts a manual resolution.
- The operator can adjust the first ADS movement, void-arrow landing point, sprint direction, timing, look sensitivity, and ADS modifier.
- Every wait and mouse-movement step must be cancellable. Abort, error, and application exit must release held inputs.
- The overlay is read-only, always on top, click-through, and synchronized through Tauri events.
- The main interface and public portal must support light and dark color schemes.
- Boss or player state recognition, kill confirmation, automatic loops, map interaction, and loot collection are not part of this release.
- The first public package targets 64-bit Windows and is released under the MIT License.

## Brand Commitments

The product name is D2 Morgath Kick. The GitHub repository is named `D2-Morgath-Kick`.

The product speaks in short, direct Chinese. It avoids game-themed hype and does not claim that the sequence succeeds in game. Labels name the setting or action; help text explains the assumption that matters.

The existing D2 monogram and Windows utility character remain recognizable across the desktop application, overlay, README, and portal.

## Evidence on Hand

- `src/App.tsx` contains the working calibration interface and user-facing settings.
- `src/OverlayApp.tsx` contains the read-only in-game status surface.
- `src-tauri/src/engine.rs` contains the seven-stage action timeline.
- `src-tauri/src/config.rs` contains reference values, validation rules, and offset calculations.
- Rust unit tests cover cancellation, timing behavior, scaling, and reference offsets.
- There are no testimonials, success-rate measurements, compatibility benchmarks, or in-game outcome claims to publish.

## Product Principles

- Show the current state, current value, and next action without making the operator hunt for them.
- Prefer deterministic and cancellable behavior over hidden automation.
- Keep calibration close to the values and phase it affects.
- Keep the main window compact and the in-game overlay quieter still.
- Publish only claims the code and tests can support.

## Accessibility & Inclusion

The interface requires keyboard operation, visible focus states, sufficient contrast, reduced-motion support, and status communication that does not rely on color alone. Light and dark themes must remain readable at common Windows scaling levels.
