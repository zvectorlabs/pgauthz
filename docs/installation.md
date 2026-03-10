# Installation Guide

This guide explains how to install the pgauthz PostgreSQL extension.

## Prerequisites

- PostgreSQL 16 or later
- Linux or macOS operating system
- Superuser access to your PostgreSQL instance

## Installation from Binary Package

### Step 1: Download the Package

Download the latest pgauthz binary package for your platform:

```bash
# For Debian/Ubuntu
wget https://github.com/your-org/pgauthz/releases/latest/download/pgauthz-postgresql-16-amd64.deb

# For RHEL/CentOS/Fedora
wget https://github.com/your-org/pgauthz/releases/latest/download/pgauthz-postgresql-16-x86_64.rpm

# For macOS (Homebrew)
brew tap your-org/pgauthz
brew install pgauthz
```

### Step 2: Install the Package

```bash
# For Debian/Ubuntu
sudo dpkg -i pgauthz-postgresql-16-amd64.deb

# For RHEL/CentOS/Fedora
sudo rpm -i pgauthz-postgresql-16-x86_64.rpm

# For macOS (Homebrew)
# Already installed in step 1
```

### Step 3: Verify Installation

Check that the extension files are installed:

```bash
# Check for the control file
ls -l $(pg_config --sharedir)/extension/pgauthz.control

# Check for the shared library
ls -l $(pg_config --pkglibdir)/pgauthz.so
```

## Creating the Extension

### Step 1: Connect to PostgreSQL

Connect to your PostgreSQL database as a superuser:

```bash
psql -U postgres -d your_database
```

### Step 2: Create the Extension

```sql
CREATE EXTENSION pgauthz;
```

### Step 3: Verify Extension

Verify the extension is installed and working:

```sql
-- Check extension version
SELECT * FROM pg_extension WHERE extname = 'pgauthz';

-- Test basic functionality
SELECT pgauthz_define_policy('type user {}');
```

## Configuration

After installation, you may want to configure pgauthz settings. See the [Configuration Guide](configuration.md) for details.

### Basic Configuration

Add these settings to your `postgresql.conf`:

```ini
# Enable OpenTelemetry (optional)
authz.otel_enabled = false

# Set tracing level
authz.tracing_level = 'info'

# Configure caching
authz.model_cache_ttl_secs = 300
authz.result_cache_ttl_secs = 60
authz.tuple_cache_ttl_secs = 60
```

Reload PostgreSQL configuration:

```sql
SELECT pg_reload_conf();
```

## Upgrading

To upgrade pgauthz to a newer version:

### Step 1: Install New Package

Install the new package using the same method as initial installation.

### Step 2: Update Extension

```sql
ALTER EXTENSION pgauthz UPDATE;
```

### Step 3: Verify Upgrade

```sql
SELECT * FROM pg_extension WHERE extname = 'pgauthz';
```

## Uninstalling

To remove pgauthz:

### Step 1: Drop Extension

```sql
DROP EXTENSION pgauthz CASCADE;
```

### Step 2: Remove Package

```bash
# For Debian/Ubuntu
sudo dpkg -r pgauthz

# For RHEL/CentOS/Fedora
sudo rpm -e pgauthz

# For macOS (Homebrew)
brew uninstall pgauthz
```

## Troubleshooting

### Extension Not Found

**Error**: `ERROR: could not open extension control file`

**Solution**: Verify the extension files are in the correct location:

```bash
# Check PostgreSQL extension directory
pg_config --sharedir

# Verify pgauthz.control exists
ls -l $(pg_config --sharedir)/extension/pgauthz.control
```

### Shared Library Not Found

**Error**: `ERROR: could not load library`

**Solution**: Verify the shared library is installed:

```bash
# Check PostgreSQL library directory
pg_config --pkglibdir

# Verify pgauthz.so exists
ls -l $(pg_config --pkglibdir)/pgauthz.so
```

### Permission Denied

**Error**: `ERROR: permission denied to create extension`

**Solution**: You need superuser privileges to create extensions:

```sql
-- Connect as superuser
psql -U postgres -d your_database

-- Or grant privileges
ALTER USER your_user WITH SUPERUSER;
```

### Version Mismatch

**Error**: `ERROR: extension "pgauthz" has no update path`

**Solution**: Check available versions and update paths:

```sql
SELECT * FROM pg_available_extension_versions WHERE name = 'pgauthz';
```

### PostgreSQL Version Incompatibility

**Error**: `ERROR: incompatible library`

**Solution**: Ensure you're using PostgreSQL 16 or later:

```bash
psql --version
```

If you're on an older version, upgrade PostgreSQL first.

## Platform-Specific Notes

### Debian/Ubuntu

pgauthz packages are available for:
- Debian 11 (Bullseye)
- Debian 12 (Bookworm)
- Ubuntu 20.04 LTS
- Ubuntu 22.04 LTS
- Ubuntu 24.04 LTS

### RHEL/CentOS/Fedora

pgauthz packages are available for:
- RHEL 8
- RHEL 9
- CentOS Stream 8
- CentOS Stream 9
- Fedora 38+

### macOS

pgauthz is available via Homebrew for:
- macOS 12 (Monterey)
- macOS 13 (Ventura)
- macOS 14 (Sonoma)

Both Intel and Apple Silicon (M1/M2/M3) are supported.

## Docker Installation

For Docker environments, use the official pgauthz Docker image:

```bash
docker pull your-org/pgauthz:latest

docker run -d \
  --name postgres-pgauthz \
  -e POSTGRES_PASSWORD=mysecretpassword \
  -p 5432:5432 \
  your-org/pgauthz:latest
```

The extension is pre-installed and ready to use:

```bash
docker exec -it postgres-pgauthz psql -U postgres
```

```sql
CREATE EXTENSION pgauthz;
```

## Next Steps

- Follow the [Quick Start Guide](quickstart.md) to learn basic usage
- Review the [Configuration Guide](configuration.md) for tuning options
- Explore the [API Reference](api-reference.md) for available functions

## Getting Help

If you encounter issues not covered here:

1. Check the [Debugging Guide](debugging.md)
2. Search [GitHub Issues](https://github.com/your-org/pgauthz/issues)
3. Ask in [GitHub Discussions](https://github.com/your-org/pgauthz/discussions)
