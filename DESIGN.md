---
name: "D2 Morgeth Kick"
description: "A calibrated Windows console for inspectable timing, precise tuning, and safe interruption."
colors:
  action-blue: "#1769e0"
  action-blue-dark: "#69a5ff"
  action-blue-soft: "#e9f1fd"
  action-blue-soft-dark: "#1c3453"
  workbench-canvas: "#eef0f3"
  workbench-canvas-dark: "#10141a"
  instrument-surface: "#ffffff"
  instrument-surface-dark: "#171c23"
  instrument-subtle: "#f6f7f9"
  instrument-subtle-dark: "#20262f"
  separator: "#dfe3e8"
  separator-dark: "#303844"
  separator-strong: "#c9cfd7"
  separator-strong-dark: "#46515f"
  instrument-ink: "#171a20"
  instrument-ink-dark: "#eef2f7"
  secondary-ink: "#5d6572"
  secondary-ink-dark: "#c1cad5"
  muted-ink: "#626b78"
  muted-ink-dark: "#9ca8b6"
  on-action: "#ffffff"
  on-action-dark: "#111820"
  safety-green: "#16835b"
  caution-amber: "#a56400"
  stop-red: "#bd3038"
  focus-blue: "#0b65d8"
  overlay-panel: "rgba(19, 24, 31, .97)"
  overlay-muted: "#b3bcc9"
  overlay-track: "#3c4552"
  overlay-progress: "#4c92ee"
typography:
  display:
    fontFamily: "Bahnschrift, Segoe UI Variable Display, Microsoft YaHei UI, PingFang SC, system-ui, sans-serif"
    fontSize: "clamp(44px, 5vw, 72px)"
    fontWeight: 760
    lineHeight: 1.06
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "Bahnschrift, Segoe UI Variable Display, Microsoft YaHei UI, PingFang SC, system-ui, sans-serif"
    fontSize: "clamp(31px, 4vw, 52px)"
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable Text, Microsoft YaHei UI, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Microsoft YaHei UI, PingFang SC, Noto Sans CJK SC, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  label:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Microsoft YaHei UI, PingFang SC, Noto Sans CJK SC, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "normal"
  control:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Microsoft YaHei UI, PingFang SC, Noto Sans CJK SC, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 650
    lineHeight: 1.45
    letterSpacing: "normal"
  field:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Microsoft YaHei UI, PingFang SC, Noto Sans CJK SC, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  mono:
    fontFamily: "Cascadia Mono, Cascadia Code, SFMono-Regular, Consolas, monospace"
    fontSize: "10px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  portal-control:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Microsoft YaHei UI, PingFang SC, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 680
    lineHeight: 1.45
    letterSpacing: "normal"
rounded:
  badge: "4px"
  inset: "5px"
  control: "6px"
  sm: "7px"
  button: "8px"
  rail: "9px"
  panel: "10px"
  preview: "13px"
  circle: "50%"
spacing:
  2xs: "3px"
  xs: "5px"
  sm: "8px"
  compact: "10px"
  md: "12px"
  control: "13px"
  panel: "18px"
  section: "24px"
  wide: "28px"
components:
  button-primary:
    backgroundColor: "{colors.action-blue}"
    textColor: "{colors.on-action}"
    typography: "{typography.control}"
    rounded: "{rounded.sm}"
    padding: "0 13px"
    height: "36px"
  button-secondary:
    backgroundColor: "{colors.instrument-surface}"
    textColor: "{colors.instrument-ink}"
    typography: "{typography.control}"
    rounded: "{rounded.sm}"
    padding: "0 13px"
    height: "36px"
  portal-cta-primary:
    backgroundColor: "{colors.action-blue}"
    textColor: "{colors.on-action}"
    typography: "{typography.portal-control}"
    rounded: "{rounded.button}"
    padding: "0 17px"
    height: "44px"
  number-field:
    backgroundColor: "{colors.instrument-surface}"
    textColor: "{colors.instrument-ink}"
    typography: "{typography.field}"
    rounded: "{rounded.control}"
    padding: "0 9px"
    height: "36px"
  calibration-panel:
    backgroundColor: "{colors.instrument-surface}"
    textColor: "{colors.instrument-ink}"
    rounded: "{rounded.panel}"
    padding: "18px"
  overlay-instrument:
    backgroundColor: "{colors.overlay-panel}"
    textColor: "{colors.on-action}"
    typography: "{typography.label}"
    rounded: "{rounded.panel}"
    padding: "0 10px"
    height: "100%"
    width: "100%"
