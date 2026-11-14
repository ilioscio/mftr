# The wardrobe (22 §4, BW5)

Clothing, armour and accessories for the humanoid builds, each **authored once** and fitted to
all four humanoid bases (slim and heavy, masculine and feminine) when the bases are built.

## An item

`<slot>/<name>/` holds:

| File | What |
|---|---|
| `item.ron` | `id` (`<slot>.<name>`), `slot`, `name`, `layer` (`under`, `over`, `armor`, `accessory`), `accent` (it shows the team color), `rigid` (a plate: little of the body's growth); `sleeves` (a torso item with sleeves, fitted to the arms too); `hold` (a bone: what stands off the body, like a quiver's arrows, rides it whole rather than following the arm under it); for a hanging item, its spring-bone chains (below) |
| `item.blend` | One mesh object, `item`, authored around the **reference body**: `body_male` in `art/creator/bodies/body_source.blend`, standing as it is there. Faces use materials named for the material slots (`cloth`, `metal`, `accent`…); the color attribute `Col` holds its paint regions. |
| `build.py` | Optional: what makes `item.blend`. A sewn garment's pattern (`mftr_blender.sewing`). |

## Making one

- **Sewn cloth** (shirts, tunics, skirts, coats, robes): write its pattern, flat panels and
  seams, in `build.py`, and run it with Blender:

  ```
  blender -b --python art/wardrobe/torso/tunic/build.py
  ```

  Blender's cloth simulation sews the panels onto the reference body; the folds and the drape
  are physics (D77, M1). The result is welded, trimmed at its open edges and lined there.

  A panel may have its own color (`sewing.Panel(color=…)`): a ranger's tunic's cream sleeves on
  its green body. Each color is a paint region of its own.

- **Modelled** (armour, accessories): model it in Blender around `body_male` and save it as
  `item.blend`, the object named `item`. A hard object can also be built by its `build.py`
  (the ranger's quiver), its strap laid on the body's surface.

Then rebuild the bases (`art/creator/humanoid_*/build.py --export`). `mftr_blender.wardrobe`
fits every item to every base:

1. carried onto the female body (same topology as the male) for the feminine bases;
2. carried onto the base, through each body vertex's place on its source body;
3. pushed out to its layer's distance from the skin;
4. weighted from the body under it, the body's helper bones included, so it follows the body
   sliders and the sculpting.

In the base it is a catalog part flagged `covers`: the forge hides the body under it by its
geometry (`mftr_forge::wardrobe`).

## Swinging (22 §4.5, BW6)

What hangs swings on **spring-bone chains** of its own (`mftr_blender.chains`), which the client
simulates with Godot's spring bones, with the body as colliders (`client/scripts/springs.gd`):

| `item.ron` field | What |
|---|---|
| `chains` | how many chains: round the hips (a skirt, a coat's skirts) or across the back (a cape) |
| `chain_turn` | where the first is, or the middle of an arc, in degrees round from the front (180 is the back) |
| `chain_arc` | optional: the chains spread across this many degrees instead of all round |
| `chain_from` | the bone they hang from: `pelvis` (the default; they start at the hip joints) or `chest` |
| `swing` | how much of the hem they carry (1: all; the skirt's 0.6 leaves the rest following the legs) |

Each chain is two bones laid down the garment where it settled, and the item's vertices below
where they start blend onto them. Hair marks its hanging lock sets in `groom.py` instead
(`Locks(chain=…)`): a tail's chain, or loose hair's (`split`: a chain each side and down the
back). The base carries every item's chains; the forge keeps only the worn ones', 32 bones at
most (a coat, a skirt and long hair together fit). Past that, the innermost garments go still
first.

## Items

| Item | Slot, layer | How | Triangles |
|---|---|---|---|
| `torso.linen_tunic` | torso, over | sewn: front and back, mirrored; four chains round the hips | 1,651 |
| `legs.wool_trousers` | legs, under | sewn: four panels, crotch extension | 1,619 |
| `torso.sleeved_shirt` | torso, over | sewn: front, back, sleeves along the arm | 2,078 |
| `waist.wrap_skirt` | waist, over | sewn: front and back, flared; four chains round the hips | 1,688 |
| `back.long_coat` | back, armor | sewn: back and two open fronts, mirrored; four chains round the hips | 3,292 |
| `back.travel_cape` | back, armor | sewn: a flared back panel on a collar round the neck; three chains from the chest | 1,292 |
| `torso.ranger_tunic` | torso, over | sewn: green body to the upper thigh, cream sleeves, teal trims; four chains round the hips (Vesper's) | ~2,000 |
| `shoulders.ranger_cloak` | shoulders, armor | sewn: a cloak off the left shoulder on a leather collar and pauldron; three chains from the chest (Vesper's) | 1,369 |
| `back.ranger_quiver` | back, accessory | modelled: leather, steel bands, teal-fletched arrows, a strap across the chest; held on the chest (Vesper's) | 545 |

## Hair

`hair/<style>/` holds a hairstyle (22 §7 item 5, BH1): `item.ron` (`slot: "hair"`) and
`groom.py`, its **grooming pattern** (`mftr_blender.grooming`): a cap above the hairline,
locks (where they root, how long, which way they're combed, how stiff), and ties. There is no
`item.blend`: each base grows its own hair when it is built. The locks are combed over that
base's head, Blender's cloth simulation drops them onto its head and shoulders, and they are
given their body. Places are in the source head's frame (`art/creator/heads`), so one pattern
fits every head.

| Style | Pattern |
|---|---|
| `hair.buzz` | the cap alone, 3.5 mm |
| `hair.short` | short locks combed back, lying on the skull |
| `hair.swept` | parted on the left, the top swept across |
| `hair.long` | parted in the middle, past the shoulders |
| `hair.ponytail` | drawn to a tie behind the crown, a round tail |
| `hair.topknot` | drawn to a tie on the crown, a coiled knot |
| `hair.mohawk` | a standing crest, the sides shaved |
| `hair.braids` | drawn to two ties behind the ears, plaited tails |
| `hair.locs` | round, even locks to the shoulders |
| `hair.coils` | a deep cap with knots of coils |

Every style but the buzz cut and the coils also has a **length control**. It is grown shorter and
longer too, with the same mesh, as the morph targets `hair_length-` and `hair_length+`.

## Beards

`beard/<style>/` holds a beard: the same files on the `beard` slot. It grows on the beard's area
(`grooming.beard`), takes the jaw's weights from the skin right under it, and has the
`beard_length` control.

| Style | Pattern |
|---|---|
| `beard.full` | jaw, chin and lip, locks falling from the chin |
| `beard.goatee` | chin and lip only |
| `beard.braided` | full, with a plaited braid from the chin |
| `beard.moustache` | the lip, drooping past the mouth's corners |

To try a pattern without rebuilding a base, grow it on a saved base `.blend` with
`mftr_blender.grooming.Head` and `grow`.

## Built on the hand

`arms.gauntlets` has no folder here. It is built on each base's own hands
(`mftr_blender.gauntlet.game_part`), the same steel gauntlet the cursor is rendered from
(05 §6). The glove is the hand's surface lifted off the skin with the hand's weights; the
cuff and the back plate ride on it.

## Checking

Check them all, on every base at every body slider's ends:

```
cargo run --release -p mftr-tools -- wardrobe check art
```
