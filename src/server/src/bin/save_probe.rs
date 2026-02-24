#[path = "../save/detect.rs"]
mod detect;
#[path = "../save/zip.rs"]
mod zip;

use anyhow::{Context, Result, bail};
use gvas::cursor_ext::ReadExt;
use gvas::error::{DeserializeError, Error as GvasError};
use gvas::game_version::GameVersion;
use gvas::properties::Property;
use gvas::properties::PropertyOptions;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use gvas::types::Guid;
use gvas::types::map::HashableIndexMap;
use oozextract::Extractor;
use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::io::Read;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let zip_path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("../../gamesave.zip");
    let zip_bytes = std::fs::read(zip_path)
        .with_context(|| format!("failed to read zip file at {}", zip_path))?;

    let entries = zip::parse_zip_entries(&zip_bytes)?;
    let root = zip::detect_world_root(&entries)?;
    println!("world root: {}", root);

    let mut rooted = BTreeMap::<String, Vec<u8>>::new();
    for entry in entries {
        if let Some(relative_path) = zip::strip_root_prefix(&root, &entry.path) {
            rooted.insert(relative_path, entry.bytes);
        }
    }

    for (path, bytes) in &rooted {
        if !path.ends_with(".sav") {
            continue;
        }
        let variant = detect::detect_save_variant(bytes);
        let decoded = decode_to_gvas(bytes, &variant);
        match decoded {
            Ok(gvas) => {
                println!(
                    "{} => magic={:?} save_type={:?} compression={} gvas_bytes={}",
                    path,
                    variant.magic,
                    variant.save_type,
                    variant.compression,
                    gvas.len()
                );
            }
            Err(error) => {
                println!(
                    "{} => magic={:?} save_type={:?} compression={} decode_error={}",
                    path, variant.magic, variant.save_type, variant.compression, error
                );
            }
        }
    }

    let level = rooted
        .get("Level.sav")
        .ok_or_else(|| anyhow::anyhow!("Level.sav not found"))?;
    let level_variant = detect::detect_save_variant(level);
    let level_gvas = decode_to_gvas(level, &level_variant)
        .map_err(|e| anyhow::anyhow!("failed to decode Level.sav: {}", e))?;

    inspect_level(&level_gvas)?;

    for (path, bytes) in rooted
        .iter()
        .filter(|(path, _)| path.starts_with("Players/") && path.ends_with(".sav"))
    {
        let variant = detect::detect_save_variant(bytes);
        let gvas = decode_to_gvas(bytes, &variant)
            .map_err(|e| anyhow::anyhow!("failed to decode {}: {}", path, e))?;
        inspect_player(path, &gvas)?;
    }

    Ok(())
}

fn inspect_level(level_gvas: &[u8]) -> Result<()> {
    let simple_hints = palworld_hints();
    let (gvas, expanded_hints) = parse_with_auto_hints(level_gvas, &simple_hints)
        .context("failed to parse Level.sav GVAS with Palworld hints")?;
    println!("Level.sav hint count used: {}", expanded_hints.len());

    println!("Level.sav top-level keys:");
    for key in gvas.properties.keys() {
        println!("  - {}", key);
    }

    let Some(world_save_data) = gvas.properties.get("worldSaveData") else {
        bail!("worldSaveData missing in Level.sav");
    };

    let Property::StructProperty(world_struct) = world_save_data else {
        bail!("worldSaveData is not StructProperty");
    };
    let StructPropertyValue::CustomStruct(world_props) = &world_struct.value else {
        bail!("worldSaveData is not CustomStruct");
    };

    println!("worldSaveData keys:");
    for key in world_props.keys() {
        println!("  - {}", key);
    }

    print_property_summary(world_props, "CharacterSaveParameterMap");
    print_property_summary(world_props, "GroupSaveDataMap");
    print_property_summary(world_props, "BaseCampSaveData");
    print_property_summary(world_props, "CharacterContainerSaveData");
    print_property_summary(world_props, "WorkSaveData");
    inspect_character_map(
        world_props,
        &expanded_hints,
        gvas.header.get_custom_versions(),
    )?;
    inspect_base_and_container_maps(world_props)?;

    Ok(())
}

