# Project Progress and Implementation Roadmap

## Usage Rule
- This file is mandatory to update after every code or data change.
- Every completed task must be checked in the same change that implements it.
- Every newly discovered task must be added in the correct phase before continuing.
- No ambiguous task text is allowed.

## Current Status
- Date baseline: 2026-02-24
- Phase in progress: Phase 3
- Blockers: None

## Locked Decisions
- Frontend: React + TypeScript + Vite + Bun + Tailwind + shadcn/ui + Lucide.
- Backend API/webserver: Rust.
- Database: PostgreSQL in development and production.
- Save transfer format: ZIP only.
- Save processing model: patch-on-original (never rebuild entire save from normalized tables alone).
- Conversion scope: player, pals, base pal assignments, and planner-required fields only.
- Excluded conversion scope: structure placement/geometry and non-planner world simulation domains.
- Save artifact policy: all imported and exported artifacts are immutable and versioned.
- ZIP import supports nested-root auto-detection.
- ZIP import requires `Players/` directory name (plural) and rejects `Player/`.
- ZIP export root layout is fixed:
- `Level.sav`
- `LevelMeta.sav`
- `LocalData.sav`
- `WorldOption.sav`
- `Players/`
- Extra input files are ignored and excluded from export.
- Only files with actual patch changes are modified.
- Save mutation style is `SAV -> GVAS decode -> object mutation -> GVAS -> SAV recompress` for changed files only.
- Artifact retention is forever.
- Artifact hashes are SHA-256 and XXH64.
- Proprietary Oodle runtime (`oo2core_9_win64.dll`) is not allowed as a runtime dependency.
- `PlM` decode must use open-source Oodle-compatible backends.
- Changed-file recompression target format is `PlZ` (`0x32`, zlib).
- Save import/export runtime pipeline is native Rust only.
- Save runtime decode/re-encode does not use Python bridge processes.
- Parser optimization decision: defer parse-scope branch skipping until importer/exporter/patch tooling is complete; current stable parse baseline (`decode ~339ms`, `gvas parse ~11.7s`, `hint passes = 0`) is accepted for now.

## Non-Negotiable Architecture Rules
- Import accepts ZIP archives containing multi-file save sets.
- ZIP must include `Level.sav` and at least one `Players/*.sav`.
- ZIP import accepts `Players/` directory name only.
- ZIP import rejects `Player/` directory name.
- Import must auto-detect valid world root from nested ZIP paths.
- Extracted files from import are stored as immutable raw artifacts.
- Normalized planner rows must store explicit links to raw artifact references.
- User edits are stored as validated patch operations.
- Export always starts from one immutable import version + one patchset.
- Export outputs a new immutable ZIP artifact.
- Unknown/untouched fields in source saves must round-trip unchanged.
- Export output contains only required files and `Players/` folder.
- Export ignores extra input files.
- Export mutates targeted files only when patch changes exist.
- Export decodes, mutates, re-encodes, and recompresses only targeted changed files.
- Export copies untouched files byte-identical from source import artifacts.

## Pal Editor Reference Findings (Normative)
- Reference repository: `KrisCris/Palworld-Pal-Editor`, commit `56ed6bec3e684545fd33b9ecf04227b518cf940d` (2025-12-23).
- Referenced save library pin in that repo: `KrisCris/palworld-save-tools`, commit `480f1f631295e32bd9c9fa5be689eb335bf912a7`.
- `Level.sav` parsing root path in reference implementation: `worldSaveData`.
- Player/pal entity collection path in reference implementation: `worldSaveData.CharacterSaveParameterMap`.
- Entity discriminator in reference implementation: `value.RawData.value.object.SaveParameter.value.IsPlayer`.
- Player identity join rule from reference implementation:
- `Level.sav` player key `PlayerUId` and `InstanceId` must match `Players/{PLAYER_UID_HEX}.sav` path and `SaveData.IndividualId`.
- Guild data source path in reference implementation: `worldSaveData.GroupSaveDataMap`.
- Guild membership fields used by reference implementation:
- `RawData.value.group_id`
- `RawData.value.players[].player_uid`
- `RawData.value.base_ids[]`
- `RawData.value.individual_character_handle_ids[].instance_id`
- Base camp source path in reference implementation: `worldSaveData.BaseCampSaveData`.
- Base camp fields used by reference implementation:
- `RawData.value.id`
- `RawData.value.group_id_belong_to`
- `RawData.value.container_id`
- Character container source path in reference implementation: `worldSaveData.CharacterContainerSaveData`.
- Character container fields used by reference implementation:
- `key.ID` (container id)
- `value.Slots[].RawData.value.instance_id` (pal instance id)
- `value.Slots[].SlotIndex.value` (slot index)
- Base worker heuristic used by reference implementation:
- pal with missing `OwnerPlayerUId` and non-empty `OldOwnerPlayerUIds` is treated as base worker candidate.
- Work assignment data paths available in pinned save library:
- `worldSaveData.WorkSaveData` (workable records, per-work assignments, fixed assignment flag, target ids)
- `worldSaveData.BaseCampSaveData.Value.WorkerDirector.RawData` (base worker director container and order bytes)
- `worldSaveData.BaseCampSaveData.Value.WorkCollection.RawData` (work id collection per base)
- Work assignment fields decoded by pinned save library:
- `WorkableType`
- `WorkAssignMap[].value.RawData.value.assigned_individual_id.instance_id`
- `WorkAssignMap[].value.RawData.value.fixed`
- `RawData.value.base_camp_id_belong_to`
- `RawData.value.owner_map_object_model_id`
- `RawData.value.owner_map_object_concrete_model_id`
- `RawData.value.assign_define_data_id`
- `RawData.value.assignable_fixed_type`
- Save write behavior in reference implementation:
- rewrites `Level.sav` and rewritten `Players/*.sav` files from parsed object graph.
- compresses output with `compress_gvas_to_sav(..., 0x32, True)` in current version.
- retains selected uninterpreted blobs via skip-decode/skip-encode property passthrough.
- Path detection behavior in reference implementation:
- save folder is considered valid when directory contains `Level.sav` and `Players` directory.
- Known parser caveat from pinned save library:
- `.worldSaveData.BaseCampSaveData.Value.ModuleMap` is listed as disabled for newer versions.

