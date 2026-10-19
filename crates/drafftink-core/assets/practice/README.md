# Practice sheet fonts

Single-line ("stroke") fonts that draw the letters on the handwriting and
calligraphy practice sheets. Each `.strokes` file holds printable ASCII
(32-126), converted by `convert.py`:

```
# source
metrics <x-height> <ascender> <descender> <slant°>   (font units)
<code> <advance> <x,y x,y ...>|<x,y ...>           (y up from the baseline)
```

Ascender and descender are measured beyond the x-height and baseline (medians
over several letters, so one flourish doesn't stretch them); slant is the
average lean of the letters' upright strokes.

## Hershey fonts

`romans`, `scripts`, `romanc`, `scriptc`, `italicc`, `gotheng`, `gothger` and
`gothita` come from the Hershey font distribution as packaged in the
`vector-text-hershey` crate (`data/hershey.jhf` and its `.hmp` mappings). The
underscore, missing from that data, is a single line. Its use restriction asks
for these acknowledgements:

- The Hershey Fonts were originally created by Dr. A. V. Hershey while working
  at the U. S. National Bureau of Standards.
- The format of the Font data in this distribution was originally created by
  James Hurt, Cognition, Inc., 900 Technology Park Drive, Billerica, MA 01821.

## EMS fonts

The `EMS*` files are Evil Mad Scientist single-line fonts by Sheldon B.
Michaels (SVG conversion by Windell H. Oskay), from Inkscape's extensions
(`svg_fonts`), licensed under the SIL Open Font License 1.1 (`OFL.txt`). Each
is derived from an OFL font: EMS Allure (Allura, Rob Leuschke), EMS Elfin
(Mountains of Christmas, Crystal Kluge), EMS Felix (Felipa, Fontstage),
EMS Nixish and Nixish Italic (Nixie One, Jovanny Lemonad), EMS Readability and
Readability Italic (Source Sans Pro Light, Paul D. Hunt / Adobe), and EMS Tech
(Architects Daughter, Kimberly Geswein).