---

# Design System: D2 Morgeth Kick

## Overview

**Creative North Star: "精密校准台 / The Calibrated Console"**

D2 Morgeth Kick feels like a compact instrument placed beside the game: restrained, trustworthy, readable, and immediately operable. It keeps the recognizable character of a Windows utility without copying Windows Settings, using a neutral workbench, blue operational states, tabular calibration values, and a compact live-status summary.

The desktop console and public portal present light and dark themes as equal expressions of one semantic system. The Overlay is the quietest expression: a read-only safety instrument that exposes status, progress, and the stop action without competing with gameplay.

**Key Characteristics:**

- Neutral workbench surfaces with crisp one-pixel separation
- One blue operational accent supported by explicit text and icon states
- Dense, tabular calibration data organized into inspectable groups
- A compact live-status summary in the main title bar and detailed phase progress only in the in-game Overlay
- Paired light and dark themes with a quieter, fixed dark Overlay
- Short, direct Chinese copy and precise Windows utility controls

## Colors

The palette is cool and neutral, with Action Blue reserved for operation and selection while semantic status colors communicate safety, caution, and failure.

### Primary

- **Action Blue:** Marks the current phase, selected calibration target, primary action, progress, and visible focus relationship.
- **Active Wash:** Places a quiet blue field behind selected or informational content without turning the surface into decoration.

### Secondary

- **Safety Green:** Confirms saved or completed states.
- **Caution Amber:** Identifies saving and stopping states.
- **Stop Red:** Identifies aborted, error, and destructive stop-related states.

### Neutral

- **Workbench Canvas:** Holds the full product as a cool, low-contrast work area.
- **Instrument Surface:** Carries cards, controls, headers, and other inspectable surfaces.
- **Subtle Instrument Surface:** Separates nested controls and readouts through tone instead of elevation.
- **Separator and Strong Separator:** Define hierarchy, field boundaries, rails, and card edges.
- **Instrument Ink, Secondary Ink, and Muted Ink:** Step from primary reading to support copy and metadata.
- **Morgeth Character Mark:** Uses the full-color square creature artwork without recoloring. Keep its teal background, square crop, and recognizable purple spore face intact across themes.
- **Overlay Charcoal:** Provides a stable dark safety surface independent of the desktop theme.

**The One Operational Accent Rule.** Use Action Blue for actionable, current, or selected states, never as ambient decoration.

**The Paired Theme Rule.** Switch semantic roles as a complete light or dark set; do not substitute isolated dark values into a light surface.

**The State Redundancy Rule.** Pair every status color with a label, icon, position, or progress value so meaning never depends on color alone.

## Typography

**Display Font:** Bahnschrift with Segoe UI Variable Display and Chinese system fallbacks

**Body Font:** Segoe UI Variable Text with Microsoft YaHei UI, PingFang SC, Noto Sans CJK SC, and system fallbacks

**Label/Mono Font:** Cascadia Mono or Cascadia Code for applied values and progress counters

**Character:** The portal uses a compact industrial display face for decisive public statements. The application stays inside the Segoe family for familiar Windows readability, with monospaced numerals appearing only where calibration or phase progress benefits from fixed-width comparison.

### Hierarchy

- **Display:** Portal hero statement only; tightly set, heavy, and balanced across short lines.
- **Headline:** Portal section transitions and closing statements.
- **Title:** Application section headings and compact instrument labels.
- **Body:** Explanations, operating context, and normal interface reading.
- **Label:** Field names, metadata, helper copy, and compact status text.
- **Control:** Desktop actions with firm weight and clear tap targets.
- **Mono:** Applied coordinates, scale factors, and phase counters.

**The Numeric Instrument Rule.** Use tabular or monospaced numerals for values that operators compare; keep surrounding prose in the UI family.

## Layout

The desktop application uses a draggable custom command title bar and a responsive two-level calibration workspace. At wide sizes, display and sensitivity occupies the upper-left panel, aiming adjustment uses the wider upper-right panel, and action timing spans the full lower row. At compact sizes, the existing workspace tabs expose one complete panel at a time. The title bar keeps runtime status, resolution, Overlay visibility and opacity, actions, and familiar minimize/maximize/close controls together. The footer remains reachable while the main workspace scrolls independently, preventing clipped controls at high Windows display scaling.

