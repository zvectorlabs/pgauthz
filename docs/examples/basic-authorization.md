# Basic Authorization Example

Complete example of implementing basic authorization for a document management system.

## Scenario

Build a document management system where:
- Users can view, edit, or own documents
- Owners can do everything editors can do
- Editors can do everything viewers can do

## Step 1: Define the Policy

```sql
SELECT pgauthz_define_policy('
  type user {}
  
  type document {
    relations
      define viewer: [user]
      define editor: [user] | viewer
      define owner: [user] | editor
  }
');
```

This policy creates a hierarchy:
- `owner` includes `editor` permissions
- `editor` includes `viewer` permissions

## Step 2: Create Sample Data

```sql
-- Alice owns doc1
SELECT pgauthz_add_relation('document', 'doc1', 'owner', 'user', 'alice');

-- Bob can edit doc1
SELECT pgauthz_add_relation('document', 'doc1', 'editor', 'user', 'bob');

-- Charlie can view doc1
SELECT pgauthz_add_relation('document', 'doc1', 'viewer', 'user', 'charlie');

-- Alice owns doc2
SELECT pgauthz_add_relation('document', 'doc2', 'owner', 'user', 'alice');

-- Dave can view doc2
SELECT pgauthz_add_relation('document', 'doc2', 'viewer', 'user', 'dave');
```

## Step 3: Check Permissions

```sql
-- Alice is owner of doc1
SELECT pgauthz_check('document', 'doc1', 'owner', 'user', 'alice');
-- Returns: true

-- Alice can also edit (because owner includes editor)
SELECT pgauthz_check('document', 'doc1', 'editor', 'user', 'alice');
-- Returns: true

-- Alice can also view (because owner includes editor includes viewer)
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'alice');
-- Returns: true

-- Bob is editor but not owner
SELECT pgauthz_check('document', 'doc1', 'owner', 'user', 'bob');
-- Returns: false

SELECT pgauthz_check('document', 'doc1', 'editor', 'user', 'bob');
-- Returns: true

-- Charlie is viewer but not editor
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'charlie');
-- Returns: true

SELECT pgauthz_check('document', 'doc1', 'editor', 'user', 'charlie');
-- Returns: false
```

## Step 4: List Operations

Find all documents a user can access:

```sql
-- All documents Alice owns
SELECT * FROM pgauthz_list_objects('user', 'alice', 'owner', 'document');
-- Returns: doc1, doc2

-- All documents Bob can edit
SELECT * FROM pgauthz_list_objects('user', 'bob', 'editor', 'document');
-- Returns: doc1

-- All documents Charlie can view
SELECT * FROM pgauthz_list_objects('user', 'charlie', 'viewer', 'document');
-- Returns: doc1
```

Find all users with access to a document:

```sql
-- All users who can view doc1
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'viewer', 'user');
-- Returns: alice, bob, charlie

-- All users who can edit doc1
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'editor', 'user');
-- Returns: alice, bob

-- All users who own doc1
SELECT * FROM pgauthz_list_subjects('document', 'doc1', 'owner', 'user');
-- Returns: alice
```

## Step 5: Application Integration

### Python Example

```python
import psycopg2

def can_user_view_document(user_id, document_id):
    """Check if user can view document."""
    conn = psycopg2.connect("dbname=mydb")
    cur = conn.cursor()
    
    cur.execute(
        "SELECT pgauthz_check(%s, %s, %s, %s, %s)",
        ('document', document_id, 'viewer', 'user', user_id)
    )
    
    result = cur.fetchone()[0]
    cur.close()
    conn.close()
    
    return result

def get_user_documents(user_id):
    """Get all documents user can view."""
    conn = psycopg2.connect("dbname=mydb")
    cur = conn.cursor()
    
    cur.execute(
        "SELECT * FROM pgauthz_list_objects(%s, %s, %s, %s)",
        ('user', user_id, 'viewer', 'document')
    )
    
    documents = [row[0] for row in cur.fetchall()]
    cur.close()
    conn.close()
    
    return documents

def grant_document_access(document_id, user_id, role):
    """Grant user access to document."""
    conn = psycopg2.connect("dbname=mydb")
    cur = conn.cursor()
    
    cur.execute(
        "SELECT pgauthz_add_relation(%s, %s, %s, %s, %s)",
        ('document', document_id, role, 'user', user_id)
    )
    
    conn.commit()
    cur.close()
    conn.close()

# Usage
if can_user_view_document('alice', 'doc1'):
    print("Alice can view doc1")

docs = get_user_documents('bob')
print(f"Bob can view: {docs}")

grant_document_access('doc3', 'charlie', 'viewer')
```

### Node.js Example