fn inspect_player(path: &str, player_gvas: &[u8]) -> Result<()> {
    let simple_hints = palworld_hints();
    let (gvas, _) = parse_with_auto_hints(player_gvas, &simple_hints)
        .with_context(|| format!("failed to parse player save {}", path))?;
    println!("{} top-level keys:", path);
    for key in gvas.properties.keys() {
        println!("  - {}", key);
    }
    Ok(())
}

fn print_property_summary(
    properties: &gvas::types::map::HashableIndexMap<String, Vec<Property>>,
    key: &str,
) {
    let Some(value_list) = properties.get(key) else {
        return;
    };
    let Some(first) = value_list.first() else {
        return;
    };
    match first {
        Property::MapProperty(map) => match map {
            MapProperty::Properties { value, .. } => {
                println!("{} map entries: {}", key, value.len())
            }
            _ => println!("{} map entries: n/a (specialized map variant)", key),
        },
        Property::ArrayProperty(array) => match array {
            ArrayProperty::Structs { structs, .. } => {
                println!("{} array structs: {}", key, structs.len())
            }
            ArrayProperty::Properties { properties, .. } => {
                println!("{} array properties: {}", key, properties.len())
            }
            _ => println!("{} array summary: non-struct variant", key),
        },
        other => println!("{} property kind: {}", key, property_kind(other)),
    }
}

fn inspect_character_map(
    world_props: &HashableIndexMap<String, Vec<Property>>,
    hints: &HashMap<String, String>,
    custom_versions: &HashableIndexMap<Guid, u32>,
) -> Result<()> {
    let Some(character_map_props) = world_props.get("CharacterSaveParameterMap") else {
        return Ok(());
    };
    let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
        character_map_props.first()
    else {
        return Ok(());
    };

    let mut player_count = 0usize;
    let mut pal_count = 0usize;
    let mut printed = 0usize;

    for (entry_key, entry_value) in value {
        let key_props = as_custom_struct(entry_key);
        let value_props = as_custom_struct(entry_value);
        let (Some(key_props), Some(value_props)) = (key_props, value_props) else {
            continue;
        };

        let instance_id = get_guid_string(get_first_prop(key_props, "InstanceId"));
        let player_uid = get_guid_string(get_first_prop(key_props, "PlayerUId"));
        let raw_data = get_array_bytes(get_first_prop(value_props, "RawData"));
        let Some(raw_data) = raw_data else {
            continue;
        };

        let mut cursor = Cursor::new(raw_data.as_slice());
        let object_props = parse_property_stream(
            &mut cursor,
            hints,
            custom_versions,
            "worldSaveData.CharacterSaveParameterMap.Value.RawData",
        )?;

        if cursor
            .get_ref()
            .len()
            .saturating_sub(cursor.position() as usize)
            >= 20
        {
            cursor.set_position(cursor.position() + 4);
            let _group_id = cursor.read_guid()?;
        }

        let Some(save_parameter) = get_first_prop(&object_props, "SaveParameter") else {
            continue;
        };
        let Some(save_parameter_props) = as_custom_struct(save_parameter) else {
            continue;
        };

        let is_player = get_bool(get_first_prop(save_parameter_props, "IsPlayer")).unwrap_or(false);
        if is_player {
            player_count += 1;
        } else {
            pal_count += 1;
        }

        if printed < 5 {
            let nickname = get_string(get_first_prop(save_parameter_props, "NickName"));
            let species = get_string(get_first_prop(save_parameter_props, "CharacterID"));
            let level = get_i32(get_first_prop(save_parameter_props, "Level"));
            println!(
                "character sample: is_player={} player_uid={:?} instance_id={:?} species={:?} nickname={:?} level={:?}",
                is_player, player_uid, instance_id, species, nickname, level
            );
            printed += 1;
        }
    }

    println!(
        "CharacterSaveParameterMap parsed: players={} pals={}",
        player_count, pal_count
    );
    Ok(())
}