The portal opens as a split stage with concise copy beside a large console preview, then moves through the sequence, calibration ledger, safety boundary, and download close. It stacks to one column on compact viewports, preserves usable control heights, reduces the preview before removing proof, and converts long rails and ledgers into smaller responsive groupings.

Spacing follows a compact control rhythm inside cards and a wider section rhythm between product arguments. Alignment is table-like: labels, values, dividers, and rail nodes share edges so scanning remains faster than reading prose.

## Elevation & Depth

The desktop console is flat by default. Depth comes from thin borders, stronger separators, and small tonal changes between canvas, surface, nested surface, and selected surface. Local shadows are limited to a pressed keycap, selected segmented control, or active phase ring. The portal permits one broad ambient shadow under the hero console preview so the implemented product reads as a physical instrument on the page.

The Overlay uses only inset highlights and lowlights. It never casts an outer shadow, because external shadow geometry can create white square artifacts beneath its rounded transparent window.

**The Border-First Rule.** Establish hierarchy with line weight and tonal layering before adding shadow.

**The One Ambient Shadow Rule.** Reserve the broad environmental shadow for the portal hero preview; application cards remain grounded and flat.

**The Inset Overlay Rule.** The Overlay may use inset depth only and must never receive an outer shadow.

## Shapes

The form language is softly squared and exact. Cards and primary panels use modest rounding, controls use tighter corners, and small badges use the tightest corners. Circular geometry is reserved for status dots, phase nodes, switch knobs, and vector targets. The D2 monogram is a compact rounded square, not a pill or ornamental badge.

Borders remain visible in both themes. Clipping is intentional on previews, selected controls, progress tracks, and the Overlay so internal color never leaks beyond the established silhouette.

## Components

Components feel precise, restrained, and sized for an unmistakable click or tap target.

### Buttons

- **Primary:** Solid Action Blue with high-contrast copy, compact icon spacing, and a visible disabled state.
- **Secondary:** Instrument Surface with a strong separator and neutral copy; hover shifts the boundary toward Action Blue.
- **Portal CTA:** Uses the same operational color at a larger public-facing control height and may lift by one subtle step on hover.
- **Focus:** Always uses a visible external outline rather than color change alone.

### Inputs / Fields

- **Numeric Field:** A bordered Instrument Surface with tabular numerals, optional unit suffix, and helper text immediately below.
- **Focus:** The field boundary becomes Action Blue and gains a compact same-color ring.
- **Grouping:** Related X/Y or timing values align as pairs until narrow viewports stack them.

### Cards / Containers

- **Calibration Panel:** Flat Instrument Surface, one separator, restrained corners, and compact internal rhythm.
- **Selected Vector Card:** Adds Active Wash and an Action Blue boundary while preserving all calibration values.
- **Safety Card:** Uses an informational blue wash, a stop icon, and explicit text instead of implying success.

### Navigation

- **Desktop:** Wide windows keep all three calibration groups visible; compact windows switch through the same groups with tabs and independent workspace scrolling.
- **Guide:** The usage guide uses larger readable type and five keyboard-accessible pages with previous/next navigation.
- **Portal:** Plain text links stay secondary to the download action and disappear cleanly on mobile.

### Overlay Safety Instrument

The Overlay is read-only and click-through. It is shown only while the `destiny2.exe` game process owns the foreground window, and follows the upper-center of the game client area. It keeps live status at the start, progress and the phase fraction in the center, and stop guidance at the end; its overall opacity is configurable. As width contracts it removes start guidance first, then progress, leaving the current status as the final safe minimum.

## Do's and Don'ts

### Do:

- **Do** keep calibration values adjacent to the phase or behavior they affect.
- **Do** use Action Blue for primary action, focus, selection, active progress, and no other purpose.
- **Do** pair light and dark semantic roles and verify both at common Windows scaling levels.
- **Do** preserve the explicit configured-stop-hotkey safety path and keep current runtime status visible in the title bar.
- **Do** keep the Overlay read-only, quiet, clipped, and free of outer shadow.

### Don't:

- **Don't** add neon color, glassmorphism, marketing gradients, or game-themed spectacle.
- **Don't** use card shadows as the default source of hierarchy in the desktop console.
- **Don't** claim gameplay success, state recognition, automatic loops, map interaction, or loot collection.
- **Don't** hide status meaning inside color alone or remove visible keyboard focus.
- **Don't** turn the Windows utility character into a copy of the Windows Settings interface.
