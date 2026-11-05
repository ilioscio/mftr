# Slink — concept

Status: **approved** (2026-11-04); built as `export/slink.glb` (`../build.py`). The black marketeer ([19](../../../docs/design/19-shopkeepers.md)).

## Identity

- **Personality in three words:** shifty, conspiratorial, opportunistic.
- **Who he is:** a minion who left the lane with a few things that weren't his. He sells from a
  cart he can wheel away in a hurry, never stops checking over his shoulder, and every item comes
  with "you didn't get it from me".
- **Shape (14 §2):** *a question mark in a long coat*. The tallest keeper, thin and stooped
  (about 1.75 m to the brim hunched; about 2 m if he ever stood straight), a long ragged coat to
  his ankles with a split tail and a popped collar, a **slouch brim** cocked back over the hood (so his one eye shows
  from the gameplay camera), a lumpy stolen sack on his back, long thin arms,
  hands that are mostly fingers.
- **Face:** the minions' void under the brim; **one eye** glows, the other is an **eye patch**.
  A ragged scarf hides the lower void.
- **Clothes:** the coat in charcoal, its lining acid green and **sewn with pockets** of vials,
  lockpicks and a dagger he flashes when he opens it; fingerless gloves; wrapped shins; a pack
  of rolled maps on his back.

## Palette (14 §4)

Signature: **acid green** (~85°): the vials, the lining. The rest is dark and dusty: he is the
darkest keeper. Linear vertex colors, as in the block-out (`../concepts.py`).

| Slot | Colors |
|---|---|
| cloth | charcoal coat (0.022, 0.022, 0.026) and folds (0.035, 0.034, 0.04), lining (0.30, 0.48, 0.02), brim (0.05, 0.035, 0.025), scarf (0.13, 0.07, 0.035), wraps (0.20, 0.15, 0.09), the sack (0.34, 0.24, 0.12) |
| metal | tarnished iron (0.07, 0.075, 0.085), brass buckles (0.55, 0.38, 0.13) |
| emissive | the vials' green (0.50, 1.0, 0.05) |
| accent | a rag tied to the cart's pole as a flag; the stall's lantern |
| accent_glow | the eye |

## Stall: the cart

A two-wheeled handcart, tipped on its prop leg on the plinth, a **tarp** half pulled back to show
the goods: **crates** stamped with crossed-out marks, racks of **glowing vials**, a bundle of
**daggers**, a rolled rug with something inside, a **cage** under a cloth (two eyes blink in it
now and then), coin purses with other people's initials, a team-colored **rag flag** on the pole
and a hooded **lantern** turned low. He stands at the cart's front corner, in the open.

| Season | Dressing |
|---|---|
| Summer | A crate of fruit stacked in front as cover |
| Autumn | Leaves blown over the tarp, a smoking brazier |
| Winter | Snow on the tarp (free); the scarf wound twice more, a small iron stove with a pipe |

## Performance

- **Idle:** hunched, rocking slowly, the long fingers drumming on the cart; the coat tail and the
  scarf's ends sway. Every few seconds the head snaps left, then right.
- **Fidgets:** (1) opens one side of his coat, shows a row of vials, snaps it shut after a look
  around; (2) flips a dagger, catches it, cleans a nail with it; (3) swings the stolen sack off
  his back, peeks in (something inside glows green), nods, glances around and slings it back.
  (The cage on the cart was the first idea, but it's behind him, out of reach while he faces
  the customer.)
- **greet:** leans way in, a beckoning finger, "psst".
- **purchase:** a quick look both ways, the item slid over, the gold vanishing into the coat.
- **farewell:** a finger to where his lips would be: "shh".
- **victory:** a silent, shoulder-shaking laugh; **defeat:** throws the tarp over the cart and
  crouches behind it.

## Silhouette check

Tall, bent and thin, with a wide flat brim and a coat that flares: the only figure taller than
wide, and the only cart.
