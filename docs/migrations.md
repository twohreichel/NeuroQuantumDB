# Online Schema Migration

NeuroQuantumDB supports online schema migrations that allow you to modify database schemas without downtime or blocking reads/writes in production environments.

## Features

- **Non-Blocking DDL Operations**: Execute schema changes without blocking reads/writes
- **Version-Controlled Migrations**: Track migration history and versions
- **Up/Down Migrations**: Apply and rollback schema changes
- **Dry-Run Mode**: Preview migration changes before applying
- **Progress Tracking**: Monitor long-running migrations
- **Safety Checks**: Pre-migration validation and automatic rollback on failure
- **Concurrent Operations**: Use `CONCURRENTLY` keyword for index operations

## Quick Start

### 1. Create a New Migration

```bash
neuroquantum-api migrate create "add status column"
```

This creates two files:
- `migrations/001_add_status_column.up.sql` - SQL to apply the migration
- `migrations/001_add_status_column.down.sql` - SQL to revert the migration

### 2. Edit Migration Files

**Up Migration (001_add_status_column.up.sql):**
```sql
-- Migration: add status column
-- Created: 2024-01-10 00:00:00

ALTER TABLE users ADD COLUMN status TEXT DEFAULT 'active' CONCURRENTLY;
```

**Down Migration (001_add_status_column.down.sql):**
```sql
-- Migration: add status column
-- Created: 2024-01-10 00:00:00

ALTER TABLE users DROP COLUMN status CONCURRENTLY;
```

### 3. Check Migration Status

```bash
neuroquantum-api migrate status
```

Output:
```
📋 Migration Status

Migration ID          | Status   | Description
----------------------|----------|--------------------
001                   | ⏳ Pending | add status column
```

### 4. Apply Migrations

```bash
neuroquantum-api migrate up
```

Output:
```
🚀 Running migrations...

📊 Migration Results:

  ✅ 001 - 1234ms

✅ Migration complete
```

### 5. Rollback Migrations (if needed)

```bash
neuroquantum-api migrate down --count 1
```

## SQL Syntax

### Non-Blocking Column Addition

```sql
-- Add a column without blocking reads/writes
ALTER TABLE users ADD COLUMN status TEXT DEFAULT 'active' CONCURRENTLY;
```

### Non-Blocking Column Removal

```sql
-- Drop a column without blocking
ALTER TABLE users DROP COLUMN old_field CONCURRENTLY;
```

### Concurrent Index Creation

```sql
-- Create an index without blocking writes
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);

-- Create a unique index concurrently
CREATE UNIQUE INDEX CONCURRENTLY idx_users_username ON users(username);
```

### Concurrent Index Removal

```sql
-- Drop an index without blocking
DROP INDEX CONCURRENTLY idx_old_index;
```

### Type Change with USING Clause

```sql
-- Change column type with explicit conversion
ALTER TABLE orders 
  ALTER COLUMN amount TYPE DECIMAL(10,2) 
  USING amount::DECIMAL 
  CONCURRENTLY;
```

## CLI Commands

### `migrate create <name>`

Create a new migration file pair.

**Options:**
- `--dir <path>`: Migrations directory (default: `migrations`)

**Example:**
```bash
neuroquantum-api migrate create "add user roles"
```

### `migrate up`

Run all pending migrations.

**Options:**
- `--dir <path>`: Migrations directory (default: `migrations`)
- `--dry-run`: Preview changes without applying them
- `--verbose`: Show detailed output

**Example:**
```bash
neuroquantum-api migrate up --dry-run
```

### `migrate down`

Rollback the last N migrations.

**Options:**
- `--count <n>`: Number of migrations to rollback (default: 1)
- `--dir <path>`: Migrations directory (default: `migrations`)
- `--dry-run`: Preview changes without applying them
- `--verbose`: Show detailed output

**Example:**
```bash
neuroquantum-api migrate down --count 2
```

### `migrate status`

Show the status of all migrations.

**Options:**
- `--dir <path>`: Migrations directory (default: `migrations`)

**Example:**
```bash
neuroquantum-api migrate status
```

## Migration File Format

Migration files follow a naming convention:
```
<number>_<description>.<direction>.sql
```

Examples:
- `001_add_status_column.up.sql`
- `001_add_status_column.down.sql`
- `002_create_user_roles.up.sql`
- `002_create_user_roles.down.sql`

### File Structure

```sql
-- Migration: <description>
-- Created: <timestamp>

-- SQL statements go here
ALTER TABLE users ADD COLUMN status TEXT;
CREATE INDEX idx_users_status ON users(status);
```

## Best Practices

### 1. Always Provide Down Migrations

Ensure every migration has a corresponding down migration for rollback:

```sql
-- up: Add column
ALTER TABLE users ADD COLUMN status TEXT;

-- down: Remove column
ALTER TABLE users DROP COLUMN status;
```

