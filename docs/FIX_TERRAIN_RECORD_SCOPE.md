# Terrain alignment record scope (2026-10-18)

The map reader previously applied `spline_terrain_align` and `spline_terrain_align_2` to the last road spline even when an object or spline-attachment record came after it. The road renderer could then treat a raised bridge as terrain-aligned.

The existing parser now checks its current record context before changing either road alignment field. Attachment flags no longer enable road alignment, while explicit road flags keep working. This is a narrow correction in the existing 0.2.50-based parser, not an engine upgrade or implementation of attachment-specific terrain alignment.

Tests cover a road 30 metres above the origin followed by an attachment or repeater, plus preservation of an explicit road alignment distance when a subsequent object has another tag. Existing terrain shaping and map file contents are not rewritten by this parser fix.
