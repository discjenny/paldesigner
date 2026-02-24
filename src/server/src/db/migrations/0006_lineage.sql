CREATE TABLE IF NOT EXISTS save_export_lineage (
    id UUID PRIMARY KEY,
    export_version_id UUID NOT NULL UNIQUE REFERENCES save_export_versions(id) ON DELETE RESTRICT,
    import_version_id UUID NOT NULL REFERENCES save_import_versions(id) ON DELETE RESTRICT,
    patchset_id UUID NOT NULL REFERENCES save_patchsets(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
