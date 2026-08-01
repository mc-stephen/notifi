# Notifi Brand

## Assets

| File | Description |
|------|-------------|
| `logo.svg` | Full wordmark — bolt icon + "Notifi" logotype |
| `logo-mark.svg` | Icon only — standalone bolt symbol |
| `logo-icon.svg` | Favicon version (16x16 optimized) |
| `colors.md` | Color palette — primary, neutral, semantic |
| `typography.md` | Type system — fonts, weights, scale |

## Color

- **Primary**: `oklch(0.515 0.235 280)` — #5B3CD9
- **Neutral base**: Fumadocs default grayscale (see `colors.md`)
- **Semantic**: Info, success, warning, error (included in palette)

## Typography

- **Display**: Geist (sans-serif)
- **Monospace**: Geist Mono
- Installed via `geist` npm package or Google Fonts

## Usage

- Apps reference colors by CSS custom properties (e.g., `--color-fd-primary`)
- The bolt icon represents "notification/alert" — the core product metaphor
- Always use SVG for logos; never rasterize
