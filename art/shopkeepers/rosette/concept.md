# Madame Rosette — concept

Status: **approved** (2026-11-04); built as `export/rosette.glb` (`../build.py`). The perfumer and charmer ([19](../../../docs/design/19-shopkeepers.md)).

## Identity

- **Personality in three words:** flirtatious, theatrical, vain.
- **Who she is:** a minion who discovered perfume and never looked back. She sells potions as
  fragrances ("a little health, darling, and you'll smell divine"), adores her own reflection and
  blows kisses at every customer.
- **Shape (14 §2):** *a bell with a bow*. A wide hooped gown (1.0 m across at the hem, hiding her
  feet; she glides), a tiny cinched waist, puffed sleeves, a **huge pink bow** on top of her hood
  whose loops stand out past the hood's width, and a lace **parasol** on her shoulder. About
  1.5 m to the top of the bow.
- **Face:** the minions' void, but with **painted lips** (glossy rose, a cupid's bow), long
  **lashes** over the two glowing eyes, and a beauty mark.
- **Clothes:** a rose gown with cream ruffles in tiers, a lace collar, long cream gloves, a pearl
  necklace, a heart-shaped hand mirror on a ribbon at her waist.

## Palette (14 §4)

Signature: **rose** (~325°), kept pink enough to never read as the enemy's red. Linear vertex colors, as in the block-out (`../concepts.py`).

| Slot | Colors |
|---|---|
| cloth | rose gown (0.60, 0.09, 0.26) in panels with deep rose (0.36, 0.035, 0.14), puffed sleeves (0.85, 0.25, 0.48), lace ruffles, gloves and parasol (0.80, 0.74, 0.66), the bow (0.90, 0.20, 0.50), the lips (0.70, 0.015, 0.12) |
| metal | gold (0.80, 0.52, 0.10): the mirror's frame, the stand; pearls (0.85, 0.83, 0.78) |
| accent | a ribbon around the parasol's handle; the stall's canopy and a hanging lamp |
| accent_glow | the eyes |

## Stall: the perfumery

Three tiers of shelves under a scalloped **canopy** in the team color, crowded with **bottles** of
every shape (round flasks, tall perfume bottles with bulb sprays, hearts) in glowing pinks,
lilacs and the health potions' red, a **heart mirror** on a stand she keeps turning to, spools of
**ribbon**, a velvet **fainting couch** with a hat box, and a puff of pink mist from a sprayer. She stands in front of her shelves, in the open; the canopy covers only the shelves.

| Season | Dressing |
|---|---|
| Summer | A trellis of climbing roses, a fan on the couch |
| Autumn | Dried flowers in vases, a knitted shawl over the couch |
| Winter | Snow on the canopy (free); a fur muff and earmuffs on her, candles in the window |

## Performance

- **Idle:** a slow sway side to side, the gown's hem lagging and swinging, the bow's loops
  bouncing; the parasol turns on her shoulder.
- **Fidgets:** (1) checks herself in the hand mirror, pats her hood, blows herself a kiss;
  (2) fans herself with a gloved hand, a dramatic sigh; (3) spritzes perfume into the air and
  twirls through it.
- **greet:** a deep curtsy, the gown pooling.
- **purchase:** claps her hands under her chin, then blows the buyer a kiss (a little heart pops).
- **farewell:** a wave with the fingers only, "toodle-oo".
- **victory:** twirls with the parasol open overhead; **defeat:** swoons backward onto the couch,
  the back of her hand to her forehead.

## Silhouette check

Wide at the bottom, narrow at the waist, a bow and a disc (the parasol) at the top: the only
bell in the game.
