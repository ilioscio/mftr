# The wardrobe (22 §4, BW5)

Clothing, armour and accessories for the humanoid builds, each **authored once** and fitted to
all four humanoid bases (slim and heavy, masculine and feminine) when the bases are built.

## An item

`<slot>/<name>/` holds:

| File | What |
|---|---|
| `item.ron` | `id` (`<slot>.<name>`), `slot`, `name`, `layer` (`under`, `over`, `armor`, `accessory`), `accent` (it shows the team color), `rigid` (a plate: little of the body's growth) |
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

- **Modelled** (armour, accessories): model it in Blender around `body_male` and save it as
  `item.blend`, the object named `item`.

Then rebuild the bases (`art/creator/humanoid_*/build.py --export`). `mftr_blender.wardrobe`
fits every item to every base:

1. carried onto the female body (same topology as the male) for the feminine bases;
2. carried onto the base, through each body vertex's place on its source body;
3. pushed out to its layer's distance from the skin;
4. weighted from the body under it, the body's helper bones included, so it follows the body
   sliders and the sculpting.

In the base it is a catalog part flagged `covers`: the forge hides the body under it by its
geometry (`mftr_forge::wardrobe`).

## Items

| Item | Slot, layer | How | Triangles |
|---|---|---|---|
| `torso.linen_tunic` | torso, over | sewn: front and back, mirrored | 1,651 |
| `legs.wool_trousers` | legs, under | sewn: four panels, crotch extension | 1,619 |
| `torso.sleeved_shirt` | torso, over | sewn: front, back, sleeves along the arm | 2,078 |
| `waist.wrap_skirt` | waist, over | sewn: front and back, flared | 1,688 |
| `back.long_coat` | back, armor | sewn: back and two open fronts, mirrored | 3,300 |

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

To try a pattern without rebuilding a base, grow it on a saved base `.blend` with
`mftr_blender.grooming.Head` and `grow`.

## Checking

Check them all, on every base at every body slider's ends:

```
cargo run --release -p mftr-tools -- wardrobe check art
```