## Canonical Repository Layout (Must Match)
- `AGENTS.md`
- `PROGRESS.md`
- `data/`
- `data/raw/`
- `data/json/`
- `docs/`
- `docs/pal_editor_reference_notes.md`
- `scripts/`
- `src/server/`
- `src/server/Cargo.toml`
- `src/server/src/main.rs`
- `src/server/src/config.rs`
- `src/server/src/db/mod.rs`
- `src/server/src/db/migrations/`
- `src/server/src/api/routes.rs`
- `src/server/src/api/handlers/`
- `src/server/src/save/zip.rs`
- `src/server/src/save/detect.rs`
- `src/server/src/save/parse.rs`
- `src/server/src/save/normalize.rs`
- `src/server/src/save/patch.rs`
- `src/server/src/save/export.rs`
- `src/server/src/storage/fs.rs`
- `src/web/`
- `src/web/package.json`
- `src/web/src/`
- `src/web/src/lib/api.ts`
- `src/web/src/lib/types.ts`
- `src/web/src/store/`
- `docker/`
- `docker/docker-compose.dev.yml`
- `docker/docker-compose.prod.yml`
- `docker/server.Dockerfile`
- `tests/fixtures/`
- `tests/integration/`

## Save ZIP Contract (Normative)
- Import request body contains exactly one ZIP file.
- ZIP file extension must be `.zip`.
- ZIP file size must be checked before extraction.
- ZIP entries are extracted with path traversal protection.
- Import root detection rules:
- Traverse ZIP paths and detect first candidate root containing:
- `Level.sav`
- `Players/*.sav`
- Reject candidate root if player save directory is named `Player/`.
- Reject import when no valid root exists.
- Supported output entries (exact export layout):
- `Level.sav`
- `LevelMeta.sav`
- `LocalData.sav`
- `WorldOption.sav`
- `Players/*.sav`
- All extracted file paths are persisted in file manifest records.
- Unsupported files are marked ignored and excluded from export ZIP.

## Raw Artifact Storage Contract (Normative)
- Imported ZIP storage key format:
- `storage/imports/{import_version_id}/source.zip`
- Extracted file storage key format:
- `storage/imports/{import_version_id}/files/{relative_path}`
- Exported ZIP storage key format:
- `storage/exports/{export_version_id}/export.zip`
- Exported extracted file storage key format:
- `storage/exports/{export_version_id}/files/{relative_path}`
- Every artifact record stores:
- SHA-256 checksum
- XXH64 checksum
- byte size
- UTC created timestamp
- immutable flag
- retention policy (`forever`)

## Normalized Conversion Scope Contract (Normative)
Only these domains are normalized into planner entities:
- Player planner identity fields.
- Pal planner state fields.
- Base roster and pal assignment fields.
- Planner-required derived values for production calculations.

Required normalized player fields:
- `import_version_id`
- `player_uid`
- `player_instance_id`
- `player_name`
- `guild_id`
- `level`
- `raw_file_ref`
- `raw_entity_path`

Required normalized pal fields:
- `import_version_id`
- `pal_instance_id`
- `owner_player_uid`
- `species_id`
- `nickname`
- `gender`
- `level`
- `exp`
- `rank`
- `rank_hp`
- `rank_attack`
- `rank_defense`
- `rank_craftspeed`
- `talent_hp`
- `talent_melee`
- `talent_shot`
- `talent_defense`
- `passive_skill_ids[]`
- `mastered_waza_ids[]`
- `equip_waza_ids[]`
- `work_suitability_ranks` (JSON object keyed by work type)
- `status_hp`
- `status_sanity`
- `status_hunger`
- `worker_sick`
- `revive_timer`
- `raw_file_ref`
- `raw_entity_path`

Required normalized base assignment fields:
- `import_version_id`
- `base_id`
- `pal_instance_id`
- `assignment_kind`
- `assignment_target`
- `priority`
- `raw_file_ref`
- `raw_entity_path`

## Patch Operation Contract (Normative)
Allowed operation types:
- `update_player_field`
- `update_pal_field`
- `replace_pal_passive_list`
- `replace_pal_mastered_waza_list`
- `replace_pal_equipped_waza_list`
- `replace_pal_work_suitability_map`
- `upsert_base_assignment`
- `delete_base_assignment`
- `create_base`
- `delete_base`

Required patch operation columns:
- `patchset_id`
- `sequence`
- `op_type`
- `target_kind`
- `target_id`
- `payload_json`
- `validated`
- `validation_error`

Validation rules:
- Operation sequence must be strictly increasing starting at 1.
- Invalid operation type is rejected.
- Unknown target entity is rejected.
- Out-of-bounds base-game values are rejected.
- Cross-field invalid states are rejected.
- Unvalidated operations cannot be applied during export.

