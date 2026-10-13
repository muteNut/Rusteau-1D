# Fonts

The interface of Rusteau-1D bundles these fonts unmodified: they are compiled into the program (`rusteau`,
`rusteau.exe`). They are the default font of the figures and a fallback of the interface for symbols that the system
font lacks (Greek letters, sub- and superscripts such as θₛ or cm⁻¹, ≤, ≥, √, ∂, arrows). This file and the license
texts are in the folder `fonts` of the release archives.

| File | Font | Version | Source | License |
|---|---|---|---|---|
| `NotoSans-Regular.ttf`, `NotoSans-Bold.ttf`, `NotoSans-Italic.ttf`, `NotoSans-BoldItalic.ttf` | Noto Sans | 2.015 | <https://github.com/notofonts/latin-greek-cyrillic> (files from <https://github.com/notofonts/notofonts.github.io>, `fonts/NotoSans/unhinted/ttf`) | SIL Open Font License 1.1, [`OFL-NotoSans.txt`](OFL-NotoSans.txt) |
| `NotoSansMath-Regular.ttf` | Noto Sans Math | 3.000 | <https://github.com/notofonts/math> (file from <https://github.com/notofonts/notofonts.github.io>, `fonts/NotoSansMath/unhinted/ttf`) | SIL Open Font License 1.1, [`OFL-NotoSansMath.txt`](OFL-NotoSansMath.txt) |

Noto Sans covers Latin, Greek and Cyrillic with super- and subscripts; Noto Sans Math covers the mathematical operators
and arrows. Text is drawn with Noto Sans and falls back to Noto Sans Math glyph by glyph.

The SIL Open Font License allows the fonts to be bundled, embedded and redistributed with software; the copyright
notices and the license text must accompany them (these files, the folder `fonts` of the release archives, the help
topic *Credits and license* and *Help → About Rusteau-1D*). The fonts are not sold by themselves, and they are not
modified (no Reserved Font Names are declared). Figures made with them (PNG, SVG) are not subject to the license.
