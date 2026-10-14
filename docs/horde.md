# Mayhem: Horde

A five-champion PvE co-op mode on The Bridge, the existing Mayhem map. Pick **Mayhem:
Horde → Co-op Horde** in the server's mode list. Choose your champion through the existing
pick lobby; bots fill missing blue seats. There are no red champions or allied minion waves.

Defend the blue Base through **20 waves**. Every scheduled minion and boss must die before
a wave clears. Losing the Base before wave 20 is defeat. Clearing wave 20 announces victory
and keeps the same champions, items, augments and defenses for **endless survival**. When the
Base eventually falls, the run ends and the usual champion select starts a new match. The
campaign victory remains secured; endless does not count as another campaign defeat.

## Existing mechanics

- The Bridge's terrain, brush, relics and blue defenses; red structures and fountain removed.
- Melee, caster, siege and super minions use their existing lane movement and attack AI.
- **Wave 10: Azure Warden miniboss. Wave 20: Ember Brute boss.** These are the existing
  monsters, with lane movement and larger boss stats. Their existing Insight/Cinder buffs
  go to the whole team on death. Endless alternates them every ten waves.
- Mayhem's level 3 start, 1,400 gold, 4 gold/s, champion abilities, items, potions, four
  augment slots, rerolls, golden rerolls and Stat Anvils. Ability points are still spent
  normally; no new items, champions or augments.
- Drafts open **before waves 1, 6, 11 and 16**, rather than at champion levels. A choice left
  open is never replaced: missed milestones queue behind it. Spellhunger, Thorns, Reset and
  Champion of Chaos are excluded from offers and rerolls because their existing effects
  require enemy champions. Other Mayhem modes keep their original catalog and level drafts.
- Gold and shared XP from horde kills reach every blue champion, including dead players and
  supports. Tower kills pay too. Last hits still count as CS. Each cleared wave also pays
  every champion a completion reward.
- Blue's fountain heals here. **B** recalls home in eight seconds; shop at home or while
  dead using the existing shop key. Blue turrets stay destroyed and the Gatehouse uses its
  existing respawn timer. Defenses are not rebuilt between waves.

## First balance pass

Designed for five champions, including bot-filled seats. There is a 30-second preparation
period, six minions per spawn batch every 1.5 seconds, and a 20-second break after clears.
Breaks are 30 seconds before a boss or new augment and 40 seconds after campaign victory.
There are no overlapping numbered waves and no time limit on clearing a wave.

| Wave | Melee | Casters | Siege | Super | Boss |
|---|---:|---:|---:|---:|---|
| 1 | 6 | 4 | 0 | 0 | — |
| 5 | 14 | 8 | 1 | 0 | — |
| 10 | 24 | 13 | 3 | 0 | Azure Warden |
| 15 | 34 | 18 | 5 | 2 | — |
| 20 | 44 | 23 | 6 | 3 | Ember Brute |

Regular minion health is its existing base health times `0.75 + 0.07 × (wave - 1)`;
attack damage times `0.65 + 0.075 × (wave - 1)`. Armor gains 1.5/wave and magic resist
1/wave. Movement gradually rises from 325 to 425. Early hordes give time to learn abilities
and buy a first finished item; siege and super minions arrive as builds and augments grow.

The Warden has 10,000 health, 35 armor/MR and 95 attack damage at 0.7 attacks/s. The Brute
has 32,000 health, 60 armor/MR and 220 attack damage at 0.75 attacks/s, with a larger body.
Both use existing attacks and remain vulnerable to the existing crowd control.

Every clear pays `140 + 18 × min(wave, 20)` gold and `200 + 55 × min(wave, 20)` XP per
champion, on top of shared kills and passive income. Boss clears add 250/750 gold. The aim
is level 6 in the early waves, roughly level 11 entering the miniboss, and level 18 with a
late-game build and four augments entering wave 20. Faster clearing reduces passive gold;
completion rewards and kill rewards still provide progression.

Endless continues raising enemy stats and adding minions up to a cap of 116 regular units
plus one boss. Health stays within the existing 16-bit network health range. Bosses return
every ten waves and grow stronger. Existing Stat Anvils give full builds a use for late gold.

## Hosting and checking

The normal server adds Horde to its default queues. `--modes horde` or `horde-coop` offers
only Horde; `horde-versus` is rejected. A development server can use `--scenario horde`.
Custom ARAM maps receive the same defense adaptation and have their own Horde queue.

```powershell
cargo build -p mftr-gdext
cargo run --release -p mftr-server -- --bind 127.0.0.1:7777 --modes horde
cargo test --workspace --release
cargo run --release -p mftr-tools -- botmatch --horde --seed 1 --minutes 40
cargo run --release -p mftr-tools -- botmatch --horde --team ember,vesper,bastion,rook,lumen --seed 3
```

The bot report includes clear times, levels, wallet plus inventory value, Base health and
replay verification. The initial six seeded balanced-team runs cleared wave 20 in
24:32–25:43 with about 18,700–19,000 gold plus inventory value per champion after the final
reward, then lost the Base in endless. This is an initial tuning baseline; human team
composition, positioning and augment choices still need co-op playtesting.

Three additional melee, mage and mixed compositions also cleared the campaign, in
25:45–26:25. The mage group finished wave 20 with only 100 Base health, while the other two
had 4,000 and 3,560, so composition and defensive positioning materially affect the margin.
All nine runs reproduced their state hashes exactly, including chosen-champion takeovers.

Horde progress is sent in a small field on every snapshot, separately from the optional
scoreboard, so crowded waves and reconnects retain the objective, countdown and boss health.
Protocol **23** adds the mode, rules flag, wave progress and campaign-victory event. Restart
the server and client together after rebuilding.
