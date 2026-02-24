#[derive(Debug, Clone)]
pub struct KnownDecoded<T> {
    pub known: T,
}

#[derive(Debug, Clone)]
pub struct OpaqueRaw {
    pub original_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HybridRaw<T> {
    pub known: T,
    pub opaque_unknown: Vec<u8>,
    pub original_bytes: Vec<u8>,
}
