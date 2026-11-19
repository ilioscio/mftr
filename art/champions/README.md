# Champions

One folder per champion, named by its sim id (`vesper`, `rook`, …). Since 0.21.0 the game shows
the champions built in the creator's wardrobe (doc 22): each folder holds the recipe the client
forges, and the hand-made originals are kept in `old/`:

```
<id>/
├─ concept.md            # copy of _template/concept.md: silhouette, palette, personality, clip checklist
└─ <id>.champion.ron     # the recipe: the official kit and the looks the game forges
old/<id>/
├─ build.py, <id>.blend  # the hand-made original: mesh, rig, every clip with markers
├─ sounds.ron
└─ export/               # <id>.glb + sidecars: no longer shown, but the forge borrows its ability
                         # clips, effects and sounds for the recipe's moves
```

A pack in `<id>/export/` would take the recipe's place in the game. Only Bastion still ships one
(his redesign waits on a non-humanoid scope call); `neo/bastion-neo.champion.ron` is his recipe
under another name, to play next to him with `--champion-dir art/champions/neo`.

The hand-made layout below is how the originals (and Bastion) were made:

Start a new champion from `art/rigs/biped_v1.blend` (*File → Save As* into the new folder), set the panel's **Id** to the champion's sim name and **Kind** to `champion`, and follow the definition of done in [10 §10](../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50). `mftr-tools pack validate` derives the required ability clips from the champion's kit in `mftr-sim` and checks each `fire` marker against the sim's windup.
