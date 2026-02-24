use crate::save::rawdata;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

pub trait RawCodec: Sync + Send {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String>;
    fn encode(&self, value: &Value) -> Result<Vec<u8>, String>;
}

struct PassthroughCodec;
struct BaseCampCodec;
struct WorkerDirectorCodec;
struct CharacterContainerCodec;
struct CharacterCodec;
struct GroupCodec;
struct WorkCodec;

impl RawCodec for PassthroughCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        Ok(rawdata::passthrough_decode(bytes))
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::passthrough_encode(value)
    }
}

impl RawCodec for BaseCampCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        rawdata::base_camp::decode(bytes)
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::base_camp::encode(value)
    }
}

impl RawCodec for WorkerDirectorCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        rawdata::worker_director::decode(bytes)
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::worker_director::encode(value)
    }
}

impl RawCodec for CharacterContainerCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        rawdata::character_container::decode(bytes)
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::character_container::encode(value)
    }
}

impl RawCodec for CharacterCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        rawdata::character::decode(bytes)
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::character::encode(value)
    }
}

impl RawCodec for GroupCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        rawdata::group::decode(bytes)
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::group::encode(value)
    }
}

impl RawCodec for WorkCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Value, String> {
        rawdata::work::decode(bytes)
    }

    fn encode(&self, value: &Value) -> Result<Vec<u8>, String> {
        rawdata::work::encode(value)
    }
}

pub fn custom_registry() -> &'static HashMap<&'static str, &'static dyn RawCodec> {
    static REGISTRY: OnceLock<HashMap<&'static str, &'static dyn RawCodec>> = OnceLock::new();

    static PASSTHROUGH: PassthroughCodec = PassthroughCodec;
    static BASE_CAMP: BaseCampCodec = BaseCampCodec;
    static WORKER_DIRECTOR: WorkerDirectorCodec = WorkerDirectorCodec;
    static CHARACTER_CONTAINER: CharacterContainerCodec = CharacterContainerCodec;
    static CHARACTER: CharacterCodec = CharacterCodec;
    static GROUP: GroupCodec = GroupCodec;
    static WORK: WorkCodec = WorkCodec;

    REGISTRY.get_or_init(|| {
        let mut registry = HashMap::<&'static str, &'static dyn RawCodec>::new();
        registry.insert(".worldSaveData.GroupSaveDataMap", &GROUP);
        registry.insert(".worldSaveData.CharacterSaveParameterMap.Value.RawData", &CHARACTER);
        registry.insert(".worldSaveData.ItemContainerSaveData.Value.RawData", &PASSTHROUGH);
        registry.insert(
            ".worldSaveData.ItemContainerSaveData.Value.Slots.Slots.RawData",
            &PASSTHROUGH,
        );
        registry.insert(
            ".worldSaveData.CharacterContainerSaveData.Value.Slots.Slots.RawData",
            &CHARACTER_CONTAINER,
        );
        registry.insert(
            ".worldSaveData.DynamicItemSaveData.DynamicItemSaveData.RawData",
            &PASSTHROUGH,
        );
        registry.insert(
            ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.RawData",
            &PASSTHROUGH,
        );
        registry.insert(
            ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Value.RawData",
            &PASSTHROUGH,
        );
        registry.insert(".worldSaveData.BaseCampSaveData.Value.RawData", &BASE_CAMP);
        registry.insert(
            ".worldSaveData.BaseCampSaveData.Value.WorkerDirector.RawData",
            &WORKER_DIRECTOR,
        );
        registry.insert(
            ".worldSaveData.BaseCampSaveData.Value.WorkCollection.RawData",
            &PASSTHROUGH,
        );
        registry.insert(".worldSaveData.BaseCampSaveData.Value.ModuleMap", &PASSTHROUGH);
        registry.insert(".worldSaveData.WorkSaveData", &WORK);
        registry.insert(".worldSaveData.MapObjectSaveData", &PASSTHROUGH);
        registry.insert(
            ".worldSaveData.GuildExtraSaveDataMap.Value.GuildItemStorage.RawData",
            &PASSTHROUGH,
        );
        registry.insert(
            ".worldSaveData.GuildExtraSaveDataMap.Value.Lab.RawData",
            &PASSTHROUGH,
        );
        registry
    })
}

pub fn decode_raw(path: &str, bytes: &[u8]) -> Result<(String, Value), String> {
    match custom_registry().get(path) {
        Some(codec) => codec
            .decode(bytes)
            .map(|value| ("decoded".to_string(), value)),
        None => Ok((
            "passthrough".to_string(),
            rawdata::passthrough_decode(bytes),
        )),
    }
}
