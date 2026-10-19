---
name: Crane
description: A calm local-first writing space shaped like a writer's desk at dusk.
colors:
  rail: "#242523"
  rail-raised: "#30312e"
  rail-muted: "#b9bbb4"
  rail-subtle: "#a1a39c"
  paper: "#f1eee7"
  canvas: "#fbfaf7"
  ink: "#282721"
  muted: "#5f5c55"
  faint: "#69655d"
  rule: "#ded9cf"
  ember: "#df5b42"
  ember-text: "#ad3b29"
  ember-soft: "#f4d7cf"
  focus: "#e27560"
  paper-dark: "#292a27"
  canvas-dark: "#20211f"
  ink-dark: "#ece8df"
  muted-dark: "#a9a59c"
  faint-dark: "#b7b3aa"
  rule-dark: "#3b3d39"
  ember-soft-dark: "#53332c"
  ember-text-dark: "#f18a75"
  white: "#ffffff"
  new-note-hover: "#c94d37"
  error: "#9b3325"
  danger: "#be4a39"
typography:
  display:
    fontFamily: "Georgia, Times New Roman, serif"
    fontSize: "clamp(27px, 3vw, 39px)"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "Georgia, Times New Roman, serif"
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  title:
    fontFamily: "Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "13px"
    fontWeight: 650
  body:
    fontFamily: "Georgia, Times New Roman, serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.75
  label:
    fontFamily: "Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "10px"
    fontWeight: 700
    letterSpacing: "0.13em"
rounded:
  marker: "2px"
  code: "3px"
  compact: "5px"
  control: "6px"
  field: "7px"
  menu: "8px"
  circle: "50%"
spacing:
  hairline: "3px"
  xs: "7px"
  sm: "9px"
  md: "11px"
  lg: "15px"
  xl: "20px"
  xxl: "28px"
components:
  new-note-button:
    backgroundColor: "{colors.ember}"
    textColor: "{colors.white}"
    rounded: "{rounded.circle}"
    padding: "0"
    size: "40px"
  new-note-button-hover:
    backgroundColor: "{colors.new-note-hover}"
    textColor: "{colors.white}"
    rounded: "{rounded.circle}"
  navigation-item:
    backgroundColor: "transparent"
    textColor: "{colors.rail-muted}"
    rounded: "{rounded.field}"
    padding: "0 10px"
    height: "40px"
  navigation-item-active:
    backgroundColor: "{colors.rail-raised}"
    textColor: "{colors.white}"
    rounded: "{rounded.field}"
  search-field:
    backgroundColor: "color-mix(in srgb, var(--canvas) 72%, transparent)"
    textColor: "{colors.ink}"
    rounded: "{rounded.field}"
    padding: "0 11px"
    height: "37px"
  note-row:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "0"
    padding: "15px 12px 14px"
  icon-button:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    rounded: "{rounded.control}"
    padding: "0"
    size: "38px"
  icon-button-active:
    backgroundColor: "{colors.ember-soft}"
    textColor: "{colors.ember}"
    rounded: "{rounded.control}"
  empty-action:
    backgroundColor: "transparent"
    textColor: "{colors.ember}"
    rounded: "{rounded.compact}"
    padding: "9px 14px"
---

# Design System: Crane

## Overview

**Creative North Star: "Writer's desk at dusk"**

Crane feels like moving from a charcoal bookcase to a stack of warm index sheets and finally to a bright page under a focused lamp. It is a quiet operating interface: familiar navigation and controls carry the work, while material contrast, restrained typography, and rare ember cues make the system feel authored.

The rail is dense and recessive, the note list is warmer and more tactile, and the editor is the brightest, calmest surface. The writing surface uses serif type for long-form focus; interface controls remain compact sans-serif. Color is functional rather than decorative, and the save confirmation is the one signature motion.

**Key Characteristics:**

- Three contiguous panes with distinct material roles: charcoal rail, warm paper list, bright editorial canvas.
- Ink-led typography with serif writing surfaces and compact sans-serif interface chrome.
- Ember reserved for selection, focus, tags, active tools, and save confirmation.
- Separators and tonal shifts establish structure; floating cards do not.
- Responsive one-pane navigation below 680px, with explicit library/list/editor movement.

