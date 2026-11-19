# The carried content — what is carried and what is written

Carried (before the SDK migration) by `tools/assemble_content.py` from the
Botania jar `ports/_shared/licence-cache/Botania-1.20.1-456-FORGE.jar`
(SHA-256 `3cf540357c9cb5e4…`; the full hash is in [`SOURCE.md`](SOURCE.md)).
The script and the old `content/` folder were retired with the `.hnmod` format; the
files now sit at the package root, `assets/botania/` and `data/`, and are packed into
`botania.hmod` by `cargo horizon-mod build`.

## Carried byte-for-byte from the jar

| File | Note |
|---|---|
| `assets/botania/blockstates/{endoflame,mana_spreader,mana_pool}.json` | one variant each, no state properties |
| `assets/botania/models/block/{endoflame,mana_spreader,mana_pool}.json` | the jar's block models |
| `assets/botania/models/block/shapes/{cross,spreader,mana_pool,spreader_item}.json` | the shape parents they name |
| `assets/botania/models/item/{endoflame,mana_spreader}.json` | the jar's item models |
| `assets/botania/textures/block/endoflame.png` | the flower |
| `assets/botania/textures/block/mana_spreader_{back,core,side}.png`, `livingwood_log.png`, `stripped_livingwood_log.png` | the spreader |
| `assets/botania/textures/block/mana_pool_{bottom,inside,side,top}.png` | the pool |

## Re-spelled for 26.3

| File | Change |
|---|---|
| `data/botania/loot_table/blocks/*.json` | the jar's `loot_tables/` path → 26.3's `loot_table/`; the pool's `conditions` **list** → one `condition` object (26.3 silently evaluates the 1.21.1 plural form as false, so every block would drop nothing); a `random_sequence` added |
| `assets/botania/items/{endoflame,mana_spreader,mana_pool}.json` | 26.3's item model definition (`{"model":{"type":"minecraft:model",...}}`) |
| `assets/botania/models/item/mana_pool.json` | its `overrides` block (the `botania:full` item property) is dropped: neither the property nor `block/mana_pool_full` is in this slice, and a model naming a missing parent fails the load |

## Written here (decisions, not upstream files)

| File | What and why |
|---|---|
| `horizon-mod.toml` | the content mod: id `botania`, API 0.2, `side = both`, `registries` |
| `data/botania/horizon/block/endoflame.json` | template **`minecraft:poppy`** — upstream is literally `BlockBehaviour.Properties.copy(Blocks.POPPY)` (`BotaniaFlowerBlocks.FLOWER_PROPS`), so the template's cross model, missing collision, soil placement and flower hardness are the mod's own. `occludes: false` |
| `data/botania/horizon/block/mana_spreader.json` | template `minecraft:stone` with upstream's `Properties.copy(livingwood)` numbers (hardness 2, resistance 2, wood sound) and upstream's `box(2, 2, 2, 14, 14, 14)` shape and collision |
| `data/botania/horizon/block/mana_pool.json` | template `minecraft:stone` with `Properties.copy(livingrock)`'s `strength(2, 10)`, stone sound and correct-tool-for-drops, and upstream's collision `box(0, 0, 0, 16, 8, 16)` |
| `assets/botania/lang/en_us.json` | the jar's names for the three blocks plus `botania.mana_pool.readout`, the port's one string of its own (upstream reads a pool with a wand HUD) |
| `data/minecraft/tags/block/mineable/{pickaxe,axe}.json` | the pool is pickaxe-mined, the spreader axe-mined (upstream's tools) |

The three blocks borrow **no vanilla machine's class**: the package declares one block
entity type per block in `src/lib.rs` (`Registrar::block_entity_type`), so the stone/poppy
templates are only properties, shapes and models.

## Deliberately not carried

- **Recipes.** `mana_pool` needs `botania:livingrock`, `mana_spreader` needs
  `#botania:petals` and `#botania:livingwood_logs` through the `botania:gog_alternation`
  type, and `endoflame` has no crafting recipe (the Petal Apothecary makes it). None of
  those exists in slice 1, so a carried recipe would be an ingredient that can never
  match — a defect, not content. The blocks are obtained with `/give` until the
  apothecary/livingrock/petals slice lands (recorded in `notes.md`).
- **The pool's `full` item model and the `mana_pool_full` block model** (see above).
- **The spreader's scaffolding and padding models** (`shapes/spreader_scaffolding`,
  `shapes/spreader_padding`): the properties and items that use them are out of slice.
- **Advancements, the lexicon's Patchouli pages, sounds, GUI textures**: out of slice.
- **Block colours / tinting**: the spreader model's faces carry `tintindex: 0`, which
  upstream tints per variant; with no block colour provider the game draws them untinted
  (white), which is what the default spreader's colour is.