fn inspect_base_and_container_maps(
    world_props: &HashableIndexMap<String, Vec<Property>>,
) -> Result<()> {
    if let Some(base_map_props) = world_props.get("BaseCampSaveData")
        && let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
            base_map_props.first()
        && let Some((_base_key, base_value)) = value.first()
    {
        if let Some(base_struct) = as_custom_struct(base_value) {
            println!("BaseCampSaveData first value keys:");
            for key in base_struct.keys() {
                println!("  - {}", key);
            }
        }
    }

    if let Some(container_map_props) = world_props.get("CharacterContainerSaveData")
        && let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
            container_map_props.first()
    {
        println!(
            "CharacterContainerSaveData container count: {}",
            value.len()
        );
        let mut printed_first = false;
        for (container_key, container_value) in value {
            if !printed_first {
                if let Some(key_struct) = as_custom_struct(container_key) {
                    println!("CharacterContainerSaveData first key keys:");
                    for key in key_struct.keys() {
                        println!("  - {}", key);
                    }
                }
                if let Some(container_struct) = as_custom_struct(container_value) {
                    println!("CharacterContainerSaveData first value keys:");
                    for key in container_struct.keys() {
                        println!("  - {}", key);
                    }
                }
                printed_first = true;
            }

            let Some(container_struct) = as_custom_struct(container_value) else {
                continue;
            };
            let Some(slots_prop) = get_first_prop(container_struct, "Slots") else {
                continue;
            };
            if let Property::ArrayProperty(ArrayProperty::Structs { structs, .. }) = slots_prop {
                if structs.is_empty() {
                    continue;
                }
                println!(
                    "CharacterContainerSaveData non-empty slots variant: {}, slot_count={}",
                    struct_value_kind(&structs[0]),
                    structs.len()
                );
                if let StructPropertyValue::CustomStruct(slot_props) = &structs[0] {
                    println!("CharacterContainerSaveData first non-empty slot keys:");
                    for key in slot_props.keys() {
                        println!("  - {}", key);
                    }
                }
                break;
            }
        }
    }

    Ok(())
}

fn property_kind(property: &Property) -> &'static str {
    match property {
        Property::ArrayProperty(_) => "ArrayProperty",
        Property::BoolProperty(_) => "BoolProperty",
        Property::ByteProperty(_) => "ByteProperty",
        Property::DoubleProperty(_) => "DoubleProperty",
        Property::EnumProperty(_) => "EnumProperty",
        Property::FloatProperty(_) => "FloatProperty",
        Property::Int16Property(_) => "Int16Property",
        Property::Int64Property(_) => "Int64Property",
        Property::Int8Property(_) => "Int8Property",
        Property::IntProperty(_) => "IntProperty",
        Property::MapProperty(_) => "MapProperty",
        Property::NameProperty(_) => "NameProperty",
        Property::ObjectProperty(_) => "ObjectProperty",
        Property::DelegateProperty(_) => "DelegateProperty",
        Property::MulticastInlineDelegateProperty(_) => "MulticastInlineDelegateProperty",
        Property::MulticastSparseDelegateProperty(_) => "MulticastSparseDelegateProperty",
        Property::FieldPathProperty(_) => "FieldPathProperty",
        Property::SetProperty(_) => "SetProperty",
        Property::StrProperty(_) => "StrProperty",
        Property::StructProperty(_) => "StructProperty",
        Property::StructPropertyValue(_) => "StructPropertyValue",
        Property::TextProperty(_) => "TextProperty",
        Property::UInt16Property(_) => "UInt16Property",
        Property::UInt32Property(_) => "UInt32Property",
        Property::UInt64Property(_) => "UInt64Property",
        Property::UnknownProperty(_) => "UnknownProperty",
    }
}

fn array_kind(array: &ArrayProperty) -> &'static str {
    match array {
        ArrayProperty::Bools { .. } => "Bools",
        ArrayProperty::Bytes { .. } => "Bytes",
        ArrayProperty::Enums { .. } => "Enums",
        ArrayProperty::Floats { .. } => "Floats",
        ArrayProperty::Ints { .. } => "Ints",
        ArrayProperty::Names { .. } => "Names",
        ArrayProperty::Strings { .. } => "Strings",
        ArrayProperty::Structs { .. } => "Structs",
        ArrayProperty::Properties { .. } => "Properties",
    }
}