## Colors

The palette pairs near-black charcoal and warm paper neutrals with a sparingly used ember accent; the dark theme preserves the same hierarchy rather than simply inverting it.

### Primary

- **Ember:** Selection markers, the create action, active tools, the editor caret, quotations, and successful-save confirmation.
- **Deep Ember Text:** Accessible accent text on light surfaces, including tags, filters, links, and mobile navigation.
- **Soft Ember Wash:** Quiet active and hover fills behind icon controls.

### Neutral

- **Charcoal Rail:** The library ground and the darkest structural surface.
- **Raised Charcoal:** Hover and active rows within the rail.
- **Warm Paper:** The note browser and secondary reading surfaces.
- **Editorial Canvas:** The brightest surface and the editor ground.
- **Ink:** Primary copy and authored content.
- **Muted Ink / Faint Ink:** Secondary copy, metadata, placeholders, and quiet status text.
- **Warm Rule:** Pane boundaries, dividers, field strokes, and the neutral save baseline.
- **Dark Theme Set:** Deep brown-gray paper and canvas, warm off-white ink, softened rules, and a lighter ember text treatment preserve role contrast at night.

### Named Rules

**The Ember Is Evidence Rule.** Ember marks an action, selection, focus target, authored tag, or completed save; it is never ambient decoration.

**The Three Materials Rule.** Keep the rail, list, and editor visibly distinct through charcoal, warm paper, and bright canvas surfaces.

## Typography

**Display Font:** Georgia (with Times New Roman and generic serif fallbacks)  
**Body Font:** Georgia (with Times New Roman and generic serif fallbacks)  
**Interface Font:** Inter (with system sans-serif fallbacks)

**Character:** The serif face makes writing and reading feel literary without becoming ornamental. The compact sans-serif UI stays direct, familiar, and dense enough for a desktop tool.

### Hierarchy

- **Display** (600, fluid 27–39px, 1.15): Borderless note titles, set with tight tracking.
- **Headline** (600, 22px, 1.2): Collection headings and prominent empty-state headings.
- **Title** (650, 13px): Note-row titles and compact high-information labels.
- **Body** (400, 16px, 1.75): Editor text and Markdown preview for sustained reading and writing.
- **Label** (700, 10px, 0.13em, uppercase): Sparse structural labels such as the Tags section; ordinary controls and metadata stay mixed case.

### Named Rules

**The Page Owns the Serif Rule.** Use serif type for authored content, document titles, previews, and literary empty states; navigation, metadata, and controls remain sans-serif.

## Layout

The desktop shell is a full-height three-column grid: a 236px library rail, a 330px note browser, and an editor that flexes from a 420px minimum. The editor document begins below the 61px toolbar and uses fluid horizontal padding from 38px to 112px, keeping the writing field generous without floating it inside a card.

At 900px and below, the rail contracts to 72px, the list to 290px, and library labels collapse while destinations remain available. At 680px and below, the shell becomes a single full-screen pane selected by `data-mobile-pane`: library, list, or editor. Back controls become explicit, common targets expand to 44px, and editor side padding settles at 27px with no horizontal scrolling.

Spacing is compact and repeated rather than theatrical: 3px between grouped controls, 7–11px within rows, 15–20px for component padding, and 28px around the desktop toolbar and outer content edges.

**The Pane Continuity Rule.** Separate regions with rules and surface shifts, never with a dashboard of detached cards.

## Elevation & Depth

The system is flat by default. Pane hierarchy comes from adjacent tonal surfaces and one-pixel rules; shadow is reserved for transient or action-oriented elements. The create button uses a soft ember cast, the action menu uses a compact lifted shadow, the toast uses a stronger overlay shadow, and the empty paper glyph uses a crisp offset sheet. Dark mode removes the general ambient shadow token.

### Shadow Vocabulary

