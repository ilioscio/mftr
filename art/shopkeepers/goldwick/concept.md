# Goldwick — concept

Status: **approved** (2026-11-04); built as `export/goldwick.glb` (`../build.py`). The rich merchant ([19](../../../docs/design/19-shopkeepers.md)).

## Identity

- **Personality in three words:** pompous, shrewd, delighted (by money, mostly yours).
- **Who he is:** a minion who did very well for himself and would like you to notice. He prices
  everything twice, the second time higher, and still gives you a wink as he takes your gold.
- **Shape (14 §2):** *a coin purse in a top hat*. A round body almost as wide as it is tall
  (0.80 m across) on short stubby legs, a tall stovepipe hood (the minion's pointed hood grown
  into a top hat, 0.45 m, a gold band), a cane topped with a big gold coin. About 1.45 m to the
  top of the hat.
- **Face:** the minions' void with two glowing eyes; over his left eye a gold-rimmed **monocle**
  that makes that eye's glow bigger. A waxed handlebar moustache, the one thing besides the eyes
  that shows in the void.
- **Clothes:** a navy velvet frock coat with gold buttons straining over the belly, a gold
  waistcoat, a watch chain looping across it, white gloves with a ring on every finger, a fur
  trim at the collar.

## Palette (14 §4)

Signature: **gold** (~45°). Linear vertex colors, as in the block-out (`../concepts.py`).

| Slot | Colors |
|---|---|
| cloth | navy velvet coat (0.025, 0.03, 0.10) and its lighter folds (0.04, 0.05, 0.15), gold waistcoat (0.62, 0.40, 0.07), white gloves (0.62, 0.60, 0.55), fur collar (0.30, 0.22, 0.15), black hat and hood (0.018, 0.018, 0.022), the moustache (0.10, 0.05, 0.02) |
| metal | gold (0.80, 0.52, 0.10): buttons, rings, watch chain, monocle rim, the hat's band, the cane's coin |
| accent | a pocket square; the stall's awning stripes and a lantern |
| accent_glow | the eyes |

## Stall: the counting house

A heavy oak counter on the plinth, its front carved with a coin, brass **scales** weighing a coin
against a feather, coin **stacks** in towers, two bulging **sacks** of gold tied with string
(one slumped open, coins spilled), an iron-banded **strongbox** with its lid ajar and glowing, an
open **ledger** with a quill, a striped **awning** in the team color on two posts, a hanging
lantern. He holds court in front of the counter, in the open, so the gameplay camera sees all of him; the awning covers only the counter and the cabinet behind it.

| Season | Dressing |
|---|---|
| Summer | Bunting along the awning, a potted lemon tree, a fan on the counter |
| Autumn | Gourds stacked by the strongbox, a harvest wreath on a post |
| Winter | Snow on the awning (free); a fur collar turned up, a steaming mug on the counter, a small brazier |

## Performance

- **Idle:** thumbs hooked in his waistcoat, rocking heel to toe, the belly rising and falling; the
  hat lags a beat behind his head.
- **Fidgets:** (1) pops the monocle out, polishes it on his coat, screws it back in with a squint;
  (2) pulls his pocket watch, taps it, holds it to where an ear would be; (3) bites a coin to test
  it, satisfied nod.
- **greet:** tips his hat with a little bow, the cane flourishing.
- **purchase:** rubs his gloved hands together, flips a coin and catches it on the back of his
  hand.
- **farewell:** a two-finger salute off the hat brim.
- **victory:** throws a handful of coins in the air, they rain around him; **defeat:** hugs a
  money sack, trembling.

## Silhouette check

Round bottom, a tall rectangle on top, a thin vertical stick beside: nothing else in the game is
round and top-hatted.