fn struct_value_kind(value: &StructPropertyValue) -> &'static str {
    match value {
        StructPropertyValue::Vector2F(_) => "Vector2F",
        StructPropertyValue::Vector2D(_) => "Vector2D",
        StructPropertyValue::VectorF(_) => "VectorF",
        StructPropertyValue::VectorD(_) => "VectorD",
        StructPropertyValue::RotatorF(_) => "RotatorF",
        StructPropertyValue::RotatorD(_) => "RotatorD",
        StructPropertyValue::QuatF(_) => "QuatF",
        StructPropertyValue::QuatD(_) => "QuatD",
        StructPropertyValue::DateTime(_) => "DateTime",
        StructPropertyValue::Timespan(_) => "Timespan",
        StructPropertyValue::Guid(_) => "Guid",
        StructPropertyValue::LinearColor(_) => "LinearColor",
        StructPropertyValue::IntPoint(_) => "IntPoint",
        StructPropertyValue::CustomStruct(_) => "CustomStruct",
    }
}

fn parse_property_stream(
    cursor: &mut Cursor<&[u8]>,
    hints: &HashMap<String, String>,
    custom_versions: &HashableIndexMap<Guid, u32>,
    base_path: &str,
) -> Result<HashableIndexMap<String, Vec<Property>>> {
    let mut properties: HashableIndexMap<String, Vec<Property>> = HashableIndexMap::new();
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
    properties: &'a HashableIndexMap<String, Vec<Property>>,
    key: &str,
) -> Option<&'a Property> {
    properties.get(key).and_then(|values| values.first())
}