## API Contract (Normative)
Base path:
- `/api/v1`

Required endpoints:
- `POST /save/import-zip`
- `GET /save/import-versions/{id}`
- `GET /save/import-versions/{id}/events`
- `GET /save/import-versions/{id}/normalized`
- `POST /save/import-versions/{id}/patchsets`
- `GET /save/patchsets/{id}`
- `POST /save/import-versions/{id}/exports`
- `GET /save/export-versions/{id}`
- `GET /save/export-versions/{id}/download`
- `GET /health`
- `GET /ready`

Endpoint behavior requirements:
- Import endpoint returns `import_version_id` after artifacts/manifests/seed rows persist and background decode-normalize job is queued.
- Import progress endpoint streams deterministic SSE phases (`progress`, `done`, `progress_error`) until terminal status.
- Patchset creation endpoint validates and stores all operations atomically.
- Export endpoint applies exactly one patchset to exactly one import version.
- Download endpoint returns ZIP binary with checksum header.

## PostgreSQL Schema Contract (Normative)
Required tables:
- `save_import_versions`
- `save_export_versions`
- `save_zip_artifacts`
- `save_files`
- `save_variant_metadata`
- `save_patchsets`
- `save_patch_operations`
- `save_export_lineage`
- `planner_players`
- `planner_pals`
- `planner_base_assignments`
- `planner_player_links`
- `planner_pal_links`
- `planner_base_assignment_links`

Required key constraints:
- Every table has UUID primary key.
- Every child table has foreign key with `ON DELETE RESTRICT` unless explicitly archival.
- `save_patch_operations` unique constraint on `(patchset_id, sequence)`.
- `save_export_lineage` unique constraint on `export_version_id`.
- `planner_*_links` must reference both normalized row and raw file artifact row.

## Rust Backend Implementation Standard
- Runtime: Tokio.
- HTTP framework: Axum.
- DB layer: SQLx with compile-time checked queries.
- Serialization: Serde.
- ZIP handling: `zip` crate with explicit path sanitization.
- Hashing: SHA-256 via Rust crypto crate.
- Logging: `tracing` + `tracing-subscriber` JSON format.
- Config: environment variables only, loaded at startup and validated.

## Frontend Implementation Standard
- Use Bun for install, scripts, and runtime.
- Vite dev server must proxy API requests to Rust server.
- Frontend never parses `.sav` directly.
- Frontend only uploads ZIP, reads normalized API payloads, sends patch operations, and downloads export ZIP.

## Phase 0: Foundation and Contracts
Definition of done:
- All normative contracts in this file are implemented as code-level schema/docs stubs.

Tasks:
- [x] Create `AGENTS.md`.
- [x] Create `PROGRESS.md`.
- [x] Lock stack and architecture decisions.
- [x] Lock requirement: nested-root auto-detect for ZIP import.
- [x] Lock requirement: fixed export ZIP layout.
- [x] Lock requirement: ignore extra files.
- [x] Lock requirement: changed-file-only decode/mutate/re-encode/recompress.
- [x] Lock requirement: forever retention.
- [x] Lock requirement: SHA-256 + XXH64 hashes.
- [x] Add `src/server/README.md` with exact dev run commands.
- [x] Add `src/web/README.md` with exact dev run commands.
- [ ] Add `docs/patch_operation_schema.json`.
- [ ] Add `docs/save_zip_contract.md`.
- [ ] Add `docs/normalized_scope_contract.md`.
- [ ] Add `docs/export_lineage_contract.md`.
- [x] Add `docs/pal_editor_reference_notes.md` with implementation notes from `https://github.com/KrisCris/Palworld-Pal-Editor`.
- [x] Add `docs/rust_palworld_save_tools_mirroring_guide.md` with exact Rust implementation instructions to mirror `palworld_save_tools` type hints, raw codecs, round-trip model, metrics, and validation gates.

## Phase 1: Repository Scaffolding
Definition of done:
- All required directories and bootstrap files exist.

Tasks:
- [x] Create Rust workspace under `src/server/`.
- [x] Create Axum app entrypoint in `src/server/src/main.rs`.
- [x] Create DB module skeleton in `src/server/src/db/mod.rs`.
- [x] Create migration folder `src/server/src/db/migrations/`.
- [x] Create API routing skeleton in `src/server/src/api/routes.rs`.
- [x] Create save modules (`zip.rs`, `detect.rs`, `parse.rs`, `normalize.rs`, `patch.rs`, `export.rs`) under `src/server/src/save/`.
- [x] Create storage module `src/server/src/storage/fs.rs`.
- [x] Create Vite React app under `src/web/` using Bun.
- [x] Add API client module `src/web/src/lib/api.ts`.
- [x] Add shared frontend types `src/web/src/lib/types.ts`.
- [x] Configure Vite `/api` proxy rewrite to backend root paths.
- [x] Extend frontend API client with import upload and import-version inspection endpoints.
- [x] Extend frontend shared API types for import-version, file metadata, and normalized payload viewer surfaces.

## Phase 2: PostgreSQL Migrations and Models
Definition of done:
- All required tables exist with keys and constraints.

