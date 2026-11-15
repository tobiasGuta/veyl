# Visual specification — Windows Terminal-inspired, not Windows-dependent

## Intent

Match the **appearance** of modern Windows Terminal (Windows 11 dark), not its APIs or Microsoft branding. Aim for closely aligned proportions, tab affordances, compact spacing, typography, low-contrast dark colors, and restrained glass-like backgrounds. Exact pixel equivalence requires reference screenshots and native Fedora comparisons.

## Starting design tokens (adjustable defaults)

| Component | Initial token |
| --- | --- |
| Terminal background | `#10121a` |
| Terminal foreground | `#e8eaf0` |
| Header overlay | `rgba(28,30,40,.95)` |
| Active tab | subtle translucent gray `rgba(255,255,255,.12)` |
| Idle tab | `rgba(255,255,255,.045)` |
| Accent | pale cool blue (`#a9bfff`) |
| Opacity | `0.86` (prefer 0.80–0.90 when legible) |
| Default monospace | Cascadia Mono, 12 pt (font fallback allowed) |
| Header height | around 44 CSS px |
| Tab row | around 32 CSS px |
| Terminal inset | around 10–12 CSS px |

CSS pixels are not guaranteed physical pixels at fractional GNOME display scaling. Never use screenshot matching without recording scaling factor, font installation, theme, and system button placement.

## Fidelity levels

1. **v0.1 (implemented intent):** header/tab design, custom dark palette, opacity control, per-user TOML. Standard GTK/Wayland window controls are deliberately retained. Reproducibility on real Fedora is pending verification.
2. **v0.2:** visual regression references and a tunable design system for fonts, colors, density, padding, accent color and tab layout; user-selectable themes.
3. **Experimental:** actual compositor background blur if a tested, supported mechanism exists. Windows Acrylic/Mica is **not** a standard GTK effect.

## GNOME Wayland transparency versus translucency

- Alpha transparency blends the window surface with whatever is behind it.
- A frosted translucent *blur* also samples/filters the background; GTK4 CSS cannot independently access other windows' pixels.
- The Blur My Shell GNOME extension is **not** a Veyl dependency or a guaranteed API. Test on standard GNOME Wayland first. No private Mutter patches or screen capture shortcuts to simulate blur.
- Avoid `GtkWidget.set_opacity` on the whole window: it dims glyphs and UI with the background. Instead alpha-blend only surfaces.
- If background blur is unavailable, use a legible semi-transparent/opaque fallback and disclose it in the app/help.

## UI acceptance checklist

- No sidebar or dashboard in v0.1; shell takes most of the window.
- Titlebar integrates active and inactive tabs, add and dropdown controls.
- Tab selection and close affordances are clear with keyboard focus.
- Colors remain readable at 40% / 86% / 100% background opacity.
- Check 100%, 125% and 150% GNOME scaling; 2+ tabs; long titles; very narrow window widths.
- Do not embed Microsoft's logo/icons or license-unclear artwork.
