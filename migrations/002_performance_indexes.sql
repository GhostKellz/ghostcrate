-- Performance indexes for production
CREATE INDEX IF NOT EXISTS idx_crates_name_lower ON crates(LOWER(name));
CREATE INDEX IF NOT EXISTS idx_crates_updated_at ON crates(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_crate_versions_crate_id_version ON crate_versions(crate_id, version);
CREATE INDEX IF NOT EXISTS idx_crate_versions_created_at ON crate_versions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_organization_members_org_user ON organization_members(organization_id, user_id);
CREATE INDEX IF NOT EXISTS idx_download_metrics_date_count ON download_metrics(date DESC, count DESC);

-- Full-text search indexes
CREATE VIRTUAL TABLE IF NOT EXISTS crates_fts USING fts5(
    name, description, keywords, 
    content='crates', 
    content_rowid='rowid'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS crates_fts_insert AFTER INSERT ON crates BEGIN
    INSERT INTO crates_fts(rowid, name, description, keywords) 
    VALUES (new.rowid, new.name, new.description, new.keywords);
END;

CREATE TRIGGER IF NOT EXISTS crates_fts_delete AFTER DELETE ON crates BEGIN
    INSERT INTO crates_fts(crates_fts, rowid, name, description, keywords) 
    VALUES('delete', old.rowid, old.name, old.description, old.keywords);
END;

CREATE TRIGGER IF NOT EXISTS crates_fts_update AFTER UPDATE ON crates BEGIN
    INSERT INTO crates_fts(crates_fts, rowid, name, description, keywords) 
    VALUES('delete', old.rowid, old.name, old.description, old.keywords);
    INSERT INTO crates_fts(rowid, name, description, keywords) 
    VALUES (new.rowid, new.name, new.description, new.keywords);
END;
