# Credits and license
keywords: credits; license; GPL; authors; acknowledgements; disclaimer

## Numerical methods

The numerical engine of Rusteau-1D is a port of the HYDRUS-1D code by J. Šimůnek, M. Šejna, H. Saito, M. Sakai and M. Th. van Genuchten [@simunek2009; @simunek2013]. The equations and methods come from the publications cited in this help ([References](references)).

## Pedotransfer functions

The Rosetta network weights come from rosetta-soil of the USDA-ARS U.S. Salinity Laboratory (CC0 1.0) [@rosettaSoil]; the models are those of [@schaap2001; @schaap2004] and [@zhangSchaap2017].

## Interface

The interface is built with egui and eframe (Rust). The figures are drawn with tiny-skia (BSD-3-Clause license); glyph outlines are read with ab_glyph and ttf-parser (Apache-2.0 / MIT licenses), PNG files are written with the png crate (MIT / Apache-2.0).

## Fonts {#fonts}

The program bundles the fonts **Noto Sans** (version 2.015, regular, bold, italic and bold italic) and **Noto Sans Math** (version 3.000), unmodified. They are the font of the interface on every system (Noto Sans Bold for headings) and the default font of the [figures](figures); Noto Sans Math supplies the mathematical signs and arrows.

Copyright 2022 The Noto Project Authors (https://github.com/notofonts/latin-greek-cyrillic) and Copyright 2022 The Noto Project Authors (https://github.com/notofonts/math).

These fonts are licensed under the **SIL Open Font License, Version 1.1** (https://openfontlicense.org). The license allows the fonts to be used, embedded, bundled and redistributed with software, also commercially, as long as they are not sold by themselves and their copyright notice and license accompany them. The full license text is in the files `OFL-NotoSans.txt` and `OFL-NotoSansMath.txt` next to the program (folder `fonts` of the release archive and of the source code, `crates/rusteau-gui/fonts`) and under *Help → About Rusteau-1D*. Figures made with the fonts (PNG, SVG) are not affected by the license.

## License

Rusteau-1D is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the file LICENSE of the program or [the GNU licenses](https://www.gnu.org/licenses/) for details.

> Rusteau-1D is an independent project. It is not affiliated with or endorsed by PC-Progress or the HYDRUS authors. "HYDRUS" is used only to name the program whose numerical methods Rusteau follows. HYDRUS-1D is available from [PC-Progress](https://www.pc-progress.com/en/Default.aspx?hydrus-1d).
