# MFTR art sources

Champion models and animations, built to [10 — Characters & Animation](../docs/design/10-characters-and-animation.md). Art, audio and content data are **CC BY-SA 4.0** ([09 §3](../docs/design/09-naming-and-legal.md)).

```
art/
├─ rigs/biped_v1.blend            # the biped v1 reference skeleton + faceted template mesh
│  └─ export/biped_v1.glb (+ .anims.ron)
├─ library/biped/biped_library.blend   # shared clips: walk, cast_utility, attack_melee_alt, cc_*
│  └─ export/biped_library.glb (+ .anims.ron)
└─ champions/<id>/                 # one folder per champion (see champions/README.md)
```

`.blend` files are the source of truth. `export/` holds generated files that are committed so CI (and the game) can check and load them without Blender. `review/` folders hold generated renders and are git-ignored.

## Setup

1. **Blender 5.2 or newer.**
2. **The MFTR add-on** (`tools/blender/mftr_blender`, an "MFTR" tab in the 3D view sidebar). Link it into your Blender extensions folder so edits in the repo take effect on restart:
   - Windows: `mklink /J "%APPDATA%\Blender Foundation\Blender\5.2\extensions\user_default\mftr_blender" "<repo>\tools\blender\mftr_blender"`
   - Linux: `ln -s <repo>/tools/blender/mftr_blender ~/.config/blender/5.2/extensions/user_default/mftr_blender`
   - macOS: `ln -s <repo>/tools/blender/mftr_blender ~/Library/Application\ Support/Blender/5.2/extensions/user_default/mftr_blender`

   Then enable **MFTR Tools** in *Edit → Preferences → Add-ons*.
3. **Optional, for working with Claude:** the official Blender Lab MCP server. In Blender, add the extensions repository `https://lab.blender.org/` and enable the **MCP** add-on (it needs *Online Access* allowed in the system preferences, and listens on `localhost:9876`). Then install the server half and register it with Claude Code:
   ```
   uv tool install "git+https://projects.blender.org/lab/blender_mcp.git#subdirectory=mcp"
   claude mcp add blender --scope user -- blender-mcp
   ```
   Keep Blender open while Claude works; it drives your running instance.

## Everyday commands

Everything the add-on panel does also runs headless (`tools/blender/run.py`):

```
blender -b art/library/biped/biped_library.blend --python tools/blender/run.py -- export --id biped_library --kind library
blender -b art/library/biped/biped_library.blend --python tools/blender/run.py -- review [--clips walk,cast_utility]
cargo run -p mftr-tools -- pack validate art
```

- **Export** writes `<id>.glb` and `<id>.anims.ron` into `export/` next to the `.blend`. The output is deterministic: the same `.blend` gives the same bytes, so a diff in `export/` always means a real change.
- **Review** renders a contact sheet (PNG) and a looping GIF per clip into `review/`: the gameplay camera of [R01](../docs/design/reference/R01-video-ezreal-flash-barrier-q.md) at its true 1080p size (shown 2× with nearest filtering) next to a ¾ close-up. The strip under each sheet tile shows the frame number and the markers (red `fire`, blue loop, green foot contacts, white the playhead). Attach these to PRs.
- **Validate** checks the rig standard, caps, clip catalogue, markers, root motion and loop seams ([10 §8.4](../docs/design/10-characters-and-animation.md#84-validation-ci-and-add-on)). CI runs it on every push.

## Authoring rules (short version)

- Face **−Y** (towards you in Front view), Z up, 1 unit = 1 m = 100 u, A-pose. `_l` bones on +X.
- One clip = one action on its own NLA track (the **New clip** button does this). Clips play **in place**: the sim moves the unit.
- Set markers with the panel buttons on the current frame: `fire` exactly where the sim fires (the validator checks it against the champion's data), `end` on one-shots, `loop_in`/`loop_out` for loop regions, `foot_l`/`foot_r` on locomotion contacts.
- A looping clip's last frame repeats its first.
- A bone with no channel in a clip is at **rest** there. The exporter drops channels that never leave rest.
- Material slots: `skin`, `cloth`, `metal`, `emissive`, plus `accent` (the team color). Colors live in the vertex color attribute `Col` (alpha = baked occlusion). No textures.

The library clips in `library/biped` started as **scripted block-outs** (`tools/blender/mftr_blender/library.py`): key poses and timing that already follow the markers and the timing contract. Polish them in Blender. Re-running the block-out script overwrites them.
