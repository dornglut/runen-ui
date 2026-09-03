# Renderer text fixtures

`Cantarell-Regular.ttf` is the redistributable Cantarell Regular font fixture
used by the renderer's retained shaped-text proof. It is bundled so the tests do
not depend on a host-installed font or platform font selection. The font is
covered by the SIL Open Font License 1.1; the license notice is retained beside
the fixture in `Cantarell-Regular.OFL.txt`.

The M8B production-path fixtures are deterministic, test-only derivatives of
that OFL font. Each retains the Cantarell outlines and adds exactly one valid
intrinsic representation to glyph `A`:

- `RunenUIFixtureColr-Regular.ttf` adds a COLR v0/CPAL table.
- `RunenUIFixtureSvg-Regular.ttf` adds an SVG table.
- `RunenUIFixtureBitmap-Regular.ttf` adds one `sbix` PNG strike.

They are generated from the checked-in Cantarell fixture without a production
dependency. The source and license record is `M8B-intrinsic-fixtures.txt`.