fn as_custom_struct(property: &Property) -> Option<&HashableIndexMap<String, Vec<Property>>> {
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

fn get_guid_string(property: Option<&Property>) -> Option<String> {
    let property = property?;
    match property {
        Property::StructProperty(value) => match &value.value {
            StructPropertyValue::Guid(guid) => Some(guid.to_string()),
            _ => None,
        },
        Property::StructPropertyValue(StructPropertyValue::Guid(guid)) => Some(guid.to_string()),
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
            gvas::properties::int_property::BytePropertyValue::Byte(level) => {
                Some(i32::from(level))
            }
            _ => None,
        },
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

fn palworld_hints() -> HashMap<String, String> {
    let mut hints = HashMap::new();
    for (path, ty) in [
        (
            "worldSaveData.CharacterContainerSaveData.Key",
            "StructProperty",
        ),
        (
            "worldSaveData.CharacterSaveParameterMap.Key",
            "StructProperty",
        ),
        (
            "worldSaveData.CharacterSaveParameterMap.Value",
            "StructProperty",
        ),
        ("worldSaveData.FoliageGridSaveDataMap.Key", "StructProperty"),
        (
            "worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Key",
            "StructProperty",
        ),
        (
            "worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.FoliageGridSaveDataMap.Value",
            "StructProperty",
        ),
        ("worldSaveData.ItemContainerSaveData.Key", "StructProperty"),
        (
            "worldSaveData.MapObjectSaveData.MapObjectSaveData.ConcreteModel.ModuleMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.MapObjectSaveData.MapObjectSaveData.Model.EffectMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.MapObjectSpawnerInStageSaveData.Key",
            "StructProperty",
        ),
        (
            "worldSaveData.MapObjectSpawnerInStageSaveData.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.MapObjectSpawnerInStageSaveData.Value.SpawnerDataMapByLevelObjectInstanceId.Key",
            "Guid",
        ),
        (
            "worldSaveData.MapObjectSpawnerInStageSaveData.Value.SpawnerDataMapByLevelObjectInstanceId.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.MapObjectSpawnerInStageSaveData.Value.SpawnerDataMapByLevelObjectInstanceId.Value.ItemMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.WorkSaveData.WorkSaveData.WorkAssignMap.Value",
            "StructProperty",
        ),
        ("worldSaveData.BaseCampSaveData.Key", "Guid"),
        ("worldSaveData.BaseCampSaveData.Value", "StructProperty"),
        (
            "worldSaveData.BaseCampSaveData.Value.ModuleMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.ItemContainerSaveData.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.CharacterContainerSaveData.Value",
            "StructProperty",
        ),
        ("worldSaveData.GroupSaveDataMap.Key", "Guid"),
        ("worldSaveData.GroupSaveDataMap.Value", "StructProperty"),
        (
            "worldSaveData.EnemyCampSaveData.EnemyCampStatusMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.DungeonSaveData.DungeonSaveData.MapObjectSaveData.MapObjectSaveData.Model.EffectMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.DungeonSaveData.DungeonSaveData.MapObjectSaveData.MapObjectSaveData.ConcreteModel.ModuleMap.Value",
            "StructProperty",
        ),
        ("worldSaveData.InvaderSaveData.Key", "Guid"),
        ("worldSaveData.InvaderSaveData.Value", "StructProperty"),
        (
            "worldSaveData.OilrigSaveData.OilrigMap.Value",
            "StructProperty",
        ),
        ("worldSaveData.SupplySaveData.SupplyInfos.Key", "Guid"),
        (
            "worldSaveData.SupplySaveData.SupplyInfos.Value",
            "StructProperty",
        ),
        ("worldSaveData.GuildExtraSaveDataMap.Key", "Guid"),
        (
            "worldSaveData.GuildExtraSaveDataMap.Value",
            "StructProperty",
        ),
        (
            "worldSaveData.EnemyCampSaveData.EnemyCampStatusMap.Value.TreasureBoxInfoMapBySpawnerName.Value",
            "StructProperty",
        ),
    ] {
        hints.insert(path.to_string(), ty.to_string());
    }
    hints
}

fn parse_with_auto_hints(
    gvas_bytes: &[u8],
    simple_hints: &HashMap<String, String>,
) -> Result<(gvas::GvasFile, HashMap<String, String>)> {
    let mut hints = simple_hints.clone();

    for _ in 0..512 {
        let mut reader = Cursor::new(gvas_bytes);
        match gvas::GvasFile::read_with_hints(&mut reader, GameVersion::Default, &hints) {
            Ok(parsed) => return Ok((parsed, hints)),
            Err(GvasError::Deserialize(DeserializeError::MissingHint(kind, path, _))) => {
                let path_string = path.to_string();
                if hints.contains_key(&path_string) {
                    bail!("missing-hint loop for path {}", path_string);
                }

                let simplified = simplify_hint_path(&path_string);
                let inferred = simple_hints
                    .get(&simplified)
                    .cloned()
                    .unwrap_or_else(|| kind.to_string());
                hints.insert(path_string, inferred);
            }
            Err(error) => return Err(anyhow::anyhow!(error)),
        }
    }

    bail!("exceeded missing-hint expansion limit while parsing GVAS")
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

fn decode_to_gvas(bytes: &[u8], variant: &detect::SaveVariantInfo) -> Result<Vec<u8>, String> {
    let payload = payload_slice(bytes, variant)?;

    match variant.compression.as_str() {
        "zlib" => decode_plz(payload, variant),
        "oodle" => decode_plm(payload, variant),
        _ => Err("decode_not_attempted".to_string()),
    }
}

fn decode_plz(payload: &[u8], variant: &detect::SaveVariantInfo) -> Result<Vec<u8>, String> {
    let first_pass =
        zlib_decompress(payload).map_err(|error| format!("zlib decode failed: {}", error))?;
    let decoded = if variant.save_type == Some(0x32) {
        zlib_decompress(&first_pass)
            .map_err(|error| format!("zlib second-pass decode failed: {}", error))?
    } else {
        first_pass
    };

    Ok(decoded)
}

fn decode_plm(payload: &[u8], variant: &detect::SaveVariantInfo) -> Result<Vec<u8>, String> {
    let Some(expected_size) = variant.uncompressed_size else {
        return Err("oodle decode requires uncompressed_size from save header".to_string());
    };

    let mut output = vec![0u8; expected_size as usize];
    let mut extractor = Extractor::new();
    let bytes_written = extractor
        .read_from_slice(payload, output.as_mut_slice())
        .map_err(|error| format!("oodle decode failed: {}", error))?;

    if bytes_written != output.len() {
        return Err(format!(
            "oodle decoded byte count mismatch: expected {} bytes, got {} bytes",
            output.len(),
            bytes_written
        ));
    }

    Ok(output)
}

fn payload_slice<'a>(
    bytes: &'a [u8],
    variant: &detect::SaveVariantInfo,
) -> Result<&'a [u8], String> {
    let payload_start = variant.payload_offset;
    let payload_end = payload_start.saturating_add(variant.payload_len);
    if payload_end > bytes.len() || payload_start >= payload_end {
        return Err("payload boundaries are invalid".to_string());
    }

    Ok(&bytes[payload_start..payload_end])
}

fn zlib_decompress(payload: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = flate2::read::ZlibDecoder::new(payload);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}
