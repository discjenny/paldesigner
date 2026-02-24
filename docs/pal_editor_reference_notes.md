# Pal Editor Reference Notes

Date captured: 2026-02-24

Primary references:
- `KrisCris/Palworld-Pal-Editor@56ed6bec3e684545fd33b9ecf04227b518cf940d`
- `KrisCris/palworld-save-tools@480f1f631295e32bd9c9fa5be689eb335bf912a7`

## 1) Save Read/Write Behavior in Reference Implementation

- `SaveManager.open()` reads `Level.sav`, decompresses to GVAS, then parses `worldSaveData` structures.
- `SaveManager.open()` also loads per-player files from `Players/{PLAYER_UID_HEX}.sav`.
- `SaveManager.save()` rewrites `Level.sav` and rewrites all loaded `Players/*.sav`.
- `SaveManager.save()` uses `compress_gvas_to_sav(..., 0x32, True)` for output in this version.
- The reference tool is object-graph rewrite, not file byte-diff patching.
- The reference tool preserves selected unknown sections by skip-decode/skip-encode passthrough.

## 2) Required Entity Paths (Observed)

- Level world root:
- `worldSaveData`
- Player and pal entries:
- `worldSaveData.CharacterSaveParameterMap`
- Player discriminator:
- `value.RawData.value.object.SaveParameter.value.IsPlayer == true`
- Player save identity cross-check:
- `Level.sav` key `PlayerUId` and `InstanceId`
- `Players/{PLAYER_UID_HEX}.sav` -> `SaveData.IndividualId`
- Group/guild records:
- `worldSaveData.GroupSaveDataMap`
- Base camps:
- `worldSaveData.BaseCampSaveData`
- Character containers:
- `worldSaveData.CharacterContainerSaveData`
- Work assignments:
- `worldSaveData.WorkSaveData`

## 3) Planner-Relevant Field Mapping (Observed)

Player and ownership:
- Player UID: `CharacterSaveParameterMap[*].key.PlayerUId`
- Player instance id: `CharacterSaveParameterMap[*].key.InstanceId`
- Group id on character object: `CharacterSaveParameterMap[*].value.RawData.value.group_id`

Guild and base linkage:
- Guild base ids: `GroupSaveDataMap[*].value.RawData.value.base_ids[]`
- Guild members: `GroupSaveDataMap[*].value.RawData.value.players[].player_uid`
- Guild pal handles: `GroupSaveDataMap[*].value.RawData.value.individual_character_handle_ids[].instance_id`

Base camp linkage:
- Base id: `BaseCampSaveData[*].value.RawData.value.id`
- Base owner guild: `BaseCampSaveData[*].value.RawData.value.group_id_belong_to`
- Base container id: `BaseCampSaveData[*].value.RawData.value.container_id`

Pal container placement:
- Container id: `CharacterContainerSaveData[*].key.ID`
- Slot rows: `CharacterContainerSaveData[*].value.Slots[]`
- Slot index: `...SlotIndex.value`
- Pal instance id: `...RawData.value.instance_id`

Pal ownership and placement:
- Owner player uid: `SaveParameter.value.OwnerPlayerUId`
- Prior owners: `SaveParameter.value.OldOwnerPlayerUIds[]`
- Slot id struct: `SaveParameter.value.SlotId` with container id + slot index
- Base worker heuristic used by reference code:
- Missing `OwnerPlayerUId` and non-empty `OldOwnerPlayerUIds` -> base worker candidate

## 4) Work Assignment Structures (From Pinned save-tools)

`worldSaveData.WorkSaveData` decodes:
- Work type: `WorkableType`
- Work raw data:
- `base_camp_id_belong_to`
- `owner_map_object_model_id`
- `owner_map_object_concrete_model_id`
- `assign_define_data_id`
- `assignable_fixed_type`
- Per-assignment map:
- `WorkAssignMap[].value.RawData.value.assigned_individual_id.instance_id`
- `WorkAssignMap[].value.RawData.value.fixed`

`worldSaveData.BaseCampSaveData.Value.WorkerDirector.RawData` decodes:
- `id`
- `spawn_transform`
- `current_order_type`
- `current_battle_type`
- `container_id`

`worldSaveData.BaseCampSaveData.Value.WorkCollection.RawData` decodes:
- `id`
- `work_ids[]`

## 5) Compression/Wrapper Details (Pinned save-tools)

- Save header parser supports:
- `PLZ` magic (zlib path)
- `PLM` magic (oodle path)
- Optional `CNK` prefix handling with shifted header/data offsets.
- Save type constants in pinned code:
- `0x31` (`PLM`) oodle path
- `0x32` (`PLZ`) zlib path

## 6) Known Caveat

- Pinned `palworld-save-tools` marks `.worldSaveData.BaseCampSaveData.Value.ModuleMap` as disabled for newer versions.
- Do not require ModuleMap parsing for initial planner scope.

## 7) Implementation Rules Derived for This Project

- Normalize only planner scope fields listed above.
- Persist raw file refs and raw entity paths for every normalized row.
- Store operations as patch intent records.
- Export mutates only targeted files with patch changes.
- Keep unknown fields unchanged for untouched entities.
- Export implementation for this project is `SAV -> GVAS decode -> object mutation -> GVAS -> SAV recompress` for changed files only.
- Save round-trip does not use JSON serialization.
- Player save directory contract for this project is strict `Players/` (plural) on import and export.
- `Player/` (singular) is treated as invalid input for this project.
