# Condition-Based Authorization Example

Example of implementing context-based permissions using conditions.

## Scenario

Build a document system with conditional access:
- Users can edit documents only from whitelisted IP addresses
- Users can view sensitive documents only during business hours
- Users can access documents only if they meet custom criteria

## Step 1: Define Policy with Conditions

```sql
SELECT pgauthz_define_policy('
  type user {}
  
  -- Condition: Business hours (9 AM to 5 PM)
  condition business_hours {
    hour >= 9 && hour <= 17
  }
  
  -- Condition: IP whitelist
  condition ip_whitelist(allowed_ips: list<string>, current_ip: string) {
    current_ip in allowed_ips
  }
  
  -- Condition: Document classification level
  condition clearance_level(required_level: int, user_level: int) {
    user_level >= required_level
  }
  
  type document {
    relations
      define viewer: [user]
      define restricted_viewer: [user with business_hours]
      define editor: [user with ip_whitelist]
      define classified_viewer: [user with clearance_level]
  }
');
```

## Step 2: Add Relations with Conditions

```sql
-- Alice can view doc1 anytime (no condition)
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'alice');

-- Bob can view doc2 only during business hours
SELECT pgauthz_add_relation('document', 'doc2', 'restricted_viewer', 'user', 'bob', 'business_hours');

-- Charlie can edit doc3 only from whitelisted IPs
SELECT pgauthz_add_relation('document', 'doc3', 'editor', 'user', 'charlie', 'ip_whitelist');

-- Dave can view doc4 only with sufficient clearance
SELECT pgauthz_add_relation('document', 'doc4', 'classified_viewer', 'user', 'dave', 'clearance_level');
```

## Step 3: Check with Context

### Business Hours Example

```sql
-- Check during business hours (14:00 = 2 PM)
SELECT pgauthz_check_with_context(
  'document',
  'doc2',
  'restricted_viewer',
  'user',
  'bob',
  '{"hour": 14}'::jsonb
);
-- Returns: true

-- Check outside business hours (20:00 = 8 PM)
SELECT pgauthz_check_with_context(
  'document',
  'doc2',
  'restricted_viewer',
  'user',
  'bob',
  '{"hour": 20}'::jsonb
);
-- Returns: false

-- Check without context (fails)
SELECT pgauthz_check('document', 'doc2', 'restricted_viewer', 'user', 'bob');
-- Returns: false (condition not satisfied)
```

### IP Whitelist Example

```sql
-- Check from whitelisted IP
SELECT pgauthz_check_with_context(
  'document',
  'doc3',
  'editor',
  'user',
  'charlie',
  '{"allowed_ips": ["10.0.0.1", "10.0.0.2", "10.0.0.3"], "current_ip": "10.0.0.1"}'::jsonb
);
-- Returns: true

-- Check from non-whitelisted IP
SELECT pgauthz_check_with_context(
  'document',
  'doc3',
  'editor',
  'user',
  'charlie',
  '{"allowed_ips": ["10.0.0.1", "10.0.0.2"], "current_ip": "192.168.1.100"}'::jsonb
);
-- Returns: false
```

### Clearance Level Example

```sql
-- User has sufficient clearance (level 3 >= required level 2)
SELECT pgauthz_check_with_context(
  'document',
  'doc4',
  'classified_viewer',
  'user',
  'dave',
  '{"required_level": 2, "user_level": 3}'::jsonb
);
-- Returns: true

-- User has insufficient clearance (level 1 < required level 2)
SELECT pgauthz_check_with_context(
  'document',
  'doc4',
  'classified_viewer',
  'user',
  'dave',
  '{"required_level": 2, "user_level": 1}'::jsonb
);
-- Returns: false
```

## Step 4: Application Integration

### Python Example with Business Hours

