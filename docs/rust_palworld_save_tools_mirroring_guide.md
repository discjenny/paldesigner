# Rust Mirroring Guide: `palworld_save_tools` Decode/Encode Pipeline

Date: 2026-02-24  
Reference commits:
- `KrisCris/palworld-save-tools@480f1f631295e32bd9c9fa5be689eb335bf912a7`
- `KrisCris/Palworld-Pal-Editor@56ed6bec3e684545fd33b9ecf04227b518cf940d`

## Objective
Implement a Rust save pipeline that mirrors the proven Python behavior for:
- type-hinted GVAS parsing,
- custom raw-property decode/encode,
- planner-scope normalization extraction,
- deterministic changed-file round-trip export.

This guide is implementation instruction, not roadmap text.

## Non-Negotiable Target Behavior
- Parse `Level.sav` and `Players/*.sav` without long hint-discovery loops.
- Use explicit path->type hints equivalent to `PALWORLD_TYPE_HINTS`.
- Use explicit path->raw-codec handlers equivalent to `PALWORLD_CUSTOM_PROPERTIES`.
- Preserve unknown/out-of-scope fields for round-trip safety.
- Apply patch operations on decoded object graph, then re-encode changed files only.

## Scope Lock
- Planner scope decoding required:
  - `CharacterSaveParameterMap.Value.RawData`
  - `BaseCampSaveData.Value.RawData`
  - `BaseCampSaveData.Value.WorkerDirector.RawData`
  - `CharacterContainerSaveData.Value.Slots.Slots.RawData`
  - `WorkSaveData`
  - `GroupSaveDataMap`
- Out-of-scope raw domains must still be preserved as opaque pass-through bytes.

## Required Rust File Layout
Create/maintain these files:
- `src/server/src/save/paltypes.rs`
- `src/server/src/save/rawdata/mod.rs`
- `src/server/src/save/rawdata/character.rs`
- `src/server/src/save/rawdata/base_camp.rs`
- `src/server/src/save/rawdata/worker_director.rs`
- `src/server/src/save/rawdata/character_container.rs`
- `src/server/src/save/rawdata/work.rs`
- `src/server/src/save/rawdata/group.rs`
- `src/server/src/save/hint_registry.rs`
- `src/server/src/save/custom_registry.rs`
- `src/server/src/save/roundtrip.rs`

## Step 1: Mirror Type Hints Exactly
Implement `src/server/src/save/paltypes.rs` with:
- `pub static PALWORLD_TYPE_HINTS: &[(&str, &str)]`
- `pub static DISABLED_PROPERTIES: &[&str]`

Rules:
- Copy every path from Python `PALWORLD_TYPE_HINTS` exactly.
- Keep leading-dot Python paths (`.worldSaveData...`) in source constants.
- Build normalized runtime keys without leading dot in one function:
  - `fn normalize_hint_path(path: &str) -> String`
- Load hints into `HashMap<String, String>` before first parse attempt.
- Remove any key present in `DISABLED_PROPERTIES` from active decode map.

Definition of done:
- No auto-hint loop is required for known sample saves.
- Missing-hint pass count is `<= 2` on `gamesave.zip`.

## Step 2: Add Custom Raw Codec Registry
Implement `src/server/src/save/custom_registry.rs`:
- Define trait:
  - `trait RawCodec { fn decode(&[u8]) -> Result<serde_json::Value, String>; fn encode(&serde_json::Value) -> Result<Vec<u8>, String>; }`
- Define registry:
  - `HashMap<&'static str, &'static dyn RawCodec>`
- Register mirrored paths from `PALWORLD_CUSTOM_PROPERTIES`.

Required registry keys to include:
- `.worldSaveData.GroupSaveDataMap`
- `.worldSaveData.CharacterSaveParameterMap.Value.RawData`
- `.worldSaveData.ItemContainerSaveData.Value.RawData`
- `.worldSaveData.ItemContainerSaveData.Value.Slots.Slots.RawData`
- `.worldSaveData.CharacterContainerSaveData.Value.Slots.Slots.RawData`
- `.worldSaveData.DynamicItemSaveData.DynamicItemSaveData.RawData`
- `.worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.RawData`
- `.worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Value.RawData`
- `.worldSaveData.BaseCampSaveData.Value.RawData`
- `.worldSaveData.BaseCampSaveData.Value.WorkerDirector.RawData`
- `.worldSaveData.BaseCampSaveData.Value.WorkCollection.RawData`
- `.worldSaveData.BaseCampSaveData.Value.ModuleMap` (disabled by default)
- `.worldSaveData.WorkSaveData`
- `.worldSaveData.MapObjectSaveData`
- `.worldSaveData.GuildExtraSaveDataMap.Value.GuildItemStorage.RawData`
- `.worldSaveData.GuildExtraSaveDataMap.Value.Lab.RawData`

