# 02 — Combat Math

All formulas here are implemented **once**, in the Rust simulation core (`mftr-sim`). The client, tooltips and tooling call the same functions. The sim keeps full precision internally; values are rounded **only for display**.

## 1. Stats

| Stat | Abbrev. | Notes |
|---|---|---|
| Health, Health regen | HP, HP5 | Regen is listed per 5 s, applied per tick |
| Resource, regen | MP, MP5 | Mana / Energy / Fury / none |
| Attack damage | AD | Base + bonus (the split matters for ratios) |
| Ability power | AP | |
| Armor | AR | Mitigates physical damage |
| Magic resist | MR | Mitigates magic damage |
| Attack speed | AS | Attacks per second |
| Critical strike chance / damage | Crit / CritDmg | CritDmg default 175% *(start)* |
| Ability haste | AH | |
| Movement speed | MS | |
| Attack range | | Center-to-edge, plus target gameplay radius |
| Life steal / Omnivamp | | |
| Lethality, % armor pen, flat/% magic pen | | See §4 |
| Heal & shield power | HSP | Amplifies heals/shields *given* |
| Tenacity | | Reduces CC duration |
| Gameplay radius | | Hitbox circle radius; changed by size effects (Mayhem) |

### Level growth

Base stats grow with level `n` (1–18) using a slightly accelerating curve:

```
stat(n) = base + growth × (n − 1) × (0.7025 + 0.0175 × (n − 1))
```

Attack speed growth is applied as a **bonus %** with the same curve. Champion data stores `base` and `growth` per stat.

### Adaptive Force