Tasks:
- [x] Create migration `0001_save_versions.sql` for import/export version tables.
- [x] Create migration `0002_save_artifacts.sql` for ZIP and file manifests.
- [x] Create migration `0003_patchsets.sql` for patchset and operations.
- [x] Create migration `0004_normalized_entities.sql` for planner players/pals/assignments.
- [x] Create migration `0005_links.sql` for normalized-to-raw link tables.
- [x] Create migration `0006_lineage.sql` for export lineage table.
- [x] Create migration `0007_import_progress.sql` for phased import progress tracking columns (`progress_phase`, `progress_pct`, `progress_message`, `failed_error`).
- [x] Create migration `0008_import_parse_metrics.sql` for parser telemetry column (`parse_metrics_json JSONB`).
- [x] Run SQLx migrations automatically at Rust server startup.
- [ ] Add Rust model structs for every table.
- [ ] Add integration test that migrates empty DB and verifies all tables/constraints exist.

## Phase 3: ZIP Import Pipeline (Rust)
Definition of done:
- ZIP import endpoint stores immutable source artifacts and normalized records.

Tasks:
- [x] Implement multipart upload handler `POST /api/v1/save/import-zip`.
- [x] Compute SHA-256, XXH64, and size for uploaded ZIP.
- [x] Persist ZIP artifact file to import storage key.
- [x] Extract ZIP with path sanitization and reject traversal entries.
- [x] Auto-detect nested world root and validate required files (`Level.sav`, `Players/*.sav`).
- [x] Persist extracted files and manifests.
- [x] Mark unsupported extra files as ignored.
- [x] Detect wrapper/compression variant for each target `.sav`.
- [x] Persist variant metadata rows.
- [x] Move heavy decode/normalization work to asynchronous post-upload processing to avoid long blocking import requests.
- [x] Add SSE progress endpoint for import processing phases.
- [x] Implement `PlM` (`0x31`) GVAS decode in Rust with `oozextract` (no `oo2core`).
- [ ] Parse planner-scope entities from extracted files.
- [x] Parse planner-scope entities from extracted files.
- [ ] Document discovered mapping for base pal assignment fields and work target fields using real save inspection plus Pal Editor reference.
- [x] Persist normalized planner entities.
- [x] Persist normalized-to-raw link rows.
- [x] Return `import_version_id`.
- [x] Implement `GET /api/v1/save/import-versions` and `GET /api/v1/save/import-versions/{id}` for persisted artifact/variant inspection.
- [x] Extend import version API payloads with `progress_phase`, `progress_pct`, `progress_message`, and `failed_error`.
- [x] Extend import version API payloads with `parse_metrics_json` for parse timing/count diagnostics.
- [x] Seed initial normalized player rows from `Players/<uid>.sav` file names with raw file linkage.

## Phase 4: Normalization and Planner Projection
Definition of done:
- API returns deterministic normalized projection for a given import version.

Tasks:
- [x] Implement `GET /api/v1/save/import-versions/{id}/normalized`.
- [x] Include players, pals, base assignments, and planner-required fields only.
- [x] Include stable IDs for all normalized rows.
- [x] Include raw link references for every normalized row.
- [ ] Add snapshot test for normalized payload determinism.

## Phase 5: Patchset API and Validation
Definition of done:
- Patchsets can be created and validated atomically.

Tasks:
- [ ] Implement `POST /api/v1/save/import-versions/{id}/patchsets`.
- [ ] Validate operation sequence and operation type.
- [ ] Validate target existence against normalized projection.
- [ ] Validate base-game legal bounds for every edited field.
- [ ] Persist patchset and operations in one DB transaction.
- [ ] Implement `GET /api/v1/save/patchsets/{id}`.
- [ ] Add negative tests for each validation error class.

## Phase 6: Export Pipeline (Rust)
Definition of done:
- Export applies patchset to immutable source version and returns ZIP.

Tasks:
- [ ] Implement `POST /api/v1/save/import-versions/{id}/exports`.
- [ ] Load source artifacts for the import version.
- [ ] Load and validate referenced patchset is already validated.
- [ ] Parse canonical source save objects retaining unknown fields.
- [ ] Apply operations in sequence order.
- [ ] Compute targeted changed file set.
- [ ] For each targeted changed file, run `SAV -> GVAS decode -> object mutation -> GVAS -> SAV recompress`.
- [ ] Leave untouched files byte-identical.
- [ ] Build export ZIP with exact root layout:
- [ ] `Level.sav`
- [ ] `LevelMeta.sav`
- [ ] `LocalData.sav`
- [ ] `WorldOption.sav`
- [ ] `Players/*.sav`
- [ ] Exclude ignored extra files from export ZIP.
- [ ] Re-pack deterministic ZIP ordering and timestamp policy.
- [ ] Persist export ZIP artifact and file manifest.
- [ ] Persist export lineage row linking import, patchset, export.
- [ ] Implement `GET /api/v1/save/export-versions/{id}/download`.
- [ ] Add round-trip fidelity tests for untouched fields.

## Phase 7: Planner Calculation Engine
Definition of done:
- Planner computes deterministic throughput and work demand from normalized data.

Tasks:
- [ ] Implement TypeScript calc module `web/src/lib/calc/index.ts`.
- [ ] Implement work demand by work type.
- [ ] Implement pal work contribution using suitability + modifiers.
- [ ] Implement building throughput computations for planner-supported production nodes.
- [ ] Implement target-output solver (`required work power`, `required assignments`).
- [ ] Add fixture tests for Ore and Wheat scenarios.

## Phase 8: Planner UI
Definition of done:
- User can import ZIP, edit planner entities, and export ZIP.

