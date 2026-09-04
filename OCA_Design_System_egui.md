# oca — UI Design System for egui
## Version 1.0

> **Familiar structure. Distinctive execution.**
>
> oca is a professional desktop video editor. The UI should feel precise, cinematic, technical, dense, restrained and purpose-built — never like a generic SaaS dashboard.

---

## 1. Design Principles

### Personality
- Professional
- Precise
- Cinematic
- Technical
- Information-dense
- Quiet / understated
- Fast and functional

### Avoid
- Glassmorphism
- Excessive rounded cards
- Large decorative gradients
- Neon effects
- Oversized typography
- Excessive whitespace
- Excessive purple
- Floating SaaS-style cards
- Decorative UI with no functional purpose
- Excessive animation

### Core rule

**The workspace is the product.**

Panels should feel like connected parts of one professional workstation, not independent cards.

---

# 2. Color System

Convert these HEX values to `egui::Color32`.

## Backgrounds

| Token | HEX | Usage |
|---|---|---|
| `bg_canvas` | `#090A0D` | Application background |
| `bg_workspace` | `#0D0E12` | Main workspace |
| `bg_panel` | `#111218` | Side panels |
| `bg_elevated` | `#15161D` | Menus, popovers, dialogs |
| `bg_control` | `#191A21` | Inputs and controls |
| `bg_hover` | `#1D1E26` | Hovered controls |
| `bg_selected` | `#211C31` | Selected/focused elements |

## Borders

| Token | HEX | Usage |
|---|---|---|
| `border_subtle` | `#1B1C23` | Very subtle separators |
| `border_default` | `#242630` | Standard structure |
| `border_strong` | `#30323D` | Active structure/dialogs |
| `border_focus` | `#9270FF` | Keyboard focus |

## Text

| Token | HEX | Usage |
|---|---|---|
| `text_primary` | `#E8E9ED` | Main labels |
| `text_secondary` | `#9A9DA8` | Secondary information |
| `text_tertiary` | `#626571` | Metadata / hints |
| `text_disabled` | `#3E404A` | Disabled controls |
| `text_inverse` | `#090A0D` | Text on bright controls |

## Brand / interaction

| Token | HEX | Usage |
|---|---|---|
| `accent_primary` | `#9270FF` | Primary interaction |
| `accent_hover` | `#A184FF` | Hover |
| `accent_active` | `#7C5CE0` | Pressed |
| `accent_muted` | `#211C31` | Selected background |

**Rule:** Purple means active, selected, focused or primary action. Do not use purple decoratively everywhere.

---

# 3. Semantic Colors

| Token | HEX | Meaning |
|---|---|---|
| `media_video` | `#3D5F91` | Video |
| `media_audio` | `#3D8A62` | Audio |
| `media_effect` | `#8C523C` | Effects |
| `media_adjustment` | `#68508C` | Adjustment layers |
| `media_graphics` | `#8B7A3E` | Text / graphics |
| `state_playhead` | `#FF5B67` | Timeline playhead |
| `state_error` | `#D95C5C` | Errors |
| `state_warning` | `#D5A84A` | Warnings |
| `state_success` | `#62B982` | Success |
| `state_recording` | `#E14B55` | Recording |

Semantic colors must remain muted rather than highly saturated.

---

# 4. Typography

## Interface font

**Inter**

Weights:
- Regular 400
- Medium 500
- Semibold 600

Avoid 700/Bold except when necessary.

## Technical font

**IBM Plex Mono**

Alternative: **JetBrains Mono**

Use for:
- Timecode
- FPS
- Resolution
- Numeric parameters
- Metadata
- Frame numbers
- Keyboard shortcuts

## Type scale

| Token | Size | Line height | Usage |
|---|---:|---:|---|
| `display` | 20px | 24px | Rare large headings |
| `section` | 12px | 16px | Panel headings |
| `body` | 12px | 16px | Normal UI |
| `small` | 11px | 14px | Metadata |
| `micro` | 10px | 12px | Timeline / compact labels |
| `technical` | 11px | 14px | Technical values |

oca should generally feel compact.

---

# 5. Spacing

Base unit: **4px**

Allowed values:

`4, 8, 12, 16, 20, 24, 32, 40, 48`

Semantic tokens:

| Token | px | Usage |
|---|---:|---|
| `space_xs` | 4 | Icon/text gap |
| `space_sm` | 8 | Control padding |
| `space_md` | 12 | Component spacing |
| `space_lg` | 16 | Panel content |
| `space_xl` | 20 | Major groups |
| `space_2xl` | 24 | Section separation |
| `space_3xl` | 32 | Major layout separation |

Do not introduce arbitrary values such as 13, 17, 19 or 23px.

---

# 6. Geometry

## Border radius

| Component | Radius |
|---|---:|
| Panels | 0px |
| Standard controls | 3px |
| Inputs | 3px |
| Buttons | 3px |
| Menus | 4px |
| Dialogs | 4px |
| Tooltips | 3px |
| Timeline clips | 3px |

Avoid pill-shaped UI unless functionally necessary.

## Borders

Default: **1px**

Borders define structure rather than decoration.

## Shadows

Default: **none**.

Use background contrast, borders and spacing instead. Menus/dialogs may have a very subtle shadow if needed.

---

# 7. egui Theme Baseline

```rust
use egui::{Color32, Rounding, Stroke, Vec2};

pub fn apply_oca_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(4);

    style.visuals.window_rounding = Rounding::same(4.0);
    style.visuals.menu_rounding = Rounding::same(4.0);
    style.visuals.window_shadow = egui::Shadow::NONE;

    style.visuals.widgets.noninteractive.bg_fill =
        Color32::from_hex("#111218").unwrap();
    style.visuals.widgets.noninteractive.fg_stroke =
        Stroke::new(1.0, Color32::from_hex("#9A9DA8").unwrap());

    style.visuals.widgets.inactive.bg_fill =
        Color32::from_hex("#191A21").unwrap();
    style.visuals.widgets.inactive.fg_stroke =
        Stroke::new(1.0, Color32::from_hex("#9A9DA8").unwrap());

    style.visuals.widgets.hovered.bg_fill =
        Color32::from_hex("#1D1E26").unwrap();
    style.visuals.widgets.hovered.fg_stroke =
        Stroke::new(1.0, Color32::from_hex("#E8E9ED").unwrap());

    style.visuals.widgets.active.bg_fill =
        Color32::from_hex("#211C31").unwrap();
    style.visuals.widgets.active.fg_stroke =
        Stroke::new(1.0, Color32::from_hex("#9270FF").unwrap());

    ctx.set_style(style);
}
```

This is the global baseline. Timeline components should use custom painting where necessary.

---

# 8. Buttons

## Primary
Used for Export, New Project, Confirm and Apply.

- Background: `accent_primary`
- Text: `text_primary`
- Radius: 3px
- Height: 28–32px
- Horizontal padding: 12px
- No shadow

Hover: `accent_hover`  
Pressed: `accent_active`

## Secondary
- Background: `bg_control`
- Border: `border_default`
- Text: `text_primary`

## Ghost
- No visible background
- Hover: `bg_hover`
- Use for toolbar and panel actions

## Destructive
Use `state_error` only for genuinely destructive actions.

---

# 9. Icon Buttons

Recommended:
- Standard: 28×28px
- Compact: 24×24px
- Primary: 32×32px
- Icon: 16px
- Stroke: approximately 1.5px

Colors:

```text
Default   #777A86
Hover     #C5C7CF
Active    #FFFFFF
Accent    #9270FF
Disabled  #41434C
```

---

# 10. Application Menu

Top bar height: **40–44px**

Example:

```text
oca  File  Edit  View  Sequence  Clip  Effects  Markers  Graphics  Window  Help
```

Menu labels:
- 12px Inter Medium
- `text_secondary`
- Hover/active: `text_primary`

## File menu

- Background: `bg_elevated`
- Border: `border_strong`
- Radius: 4px
- Item height: 26–30px
- Horizontal padding: 12px

Example:

```text
New Project...              ⌘N
New Sequence...             ⇧⌘N
New Bin

────────────────────────────

Open Project...             ⌘O
Open Recent                 ›

Import Media...             ⌘I

────────────────────────────

Save                        ⌘S
Save As...                  ⇧⌘S
Save a Copy...

────────────────────────────

Project Settings...
Project Manager...

────────────────────────────

Export                       ›

Close Project
Exit                        ⌘Q
```

Shortcuts should use a compact technical/monospace style.

---

# 11. Tool Rail

Width: **44–48px**

Tool buttons: **32×32px**

Active tool:
- Purple 2px indicator
- `accent_muted` background
- `text_primary` icon

The rail should remain visually quiet.

---

# 12. Panels