```javascript
const { Pool } = require('pg');
const pool = new Pool({ database: 'mydb' });

async function canUserViewDocument(userId, documentId) {
  const result = await pool.query(
    'SELECT pgauthz_check($1, $2, $3, $4, $5)',
    ['document', documentId, 'viewer', 'user', userId]
  );
  return result.rows[0].pgauthz_check;
}

async function getUserDocuments(userId) {
  const result = await pool.query(
    'SELECT * FROM pgauthz_list_objects($1, $2, $3, $4)',
    ['user', userId, 'viewer', 'document']
  );
  return result.rows.map(row => row.pgauthz_list_objects);
}

async function grantDocumentAccess(documentId, userId, role) {
  await pool.query(
    'SELECT pgauthz_add_relation($1, $2, $3, $4, $5)',
    ['document', documentId, role, 'user', userId]
  );
}

// Usage
(async () => {
  if (await canUserViewDocument('alice', 'doc1')) {
    console.log('Alice can view doc1');
  }
  
  const docs = await getUserDocuments('bob');
  console.log('Bob can view:', docs);
  
  await grantDocumentAccess('doc3', 'charlie', 'viewer');
})();
```

### Go Example

```go
package main

import (
    "database/sql"
    _ "github.com/lib/pq"
)

func canUserViewDocument(db *sql.DB, userID, documentID string) (bool, error) {
    var result bool
    err := db.QueryRow(
        "SELECT pgauthz_check($1, $2, $3, $4, $5)",
        "document", documentID, "viewer", "user", userID,
    ).Scan(&result)
    return result, err
}

func getUserDocuments(db *sql.DB, userID string) ([]string, error) {
    rows, err := db.Query(
        "SELECT * FROM pgauthz_list_objects($1, $2, $3, $4)",
        "user", userID, "viewer", "document",
    )
    if err != nil {
        return nil, err
    }
    defer rows.Close()
    
    var documents []string
    for rows.Next() {
        var doc string
        if err := rows.Scan(&doc); err != nil {
            return nil, err
        }
        documents = append(documents, doc)
    }
    return documents, nil
}

func grantDocumentAccess(db *sql.DB, documentID, userID, role string) error {
    _, err := db.Exec(
        "SELECT pgauthz_add_relation($1, $2, $3, $4, $5)",
        "document", documentID, role, "user", userID,
    )
    return err
}

func main() {
    db, _ := sql.Open("postgres", "dbname=mydb")
    defer db.Close()
    
    if can, _ := canUserViewDocument(db, "alice", "doc1"); can {
        println("Alice can view doc1")
    }
    
    docs, _ := getUserDocuments(db, "bob")
    println("Bob can view:", docs)
    
    grantDocumentAccess(db, "doc3", "charlie", "viewer")
}
```

## Step 6: Revoke Access

Remove permissions:

```sql
-- Create a tuple to delete
WITH to_delete AS (
  SELECT 'document'::text as object_type,
         'doc1'::text as object_id,
         'viewer'::text as relation,
         'user'::text as subject_type,
         'charlie'::text as subject_id,
         NULL::text as condition
)
SELECT pgauthz_write_tuples(
  ARRAY[]::pgauthz_tuple[],  -- no writes
  ARRAY[(SELECT ROW(object_type, object_id, relation, subject_type, subject_id, condition)::pgauthz_tuple FROM to_delete)]
);

-- Verify Charlie can no longer view doc1
SELECT pgauthz_check('document', 'doc1', 'viewer', 'user', 'charlie');
-- Returns: false
```

## Step 7: Audit Access

View all current permissions:

```sql
-- All permissions for doc1
SELECT * FROM pgauthz_read_tuples('document', 'doc1', NULL, NULL, NULL);

-- All permissions for alice
SELECT * FROM pgauthz_read_tuples(NULL, NULL, NULL, 'user', 'alice');

-- All viewer permissions
SELECT * FROM pgauthz_read_tuples(NULL, NULL, 'viewer', NULL, NULL);
```

## Common Patterns

### Check Before Action

```python
def update_document(user_id, document_id, content):
    # Check permission first
    if not can_user_edit_document(user_id, document_id):
        raise PermissionError("User cannot edit document")
    
    # Perform update
    update_document_content(document_id, content)
```

### Filter Query Results

```python
def get_documents_for_user(user_id):
    # Get list of accessible documents
    accessible_docs = get_user_documents(user_id)
    
    # Fetch document details
    documents = fetch_documents(accessible_docs)
    
    return documents
```

### Bulk Permission Check

```python
def check_multiple_documents(user_id, document_ids):
    # Get all documents user can view
    accessible = set(get_user_documents(user_id))
    
    # Check which requested documents are accessible
    return {
        doc_id: doc_id in accessible
        for doc_id in document_ids
    }
```

## Best Practices

1. **Check permissions before every action**
2. **Use list operations for filtering** instead of checking each item
3. **Cache permission checks** in your application layer
4. **Log permission denials** for security auditing
5. **Use hierarchical relations** to simplify permission management
6. **Revoke permissions explicitly** when no longer needed

## See Also

- [Condition-Based Example](condition-based.md) - Add context-based permissions
- [Performance Testing Example](performance-testing.md) - Benchmark your setup
- [API Reference](../api-reference.md) - Complete function documentation