```python
import psycopg2
import json
from datetime import datetime

def can_user_view_during_business_hours(user_id, document_id):
    """Check if user can view document during current time."""
    current_hour = datetime.now().hour
    
    conn = psycopg2.connect("dbname=mydb")
    cur = conn.cursor()
    
    context = json.dumps({"hour": current_hour})
    
    cur.execute(
        "SELECT pgauthz_check_with_context(%s, %s, %s, %s, %s, %s::jsonb)",
        ('document', document_id, 'restricted_viewer', 'user', user_id, context)
    )
    
    result = cur.fetchone()[0]
    cur.close()
    conn.close()
    
    return result

# Usage
if can_user_view_during_business_hours('bob', 'doc2'):
    print("Access granted")
else:
    print("Access denied - outside business hours")
```

### Python Example with IP Whitelist

```python
def can_user_edit_from_ip(user_id, document_id, client_ip):
    """Check if user can edit document from their IP."""
    # Fetch allowed IPs from database or config
    allowed_ips = get_allowed_ips_for_user(user_id)
    
    conn = psycopg2.connect("dbname=mydb")
    cur = conn.cursor()
    
    context = json.dumps({
        "allowed_ips": allowed_ips,
        "current_ip": client_ip
    })
    
    cur.execute(
        "SELECT pgauthz_check_with_context(%s, %s, %s, %s, %s, %s::jsonb)",
        ('document', document_id, 'editor', 'user', user_id, context)
    )
    
    result = cur.fetchone()[0]
    cur.close()
    conn.close()
    
    return result

# Usage in web application
from flask import request

@app.route('/document/<doc_id>/edit', methods=['POST'])
def edit_document(doc_id):
    user_id = get_current_user()
    client_ip = request.remote_addr
    
    if not can_user_edit_from_ip(user_id, doc_id, client_ip):
        return {"error": "Access denied from this IP"}, 403
    
    # Process edit
    return {"success": True}
```

### Node.js Example with Multiple Conditions

```javascript
async function checkDocumentAccess(userId, documentId, context) {
  const pool = new Pool({ database: 'mydb' });
  
  const result = await pool.query(
    'SELECT pgauthz_check_with_context($1, $2, $3, $4, $5, $6::jsonb)',
    ['document', documentId, 'viewer', 'user', userId, JSON.stringify(context)]
  );
  
  return result.rows[0].pgauthz_check_with_context;
}

// Usage with business hours
const currentHour = new Date().getHours();
const canView = await checkDocumentAccess('bob', 'doc2', {
  hour: currentHour
});

// Usage with IP whitelist
const canEdit = await checkDocumentAccess('charlie', 'doc3', {
  allowed_ips: ['10.0.0.1', '10.0.0.2'],
  current_ip: req.ip
});

// Usage with clearance level
const canViewClassified = await checkDocumentAccess('dave', 'doc4', {
  required_level: 2,
  user_level: getUserClearanceLevel('dave')
});
```

## Advanced Examples

### Multiple Conditions

Combine multiple conditions:

```sql
SELECT pgauthz_define_policy('
  type user {}
  
  condition business_hours {
    hour >= 9 && hour <= 17
  }
  
  condition ip_whitelist(allowed_ips: list<string>, current_ip: string) {
    current_ip in allowed_ips
  }
  
  type document {
    relations
      define secure_editor: [user with business_hours with ip_whitelist]
  }
');

-- Add relation requiring both conditions
SELECT pgauthz_add_relation(
  'document', 'secure_doc', 'secure_editor', 'user', 'alice',
  'business_hours'  -- Note: Currently only one condition per relation
);
```

### Dynamic Context from Database

```python
def check_with_user_attributes(user_id, document_id, relation):
    """Check permission using user attributes from database."""
    # Fetch user attributes
    user = get_user_from_db(user_id)
    
    context = {
        "user_level": user.clearance_level,
        "user_department": user.department,
        "user_role": user.role
    }
    
    return check_with_context(
        'document', document_id, relation, 'user', user_id,
        context
    )
```

### Time-Based Access Windows