Panels:
- `bg_panel`
- 1px `border_default`
- 0px radius
- no shadow

Panel header:
- 32–36px
- 11–12px
- Medium weight

Avoid putting every panel inside a rounded card.

---

# 13. Media Browser

Structure:

```text
MEDIA     BINS     FAVORITES     RECENT

Search media...

PROJECT MEDIA

[thumbnail]  [thumbnail]
Interview    B-Roll
Hero.mp4     City.mp4
```

Metadata:
- 10–11px
- `text_tertiary`
- technical values in IBM Plex Mono

Show duration, resolution, FPS and codec when space permits.

---

# 14. Program Monitor

Header:

```text
PROGRAM MONITOR                     50%   FIT   100%
```

Subtle technical overlays:

```text
CAM 01                         REC ●

1920 × 1080
23.976 FPS

TC 00:02:14:08
FRAME 3292
```

Use IBM Plex Mono for technical overlays.

---

# 15. Playback

Controls should be compact and tool-like:

```text
|◀   ◀   ▶   ▶|   ↻
```

Play button:
- 32–36px
- Accent purple
- Less visually dominant than consumer media players

---

# 16. Timeline

The timeline is the strongest visual signature of oca.

## Header
Height: 36px.

Example:

```text
TIMELINE     Sequence: Interview              +    ZOOM ───●──
```

## Ruler
- 10px IBM Plex Mono
- Clear major ticks
- Subtle minor ticks
- Precise timecode alignment
- Tick density changes with zoom

## Playhead
Use `state_playhead` = `#FF5B67`.

```text
       ▼
       │
       │
       │
```

1–2px vertical line.

Red is intentionally distinct from purple interaction states.

## Track headers
Width: 120–160px.

```text
VIDEO 1
  ◉  🔒  🔊

AUDIO 1
  ◉  🔒  M
```

## Clips
- Video: `media_video`
- Audio: `media_audio`
- Effects: `media_effect`
- Adjustment: `media_adjustment`

Selected clips:
- brighter outline
- subtle accent background
- visible trim handles

Do not represent selection only by changing fill color.

---

# 17. Keyframes

Keyframe language:

```text
Inactive: ◇
Active:   ◆
```

Accent: `accent_primary`.

Use the same language in Inspector, Effects, Timeline and animation controls.

---

# 18. Inspector

Contextual states:

```text
Nothing selected   → PROJECT
Video clip         → CLIP
Effect selected    → EFFECT
Text selected      → TEXT
Audio selected     → AUDIO
```

Example:

```text
INSPECTOR

Interview_Hero.mp4
Video   3840×2160   23.976 FPS

TRANSFORM

Position       X 960.0    Y 540.0
Scale          100.0 %
Rotation         0.0°

Anchor         X 960.0    Y 540.0

CROP

Left             0.0 %
Right            0.0 %
Top               0.0 %
Bottom            0.0 %

COMPOSITE

Blend Mode       Normal
Opacity          100.0 %
```

Numeric values use IBM Plex Mono.

---

# 19. Effects

Do not use a generic list of colored icons.

```text
EFFECTS

⌕ Search effects...

ALL    VIDEO    COLOR    AUDIO

RECENT
────────────────────
Color Grade
LUT Apply

COLOR
────────────────────
Exposure
Contrast
Curves
Color Balance

STYLIZE
────────────────────
Film Grain
Vignette
Glow

BLUR
────────────────────
Gaussian
Radial
```

When an effect is selected, show its actual parameters rather than leaving a generic browser visible.

---

# 20. Audio

Audio color: `media_audio`.

Waveforms:
- muted green
- thin
- dense
- centered around a baseline

Audio meters:
- vertical
- thin
- technical
- restrained

---

# 21. Loading and Status

Avoid large SaaS toast notifications.

Prefer contextual status:

```text
● Rendering preview...
```

or:

```text
PREVIEW
Rendering frame 1248...
```

Place status near the relevant system.

---

# 22. Motion

oca should feel fast.

| Interaction | Duration |
|---|---:|
| Hover | 80ms |
| Focus | 100ms |
| Menu | 100ms |
| Panel | 120ms |
| Dialog | 150ms |

Avoid long transitions and spring animations.

Motion communicates state; it should not provide spectacle.

---

# 23. Component Architecture

Recommended helpers:

