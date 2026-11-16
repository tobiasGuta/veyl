# Visual specification — Windows Terminal-inspired, not Windows-dependent

## Intent

Match the **appearance** of modern Windows Terminal (Windows 11 dark), not its APIs or Microsoft branding. Aim for closely aligned proportions, tab affordances, compact spacing, typography, low-contrast dark colors, and restrained glass-like backgrounds. Exact pixel equivalence requires reference screenshots and native Fedora comparisons.

## Starting design tokens (adjustable defaults)

| Component | Initial token |
| --- | --- |
| Terminal background | `#10121a` |
| Terminal foreground | `#e8eaf0` |
| Header overlay | Cool charcoal gradient |
| Active tab | Dark tint `rgba(14,16,23,.70)` |
| Idle tab | Clear until hovered |
| Accent | pale cool blue (`#a9bfff`) |
| Opacity | `0.72` in frosted mode |
| Default monospace | Cascadia Mono, 12 pt (font fallback allowed) |
| Header height | 39 CSS px |
| Window controls | 28 CSS px wide, flat hover, 7 CSS px line glyphs |
| Tab controls | 40 × 19 CSS px, shared translucent background, thin plus and chevron |
| Tab row | 33 CSS px |
| Terminal inset | 8 CSS px vertically, 10 horizontally |
| Window radius | 8 CSS px when unmaximized |

CSS pixels are not guaranteed physical pixels at fractional GNOME display scaling. Never use screenshot matching without recording scaling factor, font installation, theme, and system button placement.

## Fidelity levels

1. **v0.1:** compact left-aligned tabs, adjacent add and appearance controls, a dark palette, and live opacity and frosted background controls. GTK supplies the window controls and keeps native titlebar interactions.
2. **v0.2:** visual regression references and a tunable design system for fonts, colors, density, padding, accent color and tab layout; user-selectable themes.

Veyl draws the window-control glyphs with the titlebar stylesheet. The maximize glyph changes from a square to overlapping squares when the window is maximized. GTK supplies the button actions and accessible labels.

## GNOME Wayland transparency versus translucency

- Alpha transparency blends the window surface with whatever is behind it.
- GTK 4.24 forwards the window's backdrop filter to compositors that advertise `ext-background-effect-v1`. The compositor samples and blurs the backdrop. Veyl paints a dark alpha tint above it and keeps terminal glyphs opaque.
- GNOME can provide blur directly through Mutter. Test with Blur My Shell disabled to isolate that path.
- On an unsupported display or GTK version, frosted mode uses a solid background and explains the fallback in the appearance popover. Turn frosted mode off to adjust the background tint without blur.

## Customizing the window chrome

Set the terminal colors, font, blur and tint opacity in `veyl/config.toml` under your user configuration directory. For the chrome, GTK loads `$XDG_CONFIG_HOME/gtk-4.0/gtk.css` (usually `~/.config/gtk-4.0/gtk.css`). Scope overrides to `window.veyl-window`. See [the built-in stylesheet](../assets/veyl.css) for selectors covering tabs, frame, titlebar, terminal padding, and the appearance popover. For example, this sets a square frame and a larger terminal inset.

```css
window.veyl-window,
window.veyl-window .veyl-header {
  border-radius: 0;
}

window.veyl-window .veyl-terminal-surface {
  padding: 12px 14px;
}
```

## UI acceptance checklist

- No sidebar or dashboard in v0.1; shell takes most of the window.
- Titlebar integrates active and inactive tabs, add and dropdown controls.
- Tab selection and close affordances are clear with keyboard focus.
- Colors remain readable at 40% / 72% / 100% background opacity.
- Check 100%, 125% and 150% GNOME scaling; 2+ tabs; long titles; very narrow window widths.
- Do not embed Microsoft's logo/icons or license-unclear artwork.