- **Ambient Surface** (`0 18px 45px rgba(43, 37, 29, 0.08)`): Available for light-theme lifted surfaces; resolves to none in dark mode.
- **Ember Action** (`0 7px 20px rgba(223, 91, 66, 0.24)`): The circular create-note control only.
- **Transient Menu** (`0 14px 34px rgba(31, 29, 24, 0.16)`): Floating action menu.
- **Overlay Toast** (`0 10px 30px rgba(0, 0, 0, 0.24)`): Temporary status feedback.

**The Flat-Until-Transient Rule.** Resting panes and note rows stay flat; depth appears only for actions, menus, overlays, and the paper metaphor.

## Shapes

Corners are small and practical. Two-pixel rounding softens selection bars and save rules; 5–8px radii shape controls, fields, menus, and compact containers. The create-note action is the only circular control. Note rows remain rectangular and are separated by rules, while the editor itself is borderless.

**The Quiet Corners Rule.** Keep the small-radius vocabulary; do not turn the interface into a field of pills or oversized rounded cards.

## Components

### Buttons

- **Shape:** Compact utility controls use gently rounded corners (5–7px); icon controls are 38px square on desktop and 44px square on compact screens. The primary create action is a 40px circle, expanding to 44px on compact screens.
- **Primary:** The create-note button uses Ember on white with no border and a soft Ember Action shadow.
- **Hover / Focus:** Hover deepens the ember and lifts the button by 1px; all buttons use a 2px focus outline with a 2px offset.
- **Ghost:** Toolbar icons are transparent and muted at rest, then use Ember text on a Soft Ember Wash when hovered or active.
- **Outlined:** The empty-state writing action is transparent with a 1px ember border and compact 9px × 14px padding.

### Navigation

Library rows are full-width, 40px-high targets with muted rail text and 7px corners. Hover and active states move onto Raised Charcoal with white text; the active destination also receives a 3px inset ember marker. Tag rows use the same dark-rail behavior with a visible ember hash. On compact screens, navigation targets expand to 44px and restore text labels.

### Cards / Containers

Note rows are flat list items rather than cards. Each uses a warm-rule bottom divider, 15px × 12px internal spacing, a compact sans title, a two-line serif excerpt, and 9px metadata. Hover and selected states are translucent canvas washes; selection adds a 3px ember marker instead of a shadow.

### Inputs / Fields

The search field is 37px high with a 1px Warm Rule stroke, 7px corners, and a translucent canvas fill. Focus shifts the border to the focus ember and adds a restrained 3px color-mixed ring. The title and body fields are borderless on the editorial canvas; the body caret alone carries ember.

### Action Menu

The 176px menu is a lifted canvas surface with an 8px radius, warm-rule border, and 6px inset. Rows are compact and left-aligned; hover returns to Warm Paper, while destructive text uses the dedicated danger red.

### Save Feedback

The save label sits over a 14px neutral rule. While saving, the rule extends across the feedback width. After a successful native save, a 420ms ember line contracts once back to the neutral 14px baseline. Reduced-motion mode removes the animation and nearly eliminates transitions; failure remains visible inline with a Retry action.

## Do's and Don'ts

### Do:

- **Do** preserve the charcoal rail, warm-paper list, and bright editorial canvas as three adjacent materials.
- **Do** reserve ember for interactive state, selection, tags, focus, and confirmed save feedback.
- **Do** keep authored writing and preview content in the editorial serif while interface chrome remains compact sans-serif.
- **Do** use the 900px rail-collapse breakpoint and the 680px one-pane library/list/editor transition.
- **Do** keep keyboard focus visible and disable the save pulse when reduced motion is requested.

### Don't:

- **Don't** replace the pane structure with floating cards, glass panels, gradients, or decorative shadows.
- **Don't** spread ember across passive backgrounds or ornamental surfaces; its rarity carries meaning.
- **Don't** use pill-shaped navigation, oversized radii, uppercase eyebrow copy, or decorative monospace.
- **Don't** introduce a second icon style or reduce compact-screen touch targets below 44px.
- **Don't** animate routine writing interactions; the sub-500ms save line is the signature motion.
