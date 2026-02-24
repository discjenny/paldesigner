use crate::save::custom_registry::decode_raw;
use crate::save::detect::detect_save_variant;
use crate::save::hint_registry::{cache_discovered_hint, merged_hints_with_cache};
use crate::save::paltypes::DISABLED_PROPERTIES;
use crate::save::parse::decode_to_gvas;
use gvas::cursor_ext::ReadExt;
use gvas::error::{DeserializeError, Error as GvasError};
use gvas::game_version::GameVersion;
use gvas::properties::Property;
use gvas::properties::PropertyOptions;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::int_property::BytePropertyValue;
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedPlannerSummary {
    pub player_count: usize,
    pub pal_count: usize,
    pub base_assignment_count: usize,
}

pub fn empty_summary() -> NormalizedPlannerSummary {
    NormalizedPlannerSummary {
        player_count: 0,
        pal_count: 0,
        base_assignment_count: 0,
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedPlayer {
    pub player_uid: String,
    pub player_instance_id: Option<String>,
    pub player_name: Option<String>,
    pub guild_id: Option<String>,
    pub level: Option<i32>,
    pub raw_file_ref: Uuid,
    pub raw_entity_path: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedPal {
    pub pal_instance_id: String,
    pub owner_player_uid: Option<String>,
    pub species_id: Option<String>,
    pub nickname: Option<String>,
    pub gender: Option<String>,
    pub level: Option<i32>,
    pub exp: Option<i64>,
    pub passive_skill_ids: Vec<String>,
    pub mastered_waza_ids: Vec<String>,
    pub equip_waza_ids: Vec<String>,
    pub raw_file_ref: Uuid,
    pub raw_entity_path: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedAssignment {
    pub base_id: String,
    pub pal_instance_id: String,
    pub assignment_kind: Option<String>,
    pub assignment_target: Option<String>,
    pub priority: Option<i32>,
    pub raw_file_ref: Uuid,
    pub raw_entity_path: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedPlannerData {
    pub players: Vec<ExtractedPlayer>,
    pub pals: Vec<ExtractedPal>,
    pub assignments: Vec<ExtractedAssignment>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ParseMetrics {
    pub decode_wrapper_ms: u64,
    pub parse_gvas_ms: u64,
    pub hint_pass_count: u32,
    pub hint_count_start: usize,
    pub hint_count_end: usize,
    pub character_map_total: usize,
    pub character_map_selected: usize,
    pub character_map_decoded: usize,
    pub basecamp_count: usize,
    pub container_count: usize,
    pub disabled_property_skips: usize,
}

#[derive(Debug, Clone)]
pub struct NormalizationResult {
    pub data: ExtractedPlannerData,
    pub metrics: ParseMetrics,
}

#[derive(Debug, Clone)]
pub struct NormalizationProgress {
    pub progress_pct_hint: Option<i32>,
    pub message: String,
    pub total_character_entries: usize,
    pub processed_character_entries: usize,
    pub selected_character_entries: usize,
    pub player_count: usize,
    pub pal_count: usize,
}

pub fn extract_from_level_sav(
    level_sav_bytes: &[u8],
    raw_file_ref: Uuid,
) -> Result<ExtractedPlannerData, String> {
    extract_from_level_sav_with_progress(level_sav_bytes, raw_file_ref, |_| {})
        .map(|result| result.data)
}

pub fn extract_from_level_sav_with_progress<F>(
    level_sav_bytes: &[u8],
    raw_file_ref: Uuid,
    mut on_progress: F,
) -> Result<NormalizationResult, String>
where
    F: FnMut(NormalizationProgress),
{
    let mut metrics = ParseMetrics {
        disabled_property_skips: DISABLED_PROPERTIES.len(),
        ..ParseMetrics::default()
    };

    on_progress(make_stage_progress(
        76,
        "Decoding Level.sav wrapper payload",
    ));
    let decode_start = Instant::now();
    let variant = detect_save_variant(level_sav_bytes);
    let level_gvas = decode_to_gvas(level_sav_bytes, &variant)
        .map_err(|error| format!("decode failed: {}", error))?;
    metrics.decode_wrapper_ms = decode_start.elapsed().as_millis() as u64;

    on_progress(make_stage_progress(77, "Parsing Level.sav GVAS root"));
    on_progress(make_stage_progress(
        78,
        "Applying mirrored PALWORLD_TYPE_HINTS",
    ));
    let parse_start = Instant::now();
    let hint_outcome = parse_with_auto_hints(&level_gvas, &mut on_progress)
        .map_err(|error| format!("gvas parse failed: {}", error))?;
    metrics.parse_gvas_ms = parse_start.elapsed().as_millis() as u64;
    metrics.hint_pass_count = hint_outcome.hint_pass_count;
    metrics.hint_count_start = hint_outcome.hint_count_start;
    metrics.hint_count_end = hint_outcome.hint_count_end;

    let gvas = hint_outcome.gvas;
    let expanded_hints = hint_outcome.hints;

    on_progress(make_stage_progress(89, "Decoding planner raw domains"));
    let world_props = get_world_save_data_props(&gvas.properties)
        .ok_or_else(|| "missing worldSaveData CustomStruct".to_string())?;

    let (assignments, assignment_stats) = parse_base_assignments(world_props, raw_file_ref)?;
    metrics.basecamp_count = assignment_stats.basecamp_count;
    metrics.container_count = assignment_stats.container_count;
    let required_assignment_instance_ids: HashSet<String> = assignments
        .iter()
        .map(|assignment| assignment.pal_instance_id.clone())
        .collect();

    let mut players = Vec::<ExtractedPlayer>::new();
    let mut pals = Vec::<ExtractedPal>::new();
    let character_stats = parse_character_map(
        world_props,
        &expanded_hints,
        gvas.header.get_custom_versions(),
        &required_assignment_instance_ids,
        raw_file_ref,
        &mut players,
        &mut pals,
        &mut on_progress,
    )
    .map_err(|error| format!("character map parse failed: {}", error))?;
    metrics.character_map_total = character_stats.total_entries;
    metrics.character_map_selected = character_stats.selected_entries;
    metrics.character_map_decoded = character_stats.decoded_entries;

    Ok(NormalizationResult {
        data: ExtractedPlannerData {
            players,
            pals,
            assignments,
        },
        metrics,
    })
}

#[derive(Debug, Clone)]
struct HintParseOutcome {
    gvas: gvas::GvasFile,
    hints: HashMap<String, String>,
    hint_pass_count: u32,
    hint_count_start: usize,
    hint_count_end: usize,
}

fn parse_with_auto_hints(
    gvas_bytes: &[u8],
    on_progress: &mut impl FnMut(NormalizationProgress),
) -> Result<HintParseOutcome, GvasError> {
    const MAX_FALLBACK_PASSES: u32 = 64;
    let mirrored_hints = merged_hints_with_cache();
    let mut hints = mirrored_hints.clone();
    let hint_count_start = hints.len();
    let mut hint_pass_count = 0u32;

    loop {
        let mut reader = Cursor::new(gvas_bytes);
        match gvas::GvasFile::read_with_hints(&mut reader, GameVersion::Default, &hints) {
            Ok(parsed) => {
                let hint_count_end = hints.len();
                return Ok(HintParseOutcome {
                    gvas: parsed,
                    hints,
                    hint_pass_count,
                    hint_count_start,
                    hint_count_end,
                });
            }
            Err(GvasError::Deserialize(DeserializeError::MissingHint(kind, path, _))) => {
                if hint_pass_count >= MAX_FALLBACK_PASSES {
                    return Err(GvasError::Deserialize(DeserializeError::InvalidProperty(
                        format!("exceeded fallback hint resolution limit ({MAX_FALLBACK_PASSES})")
                            .into(),
                        0,
                    )));
                }

                let path_string = path.to_string();
                if hints.contains_key(&path_string) {
                    return Err(GvasError::Deserialize(DeserializeError::MissingHint(
                        kind, path, 0,
                    )));
                }

                let simplified = simplify_hint_path(&path_string);
                let inferred = mirrored_hints
                    .get(&simplified)
                    .cloned()
                    .unwrap_or_else(|| kind.to_string());
                hints.insert(path_string.clone(), inferred.clone());
                cache_discovered_hint(path_string, inferred);

                hint_pass_count += 1;
                let pct = 78 + ((hint_pass_count as i32 * 11) / MAX_FALLBACK_PASSES as i32);
                on_progress(make_stage_progress(
                    pct.clamp(78, 89),
                    format!(
                        "Fallback hint resolution pass {}/{}, hints {}, missing {} (raw: {})",
                        hint_pass_count,
                        MAX_FALLBACK_PASSES,
                        hints.len(),
                        simplified,
                        path
                    ),
                ));
            }
            Err(error) => return Err(error),
        }
    }
}

fn simplify_hint_path(path: &str) -> String {
    path.split('.')
        .filter(|segment| {
            !matches!(
                *segment,
                "StructProperty" | "MapProperty" | "ArrayProperty" | "SetProperty"
            )
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn get_world_save_data_props<'a>(
    top_level: &'a gvas::types::map::HashableIndexMap<String, Property>,
) -> Option<&'a gvas::types::map::HashableIndexMap<String, Vec<Property>>> {
    let world_prop = top_level.get("worldSaveData")?;
    as_custom_struct(world_prop)
}

#[derive(Debug, Clone, Default)]
struct CharacterParseStats {
    total_entries: usize,
    selected_entries: usize,
    decoded_entries: usize,
}

fn parse_character_map<F>(
    world_props: &gvas::types::map::HashableIndexMap<String, Vec<Property>>,
    hints: &HashMap<String, String>,
    custom_versions: &gvas::types::map::HashableIndexMap<gvas::types::Guid, u32>,
    required_assignment_instance_ids: &HashSet<String>,
    raw_file_ref: Uuid,
    players: &mut Vec<ExtractedPlayer>,
    pals: &mut Vec<ExtractedPal>,
    on_progress: &mut F,
) -> Result<CharacterParseStats, String>
where
    F: FnMut(NormalizationProgress),
{
    let character_map_props = world_props
        .get("CharacterSaveParameterMap")
        .and_then(|values| values.first())
        .ok_or_else(|| "missing CharacterSaveParameterMap".to_string())?;

    let map_entries = match character_map_props {
        Property::MapProperty(MapProperty::Properties { value, .. }) => value,
        _ => return Err("CharacterSaveParameterMap is not a map".to_string()),
    };

    let total_entries = map_entries.len();
    let mut processed_entries = 0usize;
    let mut selected_entries = 0usize;
    let mut emit_progress = |processed: usize,
                             selected: usize,
                             player_count: usize,
                             pal_count: usize| {
        let pct = if total_entries == 0 {
            98
        } else {
            90 + ((processed as i32 * 8) / total_entries as i32)
        }
        .clamp(90, 98);
        on_progress(NormalizationProgress {
            progress_pct_hint: Some(pct),
            message: format!(
                "Extracting planner entities from Level.sav ({}/{}, selected {}, players {}, pals {})",
                processed, total_entries, selected, player_count, pal_count
            ),
            total_character_entries: total_entries,
            processed_character_entries: processed,
            selected_character_entries: selected,
            player_count,
            pal_count,
        });
    };
    emit_progress(
        processed_entries,
        selected_entries,
        players.len(),
        pals.len(),
    );

    for (entry_key, entry_value) in map_entries {
        processed_entries += 1;
        let key_props = as_custom_struct(entry_key);
        let value_props = as_custom_struct(entry_value);
        let (Some(key_props), Some(value_props)) = (key_props, value_props) else {
            if should_emit_character_progress(processed_entries, total_entries) {
                emit_progress(
                    processed_entries,
                    selected_entries,
                    players.len(),
                    pals.len(),
                );
            }
            continue;
        };

        let instance_id = get_guid_uid(get_first_prop(key_props, "InstanceId"));
        let player_uid = get_guid_uid(get_first_prop(key_props, "PlayerUId"));
        let raw_data = get_array_bytes(get_first_prop(value_props, "RawData"));
        let (Some(instance_id), Some(player_uid), Some(raw_data)) =
            (instance_id, player_uid, raw_data)
        else {
            if should_emit_character_progress(processed_entries, total_entries) {
                emit_progress(
                    processed_entries,
                    selected_entries,
                    players.len(),
                    pals.len(),
                );
            }
            continue;
        };
        let is_zero_player_uid = player_uid == "00000000000000000000000000000000";
        if is_zero_player_uid && !required_assignment_instance_ids.contains(&instance_id) {
            if should_emit_character_progress(processed_entries, total_entries) {
                emit_progress(
                    processed_entries,
                    selected_entries,
                    players.len(),
                    pals.len(),
                );
            }
            continue;
        }
        selected_entries += 1;

        let mut cursor = Cursor::new(raw_data.as_slice());
        let object_props = parse_property_stream(
            &mut cursor,
            hints,
            custom_versions,
            "worldSaveData.CharacterSaveParameterMap.Value.RawData",
        )
        .map_err(|error| error.to_string())?;

        let mut skipped = [0u8; 4];
        if cursor.read_exact(&mut skipped).is_err() {
            if should_emit_character_progress(processed_entries, total_entries) {
                emit_progress(
                    processed_entries,
                    selected_entries,
                    players.len(),
                    pals.len(),
                );
            }
            continue;
        }
        let group_id = cursor
            .read_guid()
            .ok()
            .map(|guid| normalize_guid(&guid.to_string()));

        let save_parameter = get_first_prop(&object_props, "SaveParameter");
        let Some(save_parameter_props) = save_parameter.and_then(as_custom_struct) else {
            if should_emit_character_progress(processed_entries, total_entries) {
                emit_progress(
                    processed_entries,
                    selected_entries,
                    players.len(),
                    pals.len(),
                );
            }
            continue;
        };

        let is_player = get_bool(get_first_prop(save_parameter_props, "IsPlayer")).unwrap_or(false);
        let nickname = get_string(get_first_prop(save_parameter_props, "NickName"));
        let level = get_i32(get_first_prop(save_parameter_props, "Level"));
        let raw_entity_path = format!("worldSaveData.CharacterSaveParameterMap[{}]", instance_id);

        if is_player {
            players.push(ExtractedPlayer {
                player_uid: player_uid.clone(),
                player_instance_id: Some(instance_id.clone()),
                player_name: nickname,
                guild_id: group_id,
                level,
                raw_file_ref,
                raw_entity_path,
            });
            if should_emit_character_progress(processed_entries, total_entries) {
                emit_progress(
                    processed_entries,
                    selected_entries,
                    players.len(),
                    pals.len(),
                );
            }
            continue;
        }

        let owner_uid = get_guid_uid(get_first_prop(save_parameter_props, "OwnerPlayerUId"));
        let species_id = get_string(get_first_prop(save_parameter_props, "CharacterID"));
        let gender = get_string(get_first_prop(save_parameter_props, "Gender"));
        let exp = get_i64(get_first_prop(save_parameter_props, "Exp"));
        let passive_skill_ids =
            get_string_array(get_first_prop(save_parameter_props, "PassiveSkillList"));
        let mastered_waza_ids =
            get_string_array(get_first_prop(save_parameter_props, "MasteredWaza"));
        let equip_waza_ids = get_string_array(get_first_prop(save_parameter_props, "EquipWaza"));

        pals.push(ExtractedPal {
            pal_instance_id: instance_id,
            owner_player_uid: owner_uid,
            species_id,
            nickname,
            gender,
            level,
            exp,
            passive_skill_ids,
            mastered_waza_ids,
            equip_waza_ids,
            raw_file_ref,
            raw_entity_path,
        });

        if should_emit_character_progress(processed_entries, total_entries) {
            emit_progress(
                processed_entries,
                selected_entries,
                players.len(),
                pals.len(),
            );
        }
    }

    if !should_emit_character_progress(processed_entries, total_entries) {
        emit_progress(
            processed_entries,
            selected_entries,
            players.len(),
            pals.len(),
        );
    }

    Ok(CharacterParseStats {
        total_entries,
        selected_entries,
        decoded_entries: players.len() + pals.len(),
    })
}

fn make_stage_progress(
    progress_pct_hint: i32,
    message: impl Into<String>,
) -> NormalizationProgress {
    NormalizationProgress {
        progress_pct_hint: Some(progress_pct_hint.clamp(75, 98)),
        message: message.into(),
        total_character_entries: 0,
        processed_character_entries: 0,
        selected_character_entries: 0,
        player_count: 0,
        pal_count: 0,
    }
}

fn should_emit_character_progress(processed: usize, total: usize) -> bool {
    processed == total || processed.is_multiple_of(64)
}

#[derive(Debug, Clone, Default)]
struct AssignmentParseStats {
    basecamp_count: usize,
    container_count: usize,
}

fn parse_base_assignments(
    world_props: &gvas::types::map::HashableIndexMap<String, Vec<Property>>,
    raw_file_ref: Uuid,
) -> Result<(Vec<ExtractedAssignment>, AssignmentParseStats), String> {
    let mut base_to_container = HashMap::<String, String>::new();
    let mut container_slots = HashMap::<String, Vec<(i32, String)>>::new();
    let mut stats = AssignmentParseStats::default();

    if let Some(base_map_props) = world_props.get("BaseCampSaveData")
        && let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
            base_map_props.first()
    {
        for (_base_key, base_value) in value {
            let Some(base_struct) = as_custom_struct(base_value) else {
                continue;
            };
            stats.basecamp_count += 1;

            let Some(base_raw) = get_array_bytes(get_first_prop(base_struct, "RawData")) else {
                continue;
            };
            let Ok((_, base_data)) =
                decode_raw(".worldSaveData.BaseCampSaveData.Value.RawData", &base_raw)
            else {
                continue;
            };
            let Some(base_id) = base_data
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
            else {
                continue;
            };

            let container_id = get_first_prop(base_struct, "WorkerDirector")
                .and_then(as_custom_struct)
                .and_then(|worker| get_array_bytes(get_first_prop(worker, "RawData")))
                .and_then(|worker_raw| {
                    decode_raw(
                        ".worldSaveData.BaseCampSaveData.Value.WorkerDirector.RawData",
                        &worker_raw,
                    )
                    .ok()
                    .and_then(|(_, worker_data)| {
                        worker_data
                            .get("container_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                    })
                });

            if let Some(container_id) = container_id {
                base_to_container.insert(base_id, container_id);
            }
        }
    }

    if let Some(container_map_props) = world_props.get("CharacterContainerSaveData")
        && let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
            container_map_props.first()
    {
        for (container_key, container_value) in value {
            let Some(container_id) = as_custom_struct(container_key)
                .and_then(|key_props| get_guid_uid(get_first_prop(key_props, "ID")))
            else {
                continue;
            };
            stats.container_count += 1;

            let Some(container_struct) = as_custom_struct(container_value) else {
                continue;
            };
            let Some(slots_prop) = get_first_prop(container_struct, "Slots") else {
                continue;
            };
            let Property::ArrayProperty(ArrayProperty::Structs { structs, .. }) = slots_prop else {
                continue;
            };

            for slot in structs {
                let StructPropertyValue::CustomStruct(slot_props) = slot else {
                    continue;
                };
                let slot_index =
                    get_i32(get_first_prop(slot_props, "SlotIndex")).unwrap_or_default();
                let Some(slot_raw) = get_array_bytes(get_first_prop(slot_props, "RawData")) else {
                    continue;
                };
                let Ok((_, slot_data)) = decode_raw(
                    ".worldSaveData.CharacterContainerSaveData.Value.Slots.Slots.RawData",
                    &slot_raw,
                ) else {
                    continue;
                };
                let Some(instance_id) = slot_data
                    .get("instance_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                else {
                    continue;
                };
                if instance_id == "00000000000000000000000000000000" {
                    continue;
                }
                container_slots
                    .entry(container_id.clone())
                    .or_default()
                    .push((slot_index, instance_id));
            }
        }
    }

    let mut assignments = Vec::<ExtractedAssignment>::new();
    for (base_id, container_id) in base_to_container {
        if let Some(slots) = container_slots.get(&container_id) {
            for (slot_index, pal_instance_id) in slots {
                assignments.push(ExtractedAssignment {
                    base_id: base_id.clone(),
                    pal_instance_id: pal_instance_id.clone(),
                    assignment_kind: Some("base_slot".to_string()),
                    assignment_target: Some(slot_index.to_string()),
                    priority: Some(*slot_index),
                    raw_file_ref,
                    raw_entity_path: format!(
                        "worldSaveData.CharacterContainerSaveData[{}].Slots[{}]",
                        container_id, slot_index
                    ),
                });
            }
        }
    }

    Ok((assignments, stats))
}

fn parse_property_stream(
    cursor: &mut Cursor<&[u8]>,
    hints: &HashMap<String, String>,
    custom_versions: &gvas::types::map::HashableIndexMap<gvas::types::Guid, u32>,
    base_path: &str,
) -> Result<gvas::types::map::HashableIndexMap<String, Vec<Property>>, GvasError> {
    let mut properties: gvas::types::map::HashableIndexMap<String, Vec<Property>> =
        gvas::types::map::HashableIndexMap::new();
    let mut stack = if base_path.is_empty() {
        Vec::<String>::new()
    } else {
        base_path
            .split('.')
            .map(|value| value.to_string())
            .collect()
    };

    loop {
        let name = cursor.read_string()?;
        if name == "None" {
            break;
        }
        let prop_type = cursor.read_string()?;
        stack.push(name.clone());
        let mut options = PropertyOptions {
            hints,
            properties_stack: &mut stack,
            custom_versions,
        };
        let property = Property::new(cursor, &prop_type, true, &mut options, None)?;
        stack.pop();
        properties.entry(name).or_default().push(property);
    }

    Ok(properties)
}

fn get_first_prop<'a>(
    properties: &'a gvas::types::map::HashableIndexMap<String, Vec<Property>>,
    key: &str,
) -> Option<&'a Property> {
    properties.get(key).and_then(|values| values.first())
}

fn as_custom_struct(
    property: &Property,
) -> Option<&gvas::types::map::HashableIndexMap<String, Vec<Property>>> {
    match property {
        Property::StructProperty(value) => match &value.value {
            StructPropertyValue::CustomStruct(properties) => Some(properties),
            _ => None,
        },
        Property::StructPropertyValue(StructPropertyValue::CustomStruct(properties)) => {
            Some(properties)
        }
        _ => None,
    }
}

fn get_array_bytes(property: Option<&Property>) -> Option<Vec<u8>> {
    match property {
        Some(Property::ArrayProperty(ArrayProperty::Bytes { bytes })) => Some(bytes.clone()),
        _ => None,
    }
}

fn get_guid_uid(property: Option<&Property>) -> Option<String> {
    let property = property?;
    match property {
        Property::StructProperty(value) => match &value.value {
            StructPropertyValue::Guid(guid) => Some(normalize_guid(&guid.to_string())),
            _ => None,
        },
        Property::StructPropertyValue(StructPropertyValue::Guid(guid)) => {
            Some(normalize_guid(&guid.to_string()))
        }
        _ => None,
    }
}

fn get_bool(property: Option<&Property>) -> Option<bool> {
    match property {
        Some(Property::BoolProperty(value)) => Some(value.value),
        _ => None,
    }
}

fn get_i32(property: Option<&Property>) -> Option<i32> {
    match property {
        Some(Property::IntProperty(value)) => Some(value.value),
        Some(Property::ByteProperty(value)) => match value.value {
            BytePropertyValue::Byte(level) => Some(i32::from(level)),
            _ => None,
        },
        _ => None,
    }
}

fn get_i64(property: Option<&Property>) -> Option<i64> {
    match property {
        Some(Property::Int64Property(value)) => Some(value.value),
        _ => None,
    }
}

fn get_string(property: Option<&Property>) -> Option<String> {
    match property {
        Some(Property::StrProperty(value)) => value.value.clone(),
        Some(Property::NameProperty(value)) => value.value.clone(),
        Some(Property::EnumProperty(value)) => Some(value.value.clone()),
        _ => None,
    }
}

fn get_string_array(property: Option<&Property>) -> Vec<String> {
    let Some(Property::ArrayProperty(array)) = property else {
        return Vec::new();
    };

    match array {
        ArrayProperty::Names { names } => names.iter().flatten().cloned().collect(),
        ArrayProperty::Strings { strings } => strings.iter().flatten().cloned().collect(),
        ArrayProperty::Enums { enums } => enums.to_vec(),
        ArrayProperty::Properties { properties, .. } => properties
            .iter()
            .filter_map(|entry| match entry {
                Property::NameProperty(value) => value.value.clone(),
                Property::StrProperty(value) => value.value.clone(),
                Property::EnumProperty(value) => Some(value.value.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn normalize_guid(value: &str) -> String {
    if value == "0" {
        "00000000000000000000000000000000".to_string()
    } else {
        value.replace('-', "").to_uppercase()
    }
}

impl ExtractedPal {
    pub fn passive_skill_ids_json(&self) -> Value {
        Value::Array(
            self.passive_skill_ids
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        )
    }

    pub fn mastered_waza_ids_json(&self) -> Value {
        Value::Array(
            self.mastered_waza_ids
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        )
    }

    pub fn equip_waza_ids_json(&self) -> Value {
        Value::Array(
            self.equip_waza_ids
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        )
    }
}