### 2. Use CONCURRENTLY for Index Operations

Always use `CONCURRENTLY` for index operations in production:

```sql
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);
```

### 3. Test in Non-Production First

Always test migrations in a staging environment before applying to production.

### 4. Use Dry-Run Mode

Preview migration changes before applying:

```bash
neuroquantum-api migrate up --dry-run --verbose
```

### 5. Keep Migrations Small

Break large schema changes into smaller, incremental migrations:

**Good:**
```
001_add_status_column.sql
002_add_role_column.sql
003_create_status_index.sql
```

**Avoid:**
```
001_massive_schema_overhaul.sql  # Too large!
```

### 6. Use Descriptive Names

Use clear, descriptive names for migrations:

**Good:**
```
001_add_user_status_column
002_create_email_index
003_add_role_foreign_key
```

**Avoid:**
```
001_schema_update
002_fixes
003_misc
```

## Safety Features

### Pre-Migration Validation

Before applying migrations, the system checks:
- SQL syntax validity
- Availability of rollback (down) migration
- Dangerous operations (DROP TABLE, TRUNCATE, etc.)
- Estimated disk space requirements

### Automatic Rollback

If a migration fails, the system automatically:
- Rolls back the failed migration
- Restores the previous state
- Records the failure in migration history

### Lock Timeout Configuration

Configure timeout for database locks:

```rust
let config = MigrationConfig {
    lock_timeout_secs: 300,  // 5 minutes
    ..Default::default()
};
```

### Dry-Run Mode

Test migrations without applying changes:

```bash
neuroquantum-api migrate up --dry-run --verbose
```

## Progress Tracking

For long-running migrations, track progress:

```rust
// In custom migration code
let progress = executor.progress();

// Start tracking
progress.start(migration_id, total_rows).await;

// Update progress
progress.update(processed_rows, "Processing...".to_string()).await;

// Complete
progress.complete().await;
```

## Common Migration Patterns

### Adding a Column with Default Value

```sql
-- Up
ALTER TABLE users ADD COLUMN status TEXT DEFAULT 'active' CONCURRENTLY;

-- Down
ALTER TABLE users DROP COLUMN status CONCURRENTLY;
```

### Creating an Index

```sql
-- Up
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);

-- Down
DROP INDEX CONCURRENTLY idx_users_email;
```

### Adding a Foreign Key

```sql
-- Up
ALTER TABLE orders ADD CONSTRAINT fk_user_id 
  FOREIGN KEY (user_id) REFERENCES users(id);

-- Down
ALTER TABLE orders DROP CONSTRAINT fk_user_id;
```

### Renaming a Column

```sql
-- Up
ALTER TABLE users RENAME COLUMN old_name TO new_name;

-- Down
ALTER TABLE users RENAME COLUMN new_name TO old_name;
```

### Changing Column Type

```sql
-- Up
ALTER TABLE orders 
  ALTER COLUMN price TYPE DECIMAL(10,2) 
  USING price::DECIMAL 
  CONCURRENTLY;

-- Down
ALTER TABLE orders 
  ALTER COLUMN price TYPE INTEGER 
  USING price::INTEGER 
  CONCURRENTLY;
```

## Troubleshooting

### Migration Fails with Lock Timeout

**Problem:** Migration times out waiting for locks.

**Solution:**
1. Increase lock timeout: `--lock-timeout 600`
2. Run during low-traffic periods
3. Use `CONCURRENTLY` for index operations

### Validation Errors

**Problem:** Pre-migration validation fails.

**Solution:**
1. Review validation checks in output
2. Fix SQL syntax errors
3. Add missing down migration
4. Remove dangerous operations

### Rollback Fails

**Problem:** Cannot rollback migration.

**Solution:**
1. Check down migration SQL
2. Verify database state
3. Manual intervention may be needed

## Architecture

The migration framework consists of:

- **Migration Parser**: Parses migration files from disk
- **Migration History**: Tracks applied migrations
- **Migration Executor**: Executes migrations with safety checks
- **Progress Tracker**: Monitors long-running operations

## Integration Examples

### Programmatic Usage

```rust
use neuroquantum_core::storage::{
    MigrationConfig, MigrationExecutor, MigrationExecutorConfig,
};

let config = MigrationExecutorConfig {
    config: MigrationConfig {
        migrations_dir: PathBuf::from("migrations"),
        dry_run: false,
        lock_timeout_secs: 300,
        auto_rollback: true,
        max_concurrent_ops: 4,
    },
    verbose: true,
};

let executor = MigrationExecutor::new(config);
executor.initialize().await?;

// Apply migrations
let results = executor.migrate_up().await?;

// Check status
let status = executor.status().await?;
```

## See Also

- [QSQL Language Reference](./quick_reference.md)
- [Developer Guide](./developer_guide.md)
- [API Documentation](../api-docs/)
