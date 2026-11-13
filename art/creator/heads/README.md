# The authored head (21 §1.1, FR2)

`head_source.blend` holds one object, `head_source`: the **"Head - Generic Topology"** mesh from Blender Studio's **Human Base Meshes** bundle v1.4.1, with its shape keys (Real_MALE, Real_FEMALE, Ork…). It is **CC0 1.0**, from <https://www.blender.org/download/demo/> (downloaded 2026-11-09, `human-base-meshes-bundle-v1.4.1.zip`). Thanks to Blender Studio. CC0 asks for no credit, but we give it.

`tools/blender/mftr_blender/authored_head.py` builds each humanoid base's head from it:

1. Mix the base's variant: Real_MALE or Real_FEMALE, with Ork on the heavy builds.
2. Subdivide once (~1,270 vertices).
3. Fit it to the rig's head joint.
4. Open the lids onto eyeballs.
5. Paint the brows and lips.

The face bones sit on its landmarks, and the catalog's head parts are conformed to it.

Shape this file in Blender if the head needs changing. The landmark positions in `authored_head.py` must then still find the eyes, nose, lips and chin.