Tasks:
- [x] Add a minimal import-version viewer page showing persisted import versions, file decode metadata, and normalized row counts.
- [x] Add phased frontend progress bar showing upload phase and server decode/normalize phases via SSE.
- [ ] Implement Import ZIP page.
- [ ] Implement Import Version selector.
- [ ] Implement Players and Pals editor views.
- [ ] Implement Base assignment editor view.
- [ ] Implement Production target panel.
- [ ] Implement Patch preview panel.
- [ ] Implement Export ZIP action and download.
- [ ] Render in-game icons for pals/work/buildings/items where available.

## Phase 9: Development Environment
Definition of done:
- One deterministic dev startup command path works on a fresh machine.

Tasks:
- [ ] Add `docker/docker-compose.dev.yml` with PostgreSQL service.
- [ ] Add `.env.example` for Rust server and web frontend.
- [ ] Add `Makefile` with exact commands:
- [ ] `dev-db-up`
- [ ] `dev-server`
- [ ] `dev-web`
- [ ] Add health/readiness checks.
- [ ] Add startup verification script that fails fast on missing env vars.

## Phase 10: Production Containerization
Definition of done:
- Production containers run Rust API + PostgreSQL and support import/export workflow.

Tasks:
- [ ] Add `docker/server.Dockerfile` for Rust binary image.
- [ ] Add `docker/docker-compose.prod.yml`.
- [ ] Add DB migration run step to server startup.
- [ ] Mount persistent storage volume for artifact files.
- [ ] Mount persistent PostgreSQL volume.
- [ ] Add production healthchecks.

## Phase 11: Test Matrix and Release Gates
Definition of done:
- All release gates pass with no unresolved critical defects.

Tasks:
- [ ] Unit tests: save detection, normalization, patch validation, export serialization.
- [ ] Integration tests: ZIP import -> normalized -> patchset -> export -> download.
- [ ] Regression tests: untouched-field fidelity across round-trip.
- [ ] API contract tests: request/response schema validation.
- [ ] Performance tests: import/export for large saves.
- [ ] Security tests: zip traversal rejection, malformed payload rejection.
- [ ] Import tests: nested-root auto-detection success and invalid-root rejection.
- [ ] Export tests: exact root layout and required files present.
- [ ] Export tests: ignored extra files are absent from export ZIP.
- [ ] Export tests: only targeted changed files differ; untouched files remain byte-identical.
- [ ] Release checklist documented and executed.

## Immediate Next Tasks (Execution Queue)
- [x] Implement PostgreSQL migrations `0001` through `0003`.
- [x] Implement `POST /api/v1/save/import-zip` happy path.
- [x] Implement minimal normalized payload endpoint.
- [ ] Add integration tests for import endpoint ZIP validation and artifact persistence.
- [x] Implement and validate Rust `PlM` decode path with `oozextract` on `gamesave.zip`.
- [x] Implement planner-scope entity extraction from decoded `Level.sav` + `Players/*.sav`.
- [x] Add frontend viewer for `import-versions` detail + normalized payload inspection.
- [ ] Add frontend visualization for import SSE phases with deterministic status text and retry UX.
- [x] Extend frontend types to include phased import progress fields and SSE progress event contract.
- [x] Replace frontend import upload call with XHR-based upload progress reporting and SSE subscription utility.
- [x] Implement mirrored Rust hint registry (`PALWORLD_TYPE_HINTS` + disabled-property filter) and use it as primary parse path.
- [x] Implement mirrored custom codec registry keys with planner-critical decoders (`base_camp`, `worker_director`, `character_container`) and passthrough wrappers for remaining domains.
- [x] Persist parser telemetry (`decode_wrapper_ms`, `parse_gvas_ms`, hint passes/counts, character/base/container counters) to `save_import_versions.parse_metrics_json`.
- [ ] Implement full decode+encode parity codecs for `character`, `group`, and `work` raw domains (current state is passthrough-preserving wrappers).
- [ ] Add differential validation test runner comparing Rust planner projections to Python reference fixture outputs.