```rust
cine_button()
cine_icon_button()
cine_menu_item()
cine_panel()
cine_panel_header()
cine_input()
cine_numeric_input()
cine_slider()
cine_tab()
cine_tool_button()
cine_section()
cine_timecode()

cine_track_header()
cine_video_clip()
cine_audio_clip()
cine_effect_clip()
cine_keyframe()
cine_playhead()
```

Components should consume design tokens rather than hard-coded visual values.

Recommended theme structure:

```rust
pub struct ocaTheme {
    pub bg_canvas: Color32,
    pub bg_workspace: Color32,
    pub bg_panel: Color32,
    pub bg_elevated: Color32,
    pub bg_control: Color32,

    pub border_subtle: Color32,
    pub border_default: Color32,
    pub border_strong: Color32,

    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,

    pub accent_primary: Color32,
    pub accent_hover: Color32,
    pub accent_active: Color32,

    pub video: Color32,
    pub audio: Color32,
    pub effect: Color32,

    pub playhead: Color32,
}
```

---

# 24. Custom Painting

Prefer `egui::Painter` for:

- Timeline ruler
- Timeline clips
- Waveforms
- Playhead
- Keyframes
- Track separators
- Program monitor overlays
- Audio meters
- Selection handles
- Trim handles

These are core oca identity components and require precise control.

---

# 25. Layout Baseline

```text
┌───────────────────────────────────────────────────────────────┐
│ APP BAR                                                        │
├──────┬───────────────────────────────────────┬────────────────┤
│ TOOL │ MEDIA       PROGRAM MONITOR          │ INSPECTOR      │
│ RAIL │                                       │                │
│      │                                       │                │
├──────┴───────────────────────────────────────┴────────────────┤
│ TIMELINE HEADER                                                │
├────────────┬───────────────────────────────────────────────────┤
│ TRACK      │ TIMELINE                                          │
│ HEADERS    │                                                   │
└────────────┴───────────────────────────────────────────────────┘
```

Do not radically change this architecture. Differentiation comes from execution.

---

# 26. Responsive Desktop Behavior

oca is a desktop application.

Wide screen:
- Media browser: 280–320px
- Inspector: 280–320px
- Tool rail: 44–48px
- Timeline: remaining width

Medium:
- Reduce/collapse media browser
- Reduce inspector
- Preserve timeline usability

Priority:
1. Timeline
2. Program Monitor
3. Inspector
4. Media Browser

---

# 27. Accessibility

Minimum target: **24px**

Preferred: **28–32px**

Never rely exclusively on color. Selected clips should also have an outline and handles.

Errors should include:
- color
- icon
- textual explanation

Keyboard focus must be visible.

---

# 28. Visual QA Checklist

### Identity
- Does it look like professional creative software?
- Could it be mistaken for a generic SaaS dashboard?
- Does it use oca's visual language?

### Density
- Is there unnecessary whitespace?
- Can an editor understand information quickly?

### Color
- Is purple overused?
- Are semantic colors communicating meaning?

### Geometry
- Are corners too rounded?
- Are borders structural rather than decorative?

### Typography
- Is hierarchy clear?
- Are technical values monospaced?

### Function
- Does every visible element have a purpose?

### Consistency
- Same spacing?
- Same radius?
- Same states?
- Same typography?
- Same interaction rules?

---

# 29. The oca Test

Before approving a screen:

> **Could this screenshot be mistaken for a generic AI-generated SaaS application?**

If yes, revise it.

> **Does this look specifically designed for professional video editing?**

If no, revise it.

> **If the oca logo were removed, would the interface still have a recognizable identity?**

The goal is **yes**.

---

# 30. Design North Star

> **A precision instrument for video editing.**

oca should not be:
- a website
- a SaaS dashboard
- a futuristic AI tool
- a media player
- a Premiere clone
- a DaVinci clone

The ideal reaction is:

> **“This looks serious. I could spend eight hours editing here.”**

---

# 31. Implementation Priority

1. Theme tokens
2. Typography
3. Panel system
4. Buttons / controls
5. Menu system
6. Tool rail
7. Media browser
8. Program monitor
9. Timeline
10. Inspector
11. Effects
12. Dialogs
13. Loading / status
14. Micro-interactions

The timeline should receive the most custom painting work because it is the strongest opportunity for oca to develop a visual identity of its own.

---

## Final principle

**oca should not try to look different everywhere.**

Establish a small number of strong visual rules and apply them relentlessly:

**Neutral dark workspace + restrained geometry + technical typography + semantic timeline colors + precise interaction states + cinematic metadata = oca.**
