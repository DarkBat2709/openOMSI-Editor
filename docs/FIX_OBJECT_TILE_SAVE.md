# Cross-tile object-save fix (0.8.6-pre)

New objects placed from the catalogue, clipboard or repeated placement can be moved across existing tile boundaries before saving. Save now assigns them to the tile of their current position, keeping their world position, rotation, labels and terrain-relative height. Intermediate saved destinations are remembered for the session and cleaned when the object moves again, is deleted or is restored. Generated roundabout rule blocks move with the object.

Deleted objects outside the map no longer block saving. Active objects outside existing tiles are rejected before object tile files are written, with the object ID and target tile in the message. Existing version restrictions remain in place. Original placement coordinates are retained for Undo.

Regression coverage: three negative-coordinate boundary crossings matching the reported tree/forest case; repeated saves against the overlay; migration through multiple tiles; delete outside the map and restore; missing target without partial object writes; roundabout rule migration; preservation of originals.

Included in 0.8.6-pre; the earlier 0.8.5-pre release assets remain unchanged. The rescue export and memory image are private diagnostics, not repository contents.
