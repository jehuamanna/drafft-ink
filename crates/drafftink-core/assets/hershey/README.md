# Hershey stroke fonts

`romans.jhf` (Roman Simplex) and `scripts.jhf` (Script Simplex) hold the
printable ASCII characters (32-126, one glyph per line, numbered by character
code) of two Hershey fonts. They draw the letters on the practice sheets.
The underscore, missing from the source data, is a single line.

Extracted from the Hershey font distribution as packaged in the
`vector-text-hershey` crate (`data/hershey.jhf` with `romans.hmp` and
`scripts.hmp`), under its use restriction, which asks for these
acknowledgements:

- The Hershey Fonts were originally created by Dr. A. V. Hershey while working
  at the U. S. National Bureau of Standards.
- The format of the Font data in this distribution was originally created by
  James Hurt, Cognition, Inc., 900 Technology Park Drive, Billerica, MA 01821.

Format: each line is the character code (5 columns), the number of vertex
pairs (3 columns), then pairs of characters giving x and y as offsets from
`R`. The first pair is the glyph's left and right edges; ` R` lifts the pen.
