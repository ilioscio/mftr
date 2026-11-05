# Old Folio — concept

Status: **approved** (2026-11-04); built as `export/folio.glb` (`../build.py`). The ancient archivist ([19](../../../docs/design/19-shopkeepers.md)).

## Identity

- **Personality in three words:** doddering, kindly, absent-minded.
- **Who he is:** the oldest minion anyone knows of, who has written down everything that ever
  happened in the lane and carries it all on his back. He sells scrolls, tomes and the Astrolabe,
  loses his spectacles on his own head and falls asleep mid-sentence.
- **Shape (14 §2):** *a tower with tiny feet*. The keeper himself is the smallest (about 0.85 m,
  hunched like a minion but older), under a **backpack tower** of scroll cases, rolled maps,
  books and a candle on top that rises to about 1.9 m and leans. A long walking **staff** with a
  hanging lantern.
- **Face:** the minions' void with two glowing eyes behind round **spectacles**; a long white
  **beard** pours out of the void to his knees. A quill tucked into the hood.
- **Clothes:** a violet robe gone soft with age, patched, ink-stained sleeves, a long pointed hood
  that flops over at the tip, slippers.

## Palette (14 §4)

Signature: **violet** (~270°): the robe, the wax seals, a scroll that glows with a rune. Linear
vertex colors, as in the block-out (`../concepts.py`).

| Slot | Colors |
|---|---|
| cloth | violet robe (0.075, 0.03, 0.20), its dark folds (0.035, 0.012, 0.09), the beard (0.72, 0.72, 0.70), parchment (0.62, 0.50, 0.30), leather bindings (0.12, 0.045, 0.02), slippers (0.16, 0.05, 0.08) |
| metal | brass (0.55, 0.38, 0.13): spectacles, the Astrolabe, the lantern |
| emissive | the rune scroll's violet (0.55, 0.20, 1.0), the candle's flame and the lantern (1.0, 0.65, 0.20) |
| accent | the scroll cases' caps; the stall's banner and lamp |
| accent_glow | the eyes |

## Stall: the scriptorium

A sloped **writing desk** heaped with **scrolls** (some unrolled over the edge to the plinth),
**stacks of tomes** leaning against each other, a **globe** on a stand, an **hourglass**, inkwells
and quills, the **Astrolabe** (the support item) on a pedestal, a hanging **banner** in the team
color with a quill sigil, and a lectern lamp. He stands beside the desk at the plinth's front, in the open.

| Season | Dressing |
|---|---|
| Summer | Pages pegged on a line to dry |
| Autumn | Candles burning, fallen leaves on the desk |
| Winter | Snow on the tomes (free); a knitted cap with a bobble on his hood, the lantern lit, a blanket on his knees |

## Performance

- **Idle:** stooped, breathing slow and deep, the beard swaying; the pack tower lags and teeters
  a moment after every move, and the candle's flame wobbles.
- **Fidgets:** (1) pats himself for his spectacles, finds them on his hood, puts them on;
  (2) unrolls a scroll to the floor, reads, rolls it back up; (3) nods off standing (a slow head
  droop, a bubble), jerks awake.
- **greet:** squints, leans forward, then lights up with a wave of the staff.
- **purchase:** licks a finger, writes it in a tiny book, stamps it with a wax seal.
- **farewell:** raises his staff, the lantern swinging.
- **victory:** a creaky little jig, the tower swaying wildly; **defeat:** the top scrolls tumble
  off the pack around him, he sits down on the plinth.

## Silhouette check

Tiny at the bottom and tall above, leaning: the tower over a small figure, plus a staff with a
lantern; no other keeper's height is in its pack.
