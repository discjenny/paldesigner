# Paldesigner Agent Instructions

## Project Scope
Build a Palworld production planner web app modeled after Factorio and Satisfactory planners.

Required product capabilities:
- Create multiple bases.
- Add and remove Pals in each base.
- Fully customize each Pal state for planning (level, star/condense level, passives, work suitability levels, active assignment), equivalent to save-editor-level control.
- Import actual game save data into planner state.
- Export changes back to actual game save format.
- Select buildings per base (for example plantations, mining sites, logging sites, ranch, oil extractor).
- Calculate required work power by work type per base and per building set.
- Calculate production output rates (items per minute and items per hour) for selected targets (for example Ore, Wheat).
- Use in-game icons for Pals, work icons, and relevant items/buildings.

Conversion scope is restricted to planner-relevant save data:
- Player data required for planning.
- Pal data required for planning.
- Base setup data required for planning:
  - Which pals are assigned to each base.
  - What each assigned pal is set to work on.
- Any additional save fields strictly required to compute planner outputs.

Explicitly excluded from conversion scope:
- World structure/building placement geometry and structure instance editing.
- Non-planner world simulation domains not needed for production calculations.

## Stack (Fixed)
- Frontend: React + TypeScript + Vite (HMR in development).
- UI: Tailwind CSS + shadcn/ui + Lucide icons.
- Frontend runtime and package manager: Bun.
- Backend webserver/API: Rust.
- Data and tooling scripts: Python with `uv`.
- Save import/export runtime pipeline: native Rust only.
- Database: PostgreSQL (development and production).
- Container runtime: Docker.
- Production runtime: Rust webserver + PostgreSQL in containerized deployment.

## Environment Rules
- Development mode:
  - Run Bun + Vite HMR frontend, Rust webserver, and PostgreSQL.
  - Development startup requires PostgreSQL connectivity.
- Production mode:
  - Run in Docker.
  - Container stack must run Rust webserver and PostgreSQL.

## Data Rules
- Treat `data/raw/*` as immutable raw source snapshots.
- Store parsed and normalized data as JSON files before wiring UI/database logic.
- Store normalized planner entities with explicit foreign-key links to raw save artifacts and version records.
- Every numeric planner formula must include source and units in code comments or adjacent metadata.
- Do not invent production or workload numbers. If a number is unknown, mark it `null` and add a blocker note in `PROGRESS.md`.

## Pal Editing Contract (Base-Game-Legal Only)
The app must support editing all Pal fields that are possible in the base game and relevant to save fidelity/planning.

Required editable Pal fields:
- Identity and variant:
  - Species (`CharacterID`)
  - Nickname
  - Gender
  - Variant flags that exist in base game data (for example boss/rare/tower/raid/predator/oilrig where valid for that species)
- Progression:
  - Level
  - Experience
  - Friendship
  - Condense rank (star rank)
- Stat growth and enhancement:
  - Talent/IV: HP, Melee, Shot, Defense
  - Soul ranks: HP, Attack, Defense, CraftSpeed
- Skills:
  - Passive skill list
  - Learned attacks (`MasteredWaza`)
  - Equipped attacks (`EquipWaza`)
- Work:
  - Work suitability ranks for all supported work types
  - Added work suitability rank data (condense-derived suitability increase)
- Runtime/base status fields present in base save data:
  - HP
  - Sanity
  - Hunger / food state
  - Worker sickness and revive state/timers

Validation requirements:
- Reject values outside base-game legal bounds.
- Do not allow cheat-only values (no overflow ranks/IV/souls/passives/equips beyond base-game limits).
- Validate by species/variant constraints before save export.

## Save Format Contract (Only Actual Game Save Format)
- Import/export must use ZIP archives as the transfer format.
- ZIP import payload must contain the Palworld multi-file save set (not single-file only), including:
  - `Level.sav`
  - `Players/*.sav`
- Import must support nested root folders and auto-detect the valid world root.
- Import accepts `Players/` directory name only.
- Import rejects `Player/` directory name.
- Extra files outside supported export set are ignored.
- Save handling architecture:
  - Use patch model.
  - Keep imported save files immutable.
  - Do not require proprietary Oodle runtime libraries (`oo2core_9_win64.dll`) anywhere in this project.
  - Decode `PlM` (`0x31`) saves using open-source Oodle-compatible backends only.
  - Decode `PlZ` (`0x32`) saves using zlib.
  - Do not use Python bridge processes for save decode/re-encode in application runtime.
  - Apply validated patch operations to imported originals during export.
  - For each changed file only, execute: `SAV -> GVAS decode -> object mutation -> GVAS -> SAV recompress`.
  - Do not use JSON as the save round-trip format.
- Output format:
  - Produce ZIP archives with this exact root layout:
    - `Level.sav`
    - `LevelMeta.sav`
    - `LocalData.sav`
  - `WorldOption.sav`
  - `Players/`
  - Update only files that have actual patch changes.
  - Re-encode/recompress only changed files.
  - For changed files, output recompressed save files as `PlZ` (`0x32`, zlib) to avoid proprietary encoder dependency.
  - Copy untouched files byte-identical from the imported artifact set.
- Save artifact versioning:
  - Store every imported ZIP and extracted raw file set as an immutable version with timestamp.
  - Store every generated export ZIP and extracted raw file set as an immutable version with timestamp.
  - Track lineage from generated export version to source import version.
  - Retain all versions permanently.
  - Record SHA-256 and XXH64 checksums for ZIP and extracted artifacts.
- Explicitly out of scope:
  - Planner JSON import/export as a user-facing save format
  - Non-game save interchange formats

## Guard Rails
- No ambiguous TODO text in committed files.
- Every feature change must include:
  - Updated data contract or schema if applicable.
  - Validation path (unit test, fixture test, or deterministic calculation check).
  - `PROGRESS.md` checkbox update in the same change.
- Keep planner math deterministic and reproducible from JSON inputs.
- Keep UI assets local or version-pinned so icons do not break.

## Workflow Rules
- `PROGRESS.md` is the source of truth for execution order and status.
- After every file change, update `PROGRESS.md` immediately:
  - Mark completed tasks.
  - Add new tasks discovered during implementation.
  - Add blockers and decisions in the Notes sections.
- If scope changes, update both this file and `PROGRESS.md` in the same change.

## Reference Implementation Source
- Use this repository as a reference for save-structure and edit-surface validation:
  - `https://github.com/KrisCris/Palworld-Pal-Editor`

## Out of Scope For Now
- Multiplayer live telemetry dashboards.
- Hosting external services beyond the app container.
- Non-PostgreSQL production databases.