## Decisions Log
- 2026-02-24: Stack fixed to Bun frontend + Rust webserver + PostgreSQL + Docker. Python scripts use `uv`.
- 2026-02-24: Development mode requires PostgreSQL.
- 2026-02-24: Save import/export uses ZIP archives containing multi-file Palworld save sets.
- 2026-02-24: Save handling uses patch-on-original model.
- 2026-02-24: Imported and exported artifacts are immutable versioned records.
- 2026-02-24: Conversion scope is planner-only and excludes structure placement/geometry.
- 2026-02-24: ZIP import supports nested roots via auto-detection.
- 2026-02-24: Export ZIP layout is fixed to `Level.sav`, `LevelMeta.sav`, `LocalData.sav`, `WorldOption.sav`, and `Players/`.
- 2026-02-24: Extra files are ignored and excluded from export.
- 2026-02-24: Export modifies targeted changed files only using `SAV -> GVAS decode -> object mutation -> GVAS -> SAV recompress`.
- 2026-02-24: Retention policy is forever.
- 2026-02-24: Artifact hash policy uses both SHA-256 and XXH64.
- 2026-02-24: Palworld-Pal-Editor repository is a required implementation reference for patch behavior and save mapping.
- 2026-02-24: Reference inspection locked to `KrisCris/Palworld-Pal-Editor@56ed6be` and `KrisCris/palworld-save-tools@480f1f6` for initial mapping.
- 2026-02-24: Import/export player save directory name is locked to `Players/` only; `Player/` is rejected.
- 2026-02-24: Local development database `paldesigner` is created and validated with credentials `postgres:postgres`.
- 2026-02-24: Rust `/ready` check uses `SELECT 1` decoded as `i32` to match PostgreSQL scalar type.
- 2026-02-24: Server dependencies now include Axum multipart, SQLx migrations, ZIP handling, SHA-256, XXH64, and PLZ zlib decode support for importer implementation.
- 2026-02-24: Server config now includes `ARTIFACT_STORAGE_ROOT` and `MAX_IMPORT_ZIP_BYTES` for importer storage and upload validation.
- 2026-02-24: `.env.example` now documents importer storage root and max ZIP size settings.
- 2026-02-24: Default `ARTIFACT_STORAGE_ROOT` is now `.` so logical storage keys persist at `storage/...` exactly per contract.
- 2026-02-24: `/api/v1/save/import-zip` now persists immutable import ZIP + extracted files, enforces nested-root detection rules, and writes SAV wrapper/parse metadata.
- 2026-02-24: Import API response now includes a serializable normalized summary placeholder (`players`, `pals`, `assignments` counts = `0` until planner parsing is implemented).
- 2026-02-24: Axum default body size limit is disabled to support large multipart ZIP uploads; importer enforces explicit `MAX_IMPORT_ZIP_BYTES` at handler level.
- 2026-02-24: `gamesave.zip` import verified end-to-end (`201`), persisted 1 source ZIP + 6 extracted files + 6 variant rows; sample save variant detected as `PLM` (`save_type=0x31`, Oodle), so GVAS decode remains `not_attempted` pending Oodle support.
- 2026-02-24: Current Rust importer build status is clean (`cargo fmt`, `cargo check`), with only expected placeholder dead-code warnings in `save/export.rs` and `save/patch.rs`.
- 2026-02-24: `src/server/README.md` now includes a concrete `curl` command for manual `POST /api/v1/save/import-zip` validation.
- 2026-02-24: Save pipeline decision locked: no `oo2core` dependency; decode `PlM` with open-source backend and recompress changed files as `PlZ` (`0x32`).
- 2026-02-24: Save runtime policy locked to native Rust only; Python remains optional for offline tooling scripts and is not part of runtime decode/re-encode.
- 2026-02-24: Rust server now includes `oozextract` dependency to implement native `PlM` decode without proprietary Oodle runtime.
- 2026-02-24: Save decode path now handles `PlM` (Oodle via `oozextract`) and `PlZ` (single/double zlib) in Rust.
- 2026-02-24: Rust server dependency set now includes `uesave` for native GVAS parse/extract during import normalization.
- 2026-02-24: Added Rust `save_probe` utility (`src/server/src/bin/save_probe.rs`) to inspect decoded Level/Players GVAS structure and confirm extraction paths against real save data.
- 2026-02-24: `save_probe` is wired to reuse server save modules (`detect`, `parse`, `zip`) for format-consistent inspection output.
- 2026-02-24: `save_probe` module path wiring uses `src/server/src/save/*` directly to keep probe behavior identical to importer decode logic.
- 2026-02-24: `save_probe` now parses GVAS via `uesave::SaveReader.error_to_raw(true)` to tolerate unknown property schemas while exploring real saves.
- 2026-02-24: `save_probe` now prints GVAS property kind and serialized snippets when key paths are not in expected struct form, to speed schema discovery.
- 2026-02-24: `save_probe` now supplies initial Palworld struct-type hints to `uesave` (`worldSaveData.*` key/value paths) to parse beyond raw `worldSaveData` bytes.
- 2026-02-24: `save_probe` parse mode now combines Palworld hints with `error_to_raw(true)` to keep partial structures available even when some nested schema hints remain missing.
- 2026-02-24: Added Rust `gvas` dependency to evaluate hint-driven UE save parsing for `worldSaveData` and custom raw-byte substructures.
- 2026-02-24: `save_probe` now uses `gvas::GvasFile::read_with_hints` plus Palworld hint map mirrored from `palworld_save_tools/paltypes.py` for direct `worldSaveData` structure inspection.
- 2026-02-24: `save_probe` carries local decode helpers (`PlM`/`PlZ`) so probe runs independently of crate-internal module paths while preserving importer-equivalent decode behavior.
- 2026-02-24: `save_probe` now auto-expands missing GVAS struct hints at parse time by normalizing verbose property-stack paths and mapping them back to Palworld reference hints.
- 2026-02-24: `save_probe` now decodes `CharacterSaveParameterMap.Value.RawData` with gvas property parsing to validate extraction of planner-core character fields (is_player, ids, species, nickname, level).
- 2026-02-24: `save_probe` property-stream decoder now materializes `HashableIndexMap<String, Vec<Property>>` rows for raw-character object traversal.
- 2026-02-24: `save_probe` now inspects `BaseCampSaveData` and `CharacterContainerSaveData` first-entry key/value structures to lock base-assignment extraction paths.
- 2026-02-24: `save_probe` now prints `CharacterContainerSaveData.Value.Slots` property variant so slot-to-pal assignment traversal can be implemented against the correct shape.
- 2026-02-24: `save_probe` now inspects first `CharacterContainerSaveData` slot struct keys to confirm where `SlotIndex` and slot `RawData` bytes are exposed for base-roster normalization.
- 2026-02-24: `save_probe` slot inspection now covers both `ArrayProperty::Structs` and `ArrayProperty::Properties` shapes for `CharacterContainerSaveData.Value.Slots`.
- 2026-02-24: `save_probe` now prints `CharacterContainerSaveData.Value.Slots` concrete `ArrayProperty` variant to finalize slot decoding strategy.
- 2026-02-24: `save_probe` now prints first slot `StructPropertyValue` variant for `CharacterContainerSaveData.Value.Slots` to verify slot struct body parsing viability.
- 2026-02-24: `save_probe` now scans all `CharacterContainerSaveData` entries to locate a non-empty slots array and emit slot struct keys for assignment decoding.
- 2026-02-24: `src/server/src/save/normalize.rs` now includes native Rust extraction for planner-scope entities from `Level.sav`: auto-hinted GVAS parse, character map decode (`RawData`), pal/player projection, and base-slot assignment projection via `BaseCampSaveData` + `CharacterContainerSaveData`.
- 2026-02-24: `/api/v1/save/import-zip` now upserts normalized players/pals/base assignments from decoded `Level.sav` extraction and writes corresponding normalized-to-raw link rows in the same import transaction.
- 2026-02-24: Added direct `byteorder` dependency for deterministic binary cursor reads in normalization decoders (`base_camp`, `worker_director`, slot `RawData`).
- 2026-02-24: Importer normalized summary is now computed from persisted normalized rows after upserts (players/pals/base assignments) and returned in `POST /save/import-zip`.
- 2026-02-24: Added import inspection APIs for versions, files, variant metadata, and normalized-row payload reads.
- 2026-02-24: Normalized schema migrations (`0004`-`0006`) now define planner players/pals/base assignments, raw-link tables, and export lineage tables in PostgreSQL.
- 2026-02-24: Import pipeline now seeds `planner_players` from `Players/<uid>.sav` filenames to provide immediate normalized DB visibility before full GVAS entity parsing.
- 2026-02-24: Added migration `0007_import_progress.sql` so imports can expose phase-based status for SSE progress streaming.
- 2026-02-24: Added artifact storage `read_bytes` helper for background import processing stages (variant decode and normalization).
- 2026-02-24: Import handler refactor started for asynchronous post-upload processing and explicit phase progress state updates.
- 2026-02-24: `save_import_versions` insert path now initializes `progress_phase`, `progress_pct`, and `progress_message` at import creation time.
- 2026-02-24: Import request path no longer performs per-file SAV decode inspection inline; variant metadata generation is being moved to async post-processing.
- 2026-02-24: Added background post-import worker scaffold in Rust to process variant metadata and planner normalization after upload transaction commit.
- 2026-02-24: Import version API response model now includes explicit phased progress fields to support frontend progress bars and SSE updates.
- 2026-02-24: Added `GET /api/v1/save/import-versions/{id}/events` SSE stream for phase/status updates (`progress`, `done`, `progress_error` events).
- 2026-02-24: `POST /save/import-zip` now commits quickly and schedules decode/normalization in a background Tokio task.
- 2026-02-24: Frontend shared API types now include import progress phase/message/pct and SSE event payload typing.
- 2026-02-24: Frontend API client now uses `XMLHttpRequest` upload progress and `EventSource` SSE stream consumption for phased import progress updates.
- 2026-02-24: Import viewer now renders phased progress UI (`uploading`, `processing`, `ready`, `failed`) and maps backend progress events to a single progress bar.
- 2026-02-24: SSE completion handler now refreshes both import version list and selected import detail to avoid stale UI after background normalization finishes.
- 2026-02-24: Frontend SSE parsing now handles malformed/non-JSON event payloads defensively and reports deterministic error messages.
- 2026-02-24: SSE handler now emits strongly-typed `Result<Event, Infallible>` stream items to satisfy Axum SSE type inference at compile time.
- 2026-02-24: Async import/SSE flow validated on local run (`POST /import-zip` returned in ~402 ms for `gamesave.zip`; SSE emitted `decoding_variants` progress event).
- 2026-02-24: SSE server/client error channel now uses custom event name `progress_error` to avoid collision with native EventSource transport errors.
- 2026-02-24: Upload API client now falls back to `responseText` JSON parsing when `XMLHttpRequest.response` is null to avoid browser parsing edge cases.
- 2026-02-24: Fixed Vite dev proxy mismatch by removing `/api` path rewrite; backend routes are rooted at `/api/v1/*`, so dev proxy must forward `/api/*` unchanged.
- 2026-02-24: Import background processing now offloads per-file SAV decode inspection and `Level.sav` normalization to `tokio::task::spawn_blocking` with explicit timeouts to prevent API event-loop starvation/hangs.
- 2026-02-24: Normalization now pre-parses base assignments and selectively skips `CharacterSaveParameterMap` entries with zero `PlayerUId` unless the pal instance is required by base-slot assignments, reducing irrelevant world-entity decode work.
- 2026-02-24: Frontend SSE progress handling now updates import-version list rows and selected detail live (status/phase/pct/message/counts), and top progress bar now uses backend `progress_pct` directly to prevent cross-panel mismatch.
- 2026-02-24: Normalization stage now streams granular backend progress derived from `CharacterSaveParameterMap` scan (`processed/total/selected` plus current players/pals), with throttled DB updates and percentage mapped across `75..98` before finalize.
- 2026-02-24: Added pre-scan normalization stage progress events (`decode wrapper`, `GVAS root parse`, `hint resolution passes`, `world container extraction`) so imports no longer appear frozen at 75% before character scan begins.
- 2026-02-24: GVAS hint-resolution progress now reports every pass with missing-hint path and hint count, enabling exact stall-point diagnosis during parse.
- 2026-02-24: Added in-process discovered-hint cache in normalization parser so subsequent imports in the same server session start with previously learned hint paths.
- 2026-02-24: Increased normalization watchdog timeout from 120s to 300s while reducing repeated hint-resolution overhead and collecting accurate stage diagnostics.
- 2026-02-24: Added `docs/rust_palworld_save_tools_mirroring_guide.md` as the canonical implementation guide for porting the Python reference decode/encode behavior to Rust without reducing planner fidelity.
- 2026-02-24: Added `src/server/src/save/paltypes.rs` with mirrored `PALWORLD_TYPE_HINTS` and `DISABLED_PROPERTIES` from `palworld-save-tools` as the canonical Rust hint source.
- 2026-02-24: Added `src/server/src/save/hint_registry.rs` to normalize/filter mirrored hint paths, merge in discovered hint cache, and centralize hint registry behavior.
- 2026-02-24: Added `src/server/src/save/rawdata/mod.rs` scaffold with planner codec module layout plus shared binary helpers (GUID/FString reads, bounded byte reads, passthrough hex wrapper).
- 2026-02-24: Added rawdata codec files for mirrored custom domains: `character`, `base_camp`, `worker_director`, `character_container`, `work`, and `group`; implemented decoded field extraction for base/worker/container and passthrough-preserving wrappers for remaining domains.
- 2026-02-24: Added `src/server/src/save/custom_registry.rs` with mirrored `PALWORLD_CUSTOM_PROPERTIES` key coverage and codec-status behavior (`decoded` vs `passthrough`) for unimplemented domains.
- 2026-02-24: Added `src/server/src/save/roundtrip.rs` with `KnownDecoded`, `OpaqueRaw`, and `HybridRaw` primitives and exported new save modules via `src/server/src/save/mod.rs`.
- 2026-02-24: Started normalization mirror refactor by adding `ParseMetrics`, `NormalizationResult`, and registry imports in `src/server/src/save/normalize.rs` for metrics-backed parse pipeline wiring.
- 2026-02-24: Normalization parser now starts from mirrored hint registry, emits required mirrored-hint stage messages, and caps fallback hint passes at 64 with discovered-hint cache updates.
- 2026-02-24: Character-map normalization now returns explicit parse counters (`total`, `selected`, `decoded`) to populate deterministic parse metrics.
- 2026-02-24: Base assignment extraction now routes raw byte decoding through the new custom codec registry (`BaseCamp`, `WorkerDirector`, `CharacterContainer` paths) and records base/container parse counters.
- 2026-02-24: Removed legacy local hint table/cache from `normalize.rs`; normalization now relies on centralized mirrored hint registry only.
- 2026-02-24: Hint-parse outcome now records concrete start/end hint counts from runtime map size, enabling accurate parse metrics persistence.
- 2026-02-24: Added migration `0008_import_parse_metrics.sql` introducing `save_import_versions.parse_metrics_json JSONB` for importer parse telemetry persistence.
- 2026-02-24: Import background processor now persists normalization parse metrics (`parse_metrics_json`) on successful completion and binds metrics into final `ready` status update.
- 2026-02-24: Import version list/detail/SSE APIs now expose `parse_metrics_json` for UI-level parse diagnostics and hint/pass count visibility.
- 2026-02-24: Frontend import viewer now includes `parse_metrics_json` in API types and summary rendering (decode ms, parse ms, hint pass/count span).
- 2026-02-24: Fixed rawdata helper `decode_fstring` to handle nullable FString decode result correctly (`Option<String>` -> `Result<String, String>`).
- 2026-02-24: Parse metrics now derive `disabled_property_skips` from mirrored `DISABLED_PROPERTIES` instead of hard-coded constants.
- 2026-02-24: Hint inference now uses one mirrored-hint snapshot per parse invocation, avoiding repeated registry rebuilds during fallback pass loops.
- 2026-02-24: Added explicit alias hint entries for observed `MapObjectSaveData` path variants (with/without duplicated `MapObjectSaveData` segment) to avoid fallback hint passes on live saves.
- 2026-02-24: Fallback hint progress message now includes both simplified and raw missing path forms to support exact aliasing when live saves require typed-path variants.
- 2026-02-24: Hint registry now persists fallback-discovered hint paths/types to `data/discovered_hint_paths.txt` (deduped append format `path|type`) and loads that file into the active hint map on subsequent parses.
- 2026-02-24: Added 35 typed-path hint aliases from `data/discovered_hint_paths.txt` into `src/server/src/save/paltypes.rs` to reduce fallback resolution passes on live saves.
- 2026-02-24: Reset `data/discovered_hint_paths.txt` to header-only after promoting current entries into `paltypes.rs`, so next import captures only newly unresolved hints.
- 2026-02-24: Performance decision locked: retain current broad parse behavior for now; do not skip `MapObjectSaveData`/`Dungeon MapObject`/`FoliageGrid` branches purely for speed until full tooling stack is built.
- 2026-02-24: Observed stable parser telemetry after hint expansion: `decode_wrapper_ms=339`, `parse_gvas_ms=11701`, `hint_pass_count=0`, `hint_count 74->74`.
- 2026-02-24: Repository hygiene fix applied: `src/server/storage/` and `src/server/target_perf_test/` are now ignored and removed from Git tracking (`git rm --cached`) while remaining available as local runtime/build artifacts.
