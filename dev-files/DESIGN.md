---
name: Nocturne Elegance
colors:
  surface: '#13121b'
  surface-dim: '#13121b'
  surface-bright: '#3a3841'
  surface-container-lowest: '#0e0d15'
  surface-container-low: '#1c1b23'
  surface-container: '#201f27'
  surface-container-high: '#2a2932'
  surface-container-highest: '#35343d'
  on-surface: '#e5e0ed'
  on-surface-variant: '#cdc2d7'
  inverse-surface: '#e5e0ed'
  inverse-on-surface: '#312f38'
  outline: '#968da0'
  outline-variant: '#4b4454'
  surface-tint: '#d6baff'
  primary: '#d6baff'
  on-primary: '#420089'
  primary-container: '#aa73ff'
  on-primary-container: '#3a0079'
  inverse-primary: '#7832d9'
  secondary: '#c9c3dd'
  on-secondary: '#312e42'
  secondary-container: '#4c495e'
  on-secondary-container: '#beb8d2'
  tertiary: '#c9beff'
  on-tertiary: '#2f009b'
  tertiary-container: '#927dff'
  on-tertiary-container: '#290089'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#ecdcff'
  primary-fixed-dim: '#d6baff'
  on-primary-fixed: '#280057'
  on-primary-fixed-variant: '#5f00c0'
  secondary-fixed: '#e6dffa'
  secondary-fixed-dim: '#c9c3dd'
  on-secondary-fixed: '#1c192c'
  on-secondary-fixed-variant: '#484459'
  tertiary-fixed: '#e6deff'
  tertiary-fixed-dim: '#c9beff'
  on-tertiary-fixed: '#1b0063'
  on-tertiary-fixed-variant: '#4500d8'
  background: '#13121b'
  on-background: '#e5e0ed'
  surface-variant: '#35343d'
typography:
  headline-xl:
    fontFamily: Hanken Grotesk
    fontSize: 48px
    fontWeight: '700'
    lineHeight: 56px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Hanken Grotesk
    fontSize: 32px
    fontWeight: '600'
    lineHeight: 40px
    letterSpacing: -0.01em
  headline-md:
    fontFamily: Hanken Grotesk
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
  body-lg:
    fontFamily: Hanken Grotesk
    fontSize: 18px
    fontWeight: '400'
    lineHeight: 28px
  body-md:
    fontFamily: Hanken Grotesk
    fontSize: 16px
    fontWeight: '400'
    lineHeight: 24px
  label-sm:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0.05em
  headline-lg-mobile:
    fontFamily: Hanken Grotesk
    fontSize: 28px
    fontWeight: '700'
    lineHeight: 36px
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  base: 8px
  container-padding: 32px
  gutter: 24px
  card-gap: 16px
  section-margin: 64px
---

## Brand & Style

This design system is built on a foundation of "Mystic Minimalism." It balances the esoteric, celestial themes suggested by the primary brand mark with a high-performance, sleek desktop application interface. The brand personality is enigmatic, sophisticated, and focused.

The design style leverages **Minimalism** with subtle **Glassmorphism** to create a sense of depth without clutter. It uses expansive negative space, precise typography, and a "midnight" color palette to evoke a premium, serene digital environment. The visual goal is to feel like a high-end command center for nocturnal productivity.

## Colors

The palette is derived from the deep violets and obsidian tones of the brand mark. 

- **Primary (#9D5CFF):** A vibrant, electric violet used for primary actions, active states, and focus indicators.
- **Secondary (#1E1B2E):** A deep, muted plum used for surface layers, cards, and navigation sidebars.
- **Tertiary (#6C47FF):** A mid-tone indigo used for accents like progress bars and hover states.
- **Neutral (#0D0C14):** The true-dark base for backgrounds, providing maximum contrast for the purple accents.
- **Functional Grays:** Use `#94A3B8` for secondary text and `#1F2937` for borders to maintain a clean, low-friction appearance.

## Typography

The system utilizes **Hanken Grotesk** for its sharp, contemporary feel and exceptional legibility in dark mode interfaces. Its geometric construction mirrors the celestial motifs of the brand identity. 

For technical data, metadata, and status labels, **JetBrains Mono** is employed to introduce a precise, "utility-chic" aesthetic that complements the sleekness of the tables and inputs. All headlines should favor tighter letter-spacing to maintain a dense, premium look.

## Layout & Spacing

The design system uses a **Fixed-Fluid Hybrid Grid**. Sidebars are fixed (280px), while the main content area utilizes a 12-column fluid grid. 

- **Desktop:** 12 columns, 24px gutter, 32px outer margins.
- **Laptop:** 12 columns, 16px gutter, 24px outer margins.
- **Reflow Rules:** In tables and data-heavy views, columns collapse into list views or overflow horizontally with a persistent "sticky" first column for context.

Spacing follows a linear 8px scale. High-density components (like tables) may drop to 4px internal padding to maximize information density without sacrificing elegance.

## Elevation & Depth

Hierarchy is established through **Tonal Layering** and **Soft Luminescence** rather than traditional heavy shadows.

- **Level 0 (Base):** Neutral (#0D0C14). The canvas.
- **Level 1 (Cards/Sidebar):** Secondary (#1E1B2E). Used for main UI containers.
- **Level 2 (Modals/Popovers):** Lightened Secondary (#2D2945) with a 1px inner border of `#FFFFFF10` to simulate a subtle glass edge.
- **Accents:** Use a 4px-12px "Glow" (Box-shadow) using the Primary color at 20% opacity for active elements like buttons or progress indicators to give them a holographic feel.

## Shapes

The shape language is "Soft-Precision." It avoids the extreme roundness of consumer apps to maintain a professional, desktop-first feel. 

- **Standard Radius:** 4px (Soft) for buttons and inputs.
- **Container Radius:** 8px (Rounded-lg) for cards and main layout panels.
- **Interactive Elements:** Checkboxes and radio buttons use a strict 2px radius or full circles respectively. 
- **Progress Bars:** Use a 4px radius for both container and fill to maintain consistency with the button language.

## Components

### Buttons
Primary buttons use a solid `#9D5CFF` fill with white text. Ghost buttons use a 1px border of `#9D5CFF` and no fill. All buttons should have a 200ms transition on hover, increasing the brightness of the violet.

### Sleek Tables
Tables use a "No-Border" aesthetic. Rows are separated by subtle background color changes on hover. Headers use `label-sm` in all-caps with a low-opacity white (#FFFFFF60).

### Progress Bars
The track is a dark, semi-transparent gray (#FFFFFF10). The fill is a linear gradient from `#6C47FF` to `#9D5CFF`. For "processing" states, a subtle "pulse" animation should be applied to the fill's luminosity.

### Minimal Input Fields
Inputs feature a bottom-border only (2px) in their default state. On focus, the border transitions to the Primary violet, and a very subtle background tint (#9D5CFF05) fills the field.

### Chips/Tags
Small, low-contrast capsules. Background is `#FFFFFF08` with `label-sm` typography. Active or "Selected" tags switch to the Primary violet with 10% opacity and a solid violet border.