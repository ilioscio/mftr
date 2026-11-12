# The humanoid body (22 §1, BW3)

`body_source.blend` holds two objects, `body_male` and `body_female`.

**Source.** They are `GEO-body_male_realistic` and `GEO-body_female_realistic` from Blender
Studio's **Human Base Meshes** bundle v1.4.1, the same CC0 1.0 bundle as the head
(`../heads/README.md`), from <https://www.blender.org/download/demo/>. Thanks to Blender Studio.

**What was done to them** (`tools/blender/extract_bodies.py`, run on the bundle; the bundle
itself, 49 MB, isn't committed):

- the base cage fitted to the sculpt (Blender's "Apply Base"), then un-subdivided twice: 3,201
  vertices and 6,398 triangles each, the same topology for both;
- the sculpted surface itself at the multires's first level (about 42k vertices) as
  `body_<sex>_hi`: the reduced body is projected onto it, and its chest re-subdivided onto it;
- moved from the bundle's layout to the origin.

The same script printed each body's joints, read from the bundle's "primitive" body of the same
sex (its parts' origins are the joints). They are the `PRIMITIVE` table in
`tools/blender/mftr_blender/cc0_body.py`.

**How the bases use them** (`cc0_body.make`, from `creator.build_base`):

1. **The head is replaced, the neck kept**: the source is fitted with its neck, then `marry_neck`
   drops what the head bone weighs (its face and jaw) and seats the neck's top edge inside the
   authored head.
2. **The arms are corrected.** The male primitive's arms hang at another angle from the mesh's,
   so each arm's joints are turned onto the mesh's own arm.
3. **Weights:** automatic weights from a skeleton on those joints, with the biped's bone names;
   at most two bones a vertex; twins on the underwear's seams share theirs.
4. **The fit:** every vertex goes onto the build's rig bone by bone, turned, stretched to the
   bone's length and thickened by the build's girth. The torso widens by height, from the hips'
   ratio to the shoulders'.
5. **The feet** are squashed below the ankle so the soles end on the floor.
6. **Underwear** is regions of the body itself, on the cloth slot: briefs on every base, a band
   top on the feminine ones, each with a trim. Their edges are cut into the mesh along planes.
   They are the body's paint regions 0 and 1.
7. **The heavy builds' mass** is a mix of `presets.txt`'s sliders (`creator.BUILDS`' `mass`),
   baked in through the helpers (`bake_mass`).

`presets.txt` is the body sliders' presets on the helper bones (21 §9.2).
