-- Test tracing from authz-core to pgauthz

-- First, set the tracing level
SET authz.tracing_level = 'debug';

-- Create a simple model that will trigger tracing
SELECT pgauthz_define_policy('
    type user {}
    type document {
        relations define viewer: [user]
        permissions define view = viewer
    }
');

-- This should trigger tracing logs in authz-core
-- Check if tracing is working by looking for logs in PostgreSQL logs
SELECT pgauthz_check('document', 'doc1', 'view', 'user', 'alice');

-- Test different tracing levels
SET authz.tracing_level = 'error';
SELECT pgauthz_check('document', 'doc1', 'view', 'user', 'alice');

SET authz.tracing_level = 'info';
SELECT pgauthz_check('document', 'doc1', 'view', 'user', 'alice');

-- Show current GUC values
SHOW authz.tracing_level;
SHOW authz.check_strategy;
