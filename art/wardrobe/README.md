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

| Item | How | Triangles |
|---|---|---|
| `torso.linen_tunic` | Sewn: front and back panels, shoulder and side seams | 1,693 |
| `legs.wool_trousers` | Sewn: four panels, outer and inner leg seams, middle front and back | 1,723 |

Check them all, on every base at every body slider's ends:

```
cargo run --release -p mftr-tools -- wardrobe check art
```
