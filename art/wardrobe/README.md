# The wardrobe (22 §4, BW5)

Clothing, armour and accessories for the humanoid builds, each **authored once** and fitted to
all four humanoid bases (slim and heavy, masculine and feminine) when the bases are built.

## An item

`<slot>/<name>/` holds:

| File | What |
|---|---|
| `item.ron` | `id` (`<slot>.<name>`), `slot`, `name`, `layer` (`under`, `over`, `armor`, `accessory`), `accent` (it shows the team color), `rigid` (a plate: little of the body's growth); `sleeves` (a torso item with sleeves, fitted to the arms too); `hold` (a bone: what stands off the body, like a quiver's arrows, rides it whole rather than following the arm under it); for a hanging item, its spring-bone chains (below); for a **held** item (a weapon), `grip` and its frame (below) |
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

- **Shells** (boots, bracers, gloves: close-fitting, not cloth): a region of the reference
  body's own skin, lifted off it, smoothed, enveloping the body under it, thickened and colored
  by region (`mftr_blender.shell`). The item's `build.py` says which region and how.

- **Plates** (breastplates, gorgets, a boot's shaft, pauldrons, knee cops: hard, standing off the
  body): `mftr_blender.plates`. *Lathed* round an upright axis through the body, or a *tube*
  round a path through it (a foot, heel to toe): rings of rays find the skin, each point stood
  off it, smoothed into a plate; its edges and bands are the grid's rows, straight lines rather
  than the low-poly reference body's facets. *Domes* (an ellipsoid's cap) and *strips* of them
  for pauldrons, their lames and knee cops. See `shoulders/veteran_plate/build.py`.

Then rebuild the bases (`art/creator/humanoid_*/build.py --export`). `mftr_blender.wardrobe`
fits every item to every base:

1. carried onto the female body (same topology as the male) for the feminine bases;
2. carried onto the base, through each body vertex's place on its source body;
3. pushed out to its layer's distance from the skin. A torso, waist, legs or back item ignores
   the arms below the shoulder: it hangs beside them. A `shoulders` item keeps only the shoulder
   caps (fitted and weighted): a pauldron rides the shoulder, but a cloak's side doesn't follow
   the elbow or the hand;
4. weighted from the body under it, the body's helper bones included, so it follows the body
   sliders and the sculpting.

In the base it is a catalog part flagged `covers`: the forge hides the body under it by its
geometry (`mftr_forge::wardrobe`). Worn together, an outer item is pushed clear of an inner
one, except over boots, bracers and gloves (the `feet`, `arms` and `hands` slots): what's under
them is tucked in instead.

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
| `shoulders.ranger_cloak` | shoulders, armor | sewn: a cloak off the left shoulder on a leather collar and pauldron, lined all over; three chains from the chest (Vesper's) | 1,156 |
| `back.ranger_quiver` | back, accessory | modelled: leather, steel bands, teal-fletched arrows, a strap across the chest; held on the chest (Vesper's) | 545 |
| `feet.ranger_boots` | feet, over | shell: knee-high leather, a flared cuff, an ankle strap, dark soles (Vesper's) | 1,372 |
| `arms.ranger_bracers` | arms, over | shell: leather forearms, two teal bands, flaring at the elbow (Vesper's) | 864 |
| `torso.veteran_gambeson` | torso, over | sewn: padded brown, a high neck, long sleeves to the wrist, bound in dark leather (Rook's) | 1,638 |
| `shoulders.veteran_plate` | shoulders, armor | plates: an iron breastplate and backplate, open at the sides, a gorget, crimson-lacquered pauldrons with brass rims over two lames each; rigid (Rook's) | 1,632 |
| `waist.veteran_tabard` | waist, armor | sewn: long crimson panels front and back from a band at the waist, open at the sides, lined; two chains (Rook's) | 1,584 |
| `feet.veteran_boots` | feet, over | plates: heavy leather tubes from heel to toe and shafts to under the knee, two iron bands, an iron toe cap, thick soles, domed iron knee cops (Rook's) | 1,896 |
| `offhand.ranger_bow` | offhand, held | modelled: a tall recurve, flat wide limbs, a leather grip between silver bands with a teal stone, silver caps, teal-glowing tips, the string; bones that draw with Vesper's shots (Vesper's) | 472 |

## Held items (22 §4.7, BW7)

A weapon (or a shield, a lantern) is **held**, not worn: it isn't fitted to the body, it is
placed on the hand. Its `item.blend` is modelled in its own **grip frame**: the grip at the
origin, +Z along it (a blade's point, a bow's upper limb), +Y its back (a bow's string side).

| `item.ron` field | What |
|---|---|
| `grip` | the bone it is placed by (`hand_l`, `hand_r`, `forearm_l`); without `grasp_radius` it rides that bone alone, rigid |
| `grip_at` | where along that bone the grip sits, as a fraction of its length |
| `grip_up`, `grip_back` | where the item's +Z and +Y point, in the bone's own frame (+Y along the bone) |

An item **gripped in the fist** adds (22 §4.7, "Hands that hold"):

| `item.ron` field | What |
|---|---|
| `grasp_radius` | the haft's radius where it is held (m). With it, the item rides that hand's prop bone and the forge closes the hand on it in every clip |
| `grasp_at` | where along the item (its +Z) the fist's middle sits |
| `grasp_span` | how far along it a hand may hold: a second hand closes on it there |
| `grasp_edge` | it has an edge (the knuckles lead it) |
| `grasp_on_hand` | the clips carry it on the hand bone itself, not the prop bone (the ranger's bow) |
| `grasp_upright` | carried upright whatever the hand does (the orb, the lantern) |
| `grasp_strap` | strapped to the forearm (a shield): it keeps its `grip` bone, is slid in its own plane till the fist is behind its middle, gets a grip bar there on posts to its back, and the fist closes on it |

Model a haft no thicker than a hand closes on: a radius of 12 to 30 mm.

`class` and `weight` (light, medium, heavy) record its weapon class and weight class. Model one
with `mftr_blender.armory` (a `Kit` of lathes, slabs, blades, boxes and stones in the grip
frame): see `weapon/sword/build.py`. The weapons: `weapon.dagger`, `sword`, `hammer`, `staff`,
`spear`, `great_axe`, `maul`; `offhand.shield`, `tower_shield`, `orb`, `lantern`, `ranger_bow`.

A held item can have **bones of its own**: `mftr_bones` on its `item.blend` object, a JSON list
of `[name, head, tail, parent]` in the grip frame (`"grip"` is the hand), with vertex groups by
those names (and `grip` for what rides the hand). They're added to each base's rig with it.
Named as the official champion's, they play its clips: the ranger's bow's limbs and string
draw with Vesper's shots.

The frame comes from the official champion whose moves the weapon class plays, so their clips
aim it as they aim their own: the ranger's bow's was read off the hand-made Vesper, so it stands
upright at her full draw. A held item covers nothing and isn't layered.

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
| `hair.swept_ponytail` | side-swept bangs, the rest drawn to a tie behind the crown, a round tail (Vesper's) |
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