Rule:
- If path has no implemented codec yet, keep original bytes in opaque wrapper and mark `codec_status='passthrough'`.

## Step 3: Implement Planner-Critical Raw Decoders First
Implement exact byte-level codecs in `rawdata/*` for:
- `character` (player/pal SaveParameter raw object)
- `base_camp`
- `worker_director`
- `character_container`
- `work`
- `group`

Decoder contract for each codec:
- Input: raw bytes.
- Output:
  - decoded typed struct for known fields,
  - plus `unknown_tail` bytes if any remainder exists.
- Encoder must produce byte-identical output when no field is changed.

Definition of done:
- Decode->encode->decode loop returns identical known fields.
- No silent truncation of unknown bytes.

## Step 4: Replace Iterative Hint Discovery as Primary Path
In `normalize.rs` and any future import parser:
- Primary parse call must use full mirrored hint map first.
- Auto-discovery loop is fallback only and must:
  - log missing path,
  - append to in-memory cache,
  - never exceed 64 fallback passes for one file.

Required progress messages:
- `Decoding Level.sav wrapper payload`
- `Parsing Level.sav GVAS root`
- `Applying mirrored PALWORLD_TYPE_HINTS`
- `Fallback hint resolution pass X/Y` (only if fallback is used)
- `Decoding planner raw domains`

## Step 5: Add Stage Timers and Counters
Add struct `ParseMetrics` with:
- `decode_wrapper_ms`
- `parse_gvas_ms`
- `hint_pass_count`
- `hint_count_start`
- `hint_count_end`
- `character_map_total`
- `character_map_selected`
- `character_map_decoded`
- `basecamp_count`
- `container_count`

Persist metrics in DB:
- Add migration for `save_import_versions.parse_metrics_json JSONB`.
- Update import row at each major phase.

Definition of done:
- UI can display exact counts and not only generic phase text.

## Step 6: Round-Trip Preservation Model
Implement `roundtrip.rs` object model:
- `KnownDecoded<T>` for codec-decoded known fields.
- `OpaqueRaw { original_bytes: Vec<u8> }` for untouched domains.
- `HybridRaw<T> { known: T, opaque_unknown: Vec<u8> }` for partially decoded domains.

Export rules:
- Changed entity in changed file:
  - decode known + preserve unknown,
  - apply validated patch,
  - re-encode.
- Unchanged file:
  - copy byte-identical.

## Step 7: Differential Validation Against Python Reference
Create fixture runner:
- Input: same `Level.sav` into Python and Rust extractors.
- Compare planner-scope outputs:
  - player uid set,
  - pal instance id set,
  - base assignment `(base_id, pal_instance_id, slot/target)` set.

Allowed mismatch: zero.

If mismatch exists:
- fail test with specific path and first 10 differing ids.

## Step 8: Performance Gates
Add integration gate `tests/integration/import_perf.rs`:
- Fixture: `gamesave.zip`.
- Required:
  - `normalizing_entities` completes in `< 45s` on dev machine baseline.
  - hint fallback passes `<= 8`.
  - server `/health` remains responsive during normalization.

If gate fails:
- block merge.

## Step 9: Explicit Handling of Disabled Property
Property:
- `.worldSaveData.BaseCampSaveData.Value.ModuleMap`

Rule:
- keep disabled by default.
- preserve raw bytes untouched.
- do not fail import if decoding this path fails.
- add warning metric counter `disabled_property_skips`.

## Step 10: Concrete Execution Order
Implement in this exact order:
1. `paltypes.rs` full mirrored constants.
2. `hint_registry.rs` load/normalize/filter behavior.
3. `custom_registry.rs` path registry scaffold.
4. planner-critical raw codecs (`character`, `base_camp`, `worker_director`, `character_container`, `work`, `group`).
5. normalization parser migration to registry-driven decode.
6. parse metrics capture and DB persistence.
7. differential tests vs Python reference.
8. performance tests and gating.

Do not begin export patch writing before step 5 and step 7 are complete.

## Acceptance Criteria
- Import no longer stalls in hint pass loops on provided fixture.
- Granular progress shows stage and entity counts.
- Planner extracted counts match reference tool outputs for fixture.
- Health endpoint remains responsive during import.
- Unknown data preservation is verified by unchanged-file byte identity and changed-file targeted diffs only.
