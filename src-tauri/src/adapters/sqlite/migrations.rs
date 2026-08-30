pub const SCHEMA_VERSION: u32 = 1;

pub const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    checksum TEXT NOT NULL,
    applied_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS facilities (
    facility_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    timezone TEXT NOT NULL,
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    created_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workstations (
    workstation_id TEXT PRIMARY KEY,
    facility_id TEXT NOT NULL REFERENCES facilities(facility_id),
    label TEXT NOT NULL,
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    created_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    user_id TEXT PRIMARY KEY,
    facility_id TEXT NOT NULL REFERENCES facilities(facility_id),
    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
    display_name TEXT NOT NULL,
    password_verifier TEXT NOT NULL,
    verifier_version INTEGER NOT NULL,
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    failed_attempt_count INTEGER NOT NULL DEFAULT 0,
    failed_attempt_window_started_at_utc TEXT,
    locked_until_utc TEXT,
    revision INTEGER NOT NULL DEFAULT 1,
    created_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS roles (
    role_code TEXT PRIMARY KEY,
    display_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS permissions (
    permission_code TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_code TEXT NOT NULL REFERENCES roles(role_code),
    permission_code TEXT NOT NULL REFERENCES permissions(permission_code),
    PRIMARY KEY (role_code, permission_code)
);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id TEXT NOT NULL REFERENCES users(user_id),
    role_code TEXT NOT NULL REFERENCES roles(role_code),
    assigned_at_utc TEXT NOT NULL,
    PRIMARY KEY (user_id, role_code)
);

CREATE TABLE IF NOT EXISTS auth_sessions (
    session_id TEXT PRIMARY KEY,
    token_sha256 TEXT NOT NULL UNIQUE,
    user_id TEXT NOT NULL REFERENCES users(user_id),
    workstation_id TEXT NOT NULL REFERENCES workstations(workstation_id),
    created_at_utc TEXT NOT NULL,
    expires_at_utc TEXT NOT NULL,
    recent_auth_at_utc TEXT NOT NULL,
    last_activity_at_utc TEXT NOT NULL,
    revoked_at_utc TEXT
);

CREATE TABLE IF NOT EXISTS patients (
    patient_id TEXT PRIMARY KEY,
    revision INTEGER NOT NULL,
    given_names TEXT NOT NULL,
    middle_names TEXT,
    first_surname TEXT NOT NULL,
    second_surname TEXT,
    suffix TEXT,
    preferred_name TEXT,
    date_of_birth TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(user_id),
    created_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS patient_addresses (
    patient_address_id TEXT PRIMARY KEY,
    patient_id TEXT NOT NULL REFERENCES patients(patient_id),
    line1 TEXT NOT NULL,
    line2 TEXT,
    municipality TEXT NOT NULL,
    region TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    country_code TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS external_identifiers (
    external_identifier_id TEXT PRIMARY KEY,
    patient_id TEXT NOT NULL REFERENCES patients(patient_id),
    identifier_type TEXT NOT NULL,
    assigning_authority TEXT NOT NULL,
    identifier_value TEXT NOT NULL,
    UNIQUE (identifier_type, assigning_authority, identifier_value)
);

CREATE TABLE IF NOT EXISTS immunization_encounters (
    encounter_id TEXT PRIMARY KEY,
    patient_id TEXT NOT NULL REFERENCES patients(patient_id),
    facility_id TEXT NOT NULL REFERENCES facilities(facility_id),
    responsible_professional_id TEXT NOT NULL REFERENCES users(user_id),
    state TEXT NOT NULL CHECK (state IN (
        'DRAFT', 'READY_TO_ADMINISTER', 'ADMINISTERED_PENDING_DOCUMENTATION',
        'FINALIZED', 'REGISTRY_READY', 'CORRECTED', 'VOIDED'
    )),
    revision INTEGER NOT NULL,
    created_at_utc TEXT NOT NULL,
    updated_at_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_event_id TEXT NOT NULL UNIQUE,
    occurred_at_utc TEXT NOT NULL,
    recorded_at_utc TEXT NOT NULL,
    actor_id TEXT,
    session_id TEXT,
    workstation_id TEXT NOT NULL,
    facility_id TEXT NOT NULL,
    action_code TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_revision INTEGER,
    outcome TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    software_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    metadata_json TEXT NOT NULL,
    previous_hash TEXT,
    event_hash TEXT NOT NULL UNIQUE
);

CREATE TRIGGER IF NOT EXISTS audit_events_no_update
BEFORE UPDATE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'AUDIT_APPEND_ONLY');
END;

CREATE TRIGGER IF NOT EXISTS audit_events_no_delete
BEFORE DELETE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'AUDIT_APPEND_ONLY');
END;

CREATE INDEX IF NOT EXISTS idx_patients_name ON patients(first_surname, given_names);
CREATE INDEX IF NOT EXISTS idx_encounters_patient ON immunization_encounters(patient_id);
CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_events(entity_type, entity_id, sequence);
"#;
