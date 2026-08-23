---
name: Locastra Dual Intelligence
colors:
  accent: "#62F9ED"
  accentStrong: "#3CDCD1"
  canvas: "#0E1514"
  surfaceLowest: "#090F0F"
  surface: "#161D1C"
  surfaceRaised: "#1A2120"
  text: "#DDE4E2"
  textMuted: "#BACAC7"
  border: "#3C4948"
  success: "#62F9ED"
  warning: "#FFB956"
  danger: "#FFB4AB"
typography:
  family: '"Segoe UI Variable", "Noto Sans SC", "Microsoft YaHei UI", sans-serif'
  technical: '"Cascadia Code", "JetBrains Mono", Consolas, monospace'
  body: "14px"
  small: "12px"
  title: "28px"
spacing:
  unit: "4px"
  pageGutter: "40px"
shapes:
  controlRadius: "8px"
  surfaceRadius: "12px"
  pillRadius: "999px"
motion:
  fast: "160ms"
  normal: "280ms"
  reveal: "420ms"
  easing: "cubic-bezier(.22,1,.36,1)"
---

## Overview

Locastra is an operational Windows instrument with two deliberately composed appearances: Obsidian Intelligence for dark environments and Lunar White for bright environments. Both preserve the same information hierarchy, cyan action language, density, and native Windows behavior.

Design dials: variance 5, motion 4, density 8. Mode: Operate. Visual reference: the user-provided Stitch neural-console concepts.

## Colors

- Use surgical cyan for selection, primary actions, progress, links, and focus. It is a light source, not a wash.
- In dark mode, the navigation rail, fixed studio bar, and work canvas form one continuous obsidian green-black surface.
- In light mode, use cool lunar white, polished ceramic panels, faint blue-gray depth, deep navy text, and a darker teal action color with accessible white text.
- Theme selection is explicit and stored locally; the application does not follow the Windows theme automatically.
- Status always combines color with text or an icon.

## Typography

- Prefer the Windows system typeface. Simplified Chinese falls back to Noto Sans SC or Microsoft YaHei UI.
- Page titles use 28px/700. Section titles use 16px/650. Body uses 13-14px/1.55.
- Technical metadata and paths use compact 11-12px text; never smaller than 10px.
- English utility labels use uppercase with modest tracking. Technical paths, rates, hashes, and runtime data use the technical font.

## Layout

- Fixed 252px navigation rail and a 58px studio status bar; fluid content canvas.
- Operational pages use a 1320px maximum reading width with 36px gutters.
- Prefer rows, groups, and dividers over placing every item in a floating card.
- Model discovery uses a 3-column grid at wide widths and collapses predictably at narrower widths.
- Chat keeps a readable 860px message measure and a stable composer at the bottom.

## Elevation & Depth

- Most hierarchy comes from surface color, translucency, and 1px catch-light borders.
- Use only three elevations: flat, instrument surface, modal. Blur belongs to the fixed shell and overlays, not every row.
- Cyan glow is restricted to active runtime, selected navigation, focus, and progress.

## Shapes

- Controls: 8px radius. Content surfaces: 12px. Status badges: full pill.
- Icon buttons keep a square hit area of at least 36px; primary controls are at least 40px tall.

## Components

- Navigation selection uses a cyan rail, quiet cyan wash, and a restrained emission glow.
- Primary buttons use cyan with ink text; secondary buttons are translucent outlined surfaces.
- Search is one composed input group with integrated action, clear focus, and no decorative gradient container.
- Cards expose model name, source, fit, metadata, and action in a consistent scan order.
- Messages use restrained surfaces: user messages are cyan-tinted, assistant messages live on the canvas without a heavy bubble.
- Forms use visible labels, explanatory text, strong focus rings, and inline error placement.
- Empty, loading, failed, paused, and completed states must be visually complete.

## Do's and Don'ts

- Do make runtime, download source, progress, and hardware fit visible.
- Do preserve keyboard navigation and reduced motion.
- Do use the existing Lucide icon family consistently.
- Do not use emoji as interface icons.
- Do not use generic AI purple gradients, decorative star fields, or excessive floating cards. Glass is structural and sparse.
- Do not hide expert controls; place them behind explicit advanced disclosures.