```sql
-- Condition: Access only during specific time window
condition time_window(start_hour: int, end_hour: int, current_hour: int) {
  current_hour >= start_hour && current_hour <= end_hour
}

-- Usage
SELECT pgauthz_check_with_context(
  'document', 'doc5', 'viewer', 'user', 'eve',
  '{"start_hour": 9, "end_hour": 17, "current_hour": 14}'::jsonb
);
```

### Geographic Restrictions

```sql
-- Condition: Access only from specific countries
condition geo_restriction(allowed_countries: list<string>, user_country: string) {
  user_country in allowed_countries
}

-- Usage
SELECT pgauthz_check_with_context(
  'document', 'doc6', 'viewer', 'user', 'frank',
  '{"allowed_countries": ["US", "CA", "UK"], "user_country": "US"}'::jsonb
);
```

## Middleware Pattern

### Express.js Middleware

```javascript
function requireBusinessHours(req, res, next) {
  const currentHour = new Date().getHours();
  
  if (currentHour < 9 || currentHour > 17) {
    return res.status(403).json({
      error: 'Access denied outside business hours'
    });
  }
  
  next();
}

function requireWhitelistedIP(allowedIPs) {
  return (req, res, next) => {
    const clientIP = req.ip;
    
    if (!allowedIPs.includes(clientIP)) {
      return res.status(403).json({
        error: 'Access denied from this IP address'
      });
    }
    
    next();
  };
}

// Usage
app.get('/document/:id',
  requireBusinessHours,
  requireWhitelistedIP(['10.0.0.1', '10.0.0.2']),
  async (req, res) => {
    // Handle request
  }
);
```

## Testing Conditions

### Test Business Hours

```sql
-- Test all hours of the day
DO $$
BEGIN
  FOR hour IN 0..23 LOOP
    RAISE NOTICE 'Hour %: %',
      hour,
      pgauthz_check_with_context(
        'document', 'doc2', 'restricted_viewer', 'user', 'bob',
        format('{"hour": %s}', hour)::jsonb
      );
  END LOOP;
END $$;
```

### Test IP Ranges

```sql
-- Test various IPs
DO $$
DECLARE
  test_ips text[] := ARRAY['10.0.0.1', '10.0.0.5', '192.168.1.1'];
  ip text;
BEGIN
  FOREACH ip IN ARRAY test_ips LOOP
    RAISE NOTICE 'IP %: %',
      ip,
      pgauthz_check_with_context(
        'document', 'doc3', 'editor', 'user', 'charlie',
        format('{"allowed_ips": ["10.0.0.1", "10.0.0.2"], "current_ip": "%s"}', ip)::jsonb
      );
  END LOOP;
END $$;
```

## Best Practices

1. **Keep context small** - Only include necessary variables
2. **Validate context** - Check context values before passing to pgauthz
3. **Cache context** - Cache user attributes to avoid repeated lookups
4. **Log denials** - Log when conditions fail for security auditing
5. **Test conditions** - Thoroughly test all condition branches
6. **Document conditions** - Clearly document what each condition checks
7. **Use meaningful names** - Name conditions descriptively

## Common Pitfalls

### Missing Context

```python
# Bad: No context provided for conditional relation
result = check('document', 'doc2', 'restricted_viewer', 'user', 'bob')
# Returns: false (condition not satisfied)

# Good: Context provided
result = check_with_context(
    'document', 'doc2', 'restricted_viewer', 'user', 'bob',
    {"hour": 14}
)
# Returns: true (if during business hours)
```

### Wrong Context Keys

```python
# Bad: Wrong key name
context = {"current_hour": 14}  # Should be "hour"

# Good: Correct key name
context = {"hour": 14}
```

### Type Mismatches

```python
# Bad: String instead of integer
context = {"hour": "14"}

# Good: Integer value
context = {"hour": 14}
```

## See Also

- [Basic Authorization Example](basic-authorization.md) - Simple authorization patterns
- [API Reference](../api-reference.md) - Complete function documentation
- [Performance Guide](../performance.md) - Optimization tips
