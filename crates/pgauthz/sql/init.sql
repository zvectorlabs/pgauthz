-- authz schema and tables - Global model (SpiceDB-style)
-- Single global authorization model for the entire system.
-- Optimized indexes based on SpiceDB patterns for production workloads.

CREATE SCHEMA IF NOT EXISTS authz;

-- revision: global revision tracking
CREATE TABLE authz.revision (
    revision_id TEXT NOT NULL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- authorization_policy: global DSL definition
CREATE TABLE authz.authorization_policy (
    id TEXT NOT NULL PRIMARY KEY,
    definition TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- tuple: relationship storage (object#relation@subject)
CREATE TABLE authz.tuple (
    object_type TEXT NOT NULL,
    object_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    condition TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (object_type, object_id, relation, subject_type, subject_id)
);

-- Optimized indexes for tuple queries (SpiceDB-inspired)

-- 1. Comprehensive subject-first index for reverse lookups
-- Optimizes "what can this user access?" queries
CREATE INDEX authz_tuple_by_subject ON authz.tuple 
(subject_id, subject_type, object_type, object_id, relation);

-- 2. Object-subject composite for permission checks
-- Optimizes "can this user access this resource?" queries  
CREATE INDEX authz_tuple_object_subject ON authz.tuple 
(object_type, relation, subject_type, subject_id);

-- 3. Covering index for common permission queries
-- Includes frequently accessed columns to avoid table lookups
CREATE INDEX authz_tuple_covering ON authz.tuple 
(object_type, object_id, relation) INCLUDE (subject_type, subject_id, condition);

-- 4. Multi-column filter index for list operations
-- Supports common filter combinations used by list APIs
CREATE INDEX authz_tuple_filter ON authz.tuple 
(object_type, subject_type, relation);

-- 5. Watch API support index
-- Supports changelog/watch functionality by object type and time
CREATE INDEX authz_tuple_watch ON authz.tuple 
(object_type, created_at);

-- changelog: for Watch API
CREATE TABLE authz.changelog (
    object_type TEXT NOT NULL,
    object_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('write', 'delete')),
    ulid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Enhanced changelog indexes for better Watch API performance
CREATE INDEX authz_changelog_object_ulid ON authz.changelog (object_type, ulid);
CREATE INDEX authz_changelog_time ON authz.changelog (created_at);
CREATE INDEX authz_changelog_object_time ON authz.changelog (object_type, created_at);

-- assertion: for assertion tests
CREATE TABLE authz.assertion (
    id TEXT NOT NULL PRIMARY KEY,
    assertions JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Analyze tables for query planner optimization
ANALYZE authz.tuple;
ANALYZE authz.changelog;