Some sources grant "Adaptive Force". It converts to **AD** (1 AF → 0.6 bonus AD) or **AP** (1 AF → 1 AP), whichever bonus the champion currently has more of (default: champion's data-defined preference).

## 2. Damage types

- **Physical**: mitigated by Armor.
- **Magic**: mitigated by Magic Resist.
- **True**: ignores resistances. Still affected by shields and, where specified, damage reduction.

Every damage instance carries **flags** used by on-hit and augment hooks:
`basic_attack`, `ability`, `proc`, `aoe`, `dot`, `pet`, `reflected`, `crit`, `item`, `true_source` (and so on).

## 3. Resistance mitigation

For resistance `R`:

```
if R ≥ 0:  multiplier = 100 / (100 + R)
if R < 0:  multiplier = 2 − 100 / (100 − R)
damage_taken = raw_damage × multiplier
```

| R | Multiplier | Damage taken |
|---|---|---|
| −20 | 1.167 | 116.7% |
| 0 | 1.000 | 100% |
| 50 | 0.667 | 66.7% |
| 100 | 0.500 | 50% |
| 200 | 0.333 | 33.3% |
| 300 | 0.250 | 25% |

**Effective health:** `EHP = HP × (1 + R/100)`. Each point of resistance adds 1% of max HP as effective health against that damage type. Resistance therefore has **no diminishing returns on survival time**. Going from 200 → 300 armor adds the same physical EHP as 0 → 100. The *percentage* reduction does diminish, which is why tooltips show EHP rather than "% reduced".

## 4. Reduction & penetration order

Resistance modifiers apply in a strict order. **Reduction** changes the target's real stat, benefits the whole team, and *can go below 0*. **Penetration** applies only to the attacker's own calculation and *cannot push the value below 0*.

1. **Flat reduction:** `R = R − flat_red` (may go negative)
2. **Percent reduction:** `if R > 0: R = R × (1 − pct_red)`
3. **Percent penetration:** `if R > 0: R = R × (1 − pct_pen)`
4. **Flat penetration:** `if R > 0: R = max(0, R − flat_pen)`

**Lethality** converts to flat armor penetration based on the *attacker's* level:

```
flat_armor_pen = lethality × (0.6 + 0.4 × level / 18)
```

### Worked example

Target has 120 armor. The attacker has 10 flat reduction, 20% reduction, 30% armor pen, and 15 flat armor pen (already converted from lethality).

| Step | Calculation | R |
|---|---|---|
| Start | | 120 |
| 1. Flat reduction | 120 − 10 | 110 |
| 2. % reduction | 110 × 0.8 | 88 |
| 3. % penetration | 88 × 0.7 | 61.6 |
| 4. Flat penetration | 61.6 − 15 | **46.6** |

Multiplier = 100 / 146.6 = **0.682**, so the target takes 68.2% of the raw damage, instead of 45.5% at 120 armor.

This must be a **golden test case** in `mftr-sim`, along with negative-R and the clamp-at-zero cases.

## 5. The damage pipeline

Every damage event resolves in this exact order:

1. **Raw damage:** base + Σ(ratio × stat). Ratios can reference AD, bonus AD, AP, max/bonus/missing HP (self or target), armor, MR, and so on.
2. **Critical strike** (if the source can crit): × CritDmg.
3. **Outgoing modifiers:** attacker's damage amps (e.g., "+10% damage to champions", wyrm buffs, augments).
4. **Resistance mitigation** (§3, §4), using the effective resistance.
5. **Incoming % damage reduction:** target's multiplicative reductions, combined as `Π(1 − r_i)`.
6. **Incoming flat damage reduction:** e.g., "−X damage from basic attacks". Floored at 0.
7. **Shields:** type-specific shields (magic-only, physical-only) are consumed first, then general shields. Oldest shield first.
8. **Health:** apply to HP, and check death / execute thresholds.
9. **Post-damage hooks:** lifesteal/omnivamp, on-damage triggers, damage-over-time refresh, assist tracking and combat log.

Each step emits to the **combat log**, which feeds the death recap, replay analysis and balancing tools.

## 6. Healing & shielding

```
heal = base_heal × (1 + healer_HSP) × (1 − grievous_reduction)
shield = base_shield × (1 + shielder_HSP)
```

Grievous-wounds style healing reduction does not stack; the strongest applies. *(start)* tiers: 25% / 40%.

## 7. Attacks

- **Attack speed:** `AS = base_AS × (1 + bonus_AS%)`, capped at 2.5 *(start)* except by explicit uncap effects.
- **Attack timer:** `1 / AS` seconds. Each champion has a **windup %** (the portion of the attack before damage or projectile launch). Windup scales with AS by a per-champion windup modifier.
- **Attack-move cancel** (orb-walking) is a skill expression and is **intended**. Cancelling during windup resets the attack with no damage.
- Ranged basic attacks are homing projectiles. Melee attacks hit on windup completion.
- **On-hit effects** trigger on basic attacks and on abilities flagged `applies_on_hit`.

## 8. Ability haste & cooldowns

```
cooldown = base_cooldown × 100 / (100 + AH)
```

Like resistance, haste scales linearly in "casts per minute": every 100 AH adds 100% more casts. Item and utility spell cooldowns use **summoner haste** / item haste separately.

## 9. Crowd control & tenacity

- **Tenacity** reduces the duration of most CC: stacking multiplicatively, `duration × Π(1 − t_i)`.
- **Airborne** (knock-up/back) and **Suppression** ignore tenacity and **cannot be cleansed** (except by specific "unstoppable" effects).
- **Slows** stack by taking the strongest only. Movement speed has soft caps, and a hard floor at 110 MS *(start)*.
- **CC immunity/unstoppable** states are explicit buffs with visible indicators. Readability: you must be able to *see* that a target can't be CC'd.

### Movement speed soft caps *(start)*
```
raw MS > 490: MS = raw × 0.5 + 230
raw MS > 415: MS = raw × 0.8 + 83
raw MS < 220: MS = raw × 0.5 + 110
```

## 10. Implementation notes

- All stats are computed through an **ordered modifier stack**: base → flat bonuses → % bonuses → conversions (adaptive, augments like "AD → AP") → caps. Conversion order must be explicit and data-defined to avoid circular dependencies.
- Stats are cached per entity and recomputed only when a modifier changes (dirty flag).
- **Determinism:** formulas use the sim's numeric policy (see [03](03-netcode.md#17-determinism-policy)). No platform-dependent math (use software `libm` for transcendentals).
- Every formula in this file gets a unit test with the table values above.
