CREATE TABLE IF NOT EXISTS save_import_versions (
    id UUID PRIMARY KEY,
    source_file_name TEXT NOT NULL,
    world_root_path TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('processing', 'ready', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS save_export_versions (
    id UUID PRIMARY KEY,
    import_version_id UUID NOT NULL REFERENCES save_import_versions(id) ON DELETE RESTRICT,
    patchset_id UUID,
    status TEXT NOT NULL CHECK (status IN ('processing', 'ready', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
