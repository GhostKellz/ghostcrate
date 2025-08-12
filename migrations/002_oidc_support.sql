-- Add OIDC support to GhostCrate
-- This migration adds tables for OIDC provider configuration and user links

-- OIDC Providers table
CREATE TABLE IF NOT EXISTS oidc_providers (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    provider_type TEXT NOT NULL, -- 'entraid', 'github', 'google', 'generic'
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    discovery_url TEXT,
    authority_url TEXT,
    scopes TEXT NOT NULL, -- JSON array as text
    enabled BOOLEAN NOT NULL DEFAULT 1,
    auto_register BOOLEAN NOT NULL DEFAULT 1,
    default_role TEXT,
    claim_mappings TEXT, -- JSON object for claim mappings
    config_json TEXT, -- Additional provider-specific config
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- OIDC User Links table - links local users to external OIDC identities
CREATE TABLE IF NOT EXISTS oidc_user_links (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider_id TEXT,
    provider_type TEXT NOT NULL, -- For simple provider identification
    external_id TEXT NOT NULL, -- Subject ID from OIDC provider
    email TEXT NOT NULL,
    name TEXT,
    avatar_url TEXT,
    last_login TEXT,
    metadata_json TEXT, -- Store additional claims/metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES oidc_providers (id) ON DELETE SET NULL,
    UNIQUE(provider_type, external_id)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_oidc_user_links_user_id ON oidc_user_links(user_id);
CREATE INDEX IF NOT EXISTS idx_oidc_user_links_provider ON oidc_user_links(provider_type, external_id);
CREATE INDEX IF NOT EXISTS idx_oidc_user_links_email ON oidc_user_links(email);

-- Update users table to support OIDC users (password_hash can be empty for OIDC-only users)
-- This is handled by making password_hash nullable in application logic since SQLite doesn't have good ALTER support

-- Insert default OIDC providers if they don't exist
INSERT OR IGNORE INTO oidc_providers (
    id, name, provider_type, client_id, client_secret, discovery_url, scopes, enabled, auto_register, created_at, updated_at
) VALUES 
(
    'entraid-default', 
    'Microsoft Entra ID', 
    'entraid', 
    '', 
    '', 
    '', 
    '["openid", "profile", "email", "User.Read"]', 
    0, 
    1, 
    datetime('now'), 
    datetime('now')
),
(
    'github-default', 
    'GitHub', 
    'github', 
    '', 
    '', 
    'https://github.com/login/oauth/authorize', 
    '["user:email", "read:org"]', 
    0, 
    1, 
    datetime('now'), 
    datetime('now')
);
