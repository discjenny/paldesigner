CREATE TABLE IF NOT EXISTS save_patchsets (
    id UUID PRIMARY KEY,
    import_version_id UUID NOT NULL REFERENCES save_import_versions(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    validated BOOLEAN NOT NULL DEFAULT FALSE,
    validation_error TEXT
);

CREATE TABLE IF NOT EXISTS save_patch_operations (
    id UUID PRIMARY KEY,
    patchset_id UUID NOT NULL REFERENCES save_patchsets(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    op_type TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    validated BOOLEAN NOT NULL DEFAULT FALSE,
    validation_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (patchset_id, sequence)
);
