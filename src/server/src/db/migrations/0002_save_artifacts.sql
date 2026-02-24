CREATE TABLE IF NOT EXISTS save_zip_artifacts (
    id UUID PRIMARY KEY,
    import_version_id UUID REFERENCES save_import_versions(id) ON DELETE RESTRICT,
    export_version_id UUID REFERENCES save_export_versions(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('import_source_zip', 'export_zip')),
    storage_key TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    byte_size BIGINT NOT NULL,
    sha256 CHAR(64) NOT NULL,
    xxh64 CHAR(16) NOT NULL,
    immutable BOOLEAN NOT NULL DEFAULT TRUE,
    retention_policy TEXT NOT NULL DEFAULT 'forever',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        (import_version_id IS NOT NULL AND export_version_id IS NULL)
        OR (import_version_id IS NULL AND export_version_id IS NOT NULL)
    )
);

CREATE TABLE IF NOT EXISTS save_files (
    id UUID PRIMARY KEY,
    import_version_id UUID NOT NULL REFERENCES save_import_versions(id) ON DELETE RESTRICT,
    relative_path TEXT NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    is_supported BOOLEAN NOT NULL,
    ignored_reason TEXT,
    byte_size BIGINT NOT NULL,
    sha256 CHAR(64) NOT NULL,
    xxh64 CHAR(16) NOT NULL,
    immutable BOOLEAN NOT NULL DEFAULT TRUE,
    retention_policy TEXT NOT NULL DEFAULT 'forever',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (import_version_id, relative_path)
);

CREATE TABLE IF NOT EXISTS save_variant_metadata (
    id UUID PRIMARY KEY,
    save_file_id UUID NOT NULL UNIQUE REFERENCES save_files(id) ON DELETE RESTRICT,
    has_cnk_prefix BOOLEAN NOT NULL DEFAULT FALSE,
    magic TEXT,
    save_type SMALLINT,
    compression TEXT NOT NULL,
    uncompressed_size BIGINT,
    compressed_size BIGINT,
    gvas_magic TEXT,
    decompressed_size BIGINT,
    decode_status TEXT NOT NULL CHECK (decode_status IN ('not_attempted', 'ok', 'error')),
    decode_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
