# Getting Started with pgauthz Documentation

This repository contains the complete documentation system for pgauthz, including user documentation, skills repository, agent integration, and version management.

## What's Been Created

### 📚 Core Documentation
- **index.md** - Home page with overview and quick links
- **installation.md** - Binary installation guide (user-focused, no Rust/cargo details)
- **quickstart.md** - 5-minute tutorial with examples
- **api-reference.md** - Complete SQL function reference with all APIs including:
  - Authorization checks: `pgauthz_check()`, `pgauthz_check_with_context()`, `pgauthz_expand()`
  - List operations: `pgauthz_list_objects()`, `pgauthz_list_subjects()`
  - Relation management: `pgauthz_add_relation()`, `pgauthz_read_tuples()`
  - Policy management: `pgauthz_define_policy()`, `pgauthz_read_model()`, `pgauthz_read_latest_model()`, `pgauthz_list_models()`
  - Change tracking: `pgauthz_read_changes()`
- **configuration.md** - GUC parameters and tuning guide
- **observability.md** - OpenTelemetry metrics and tracing setup
- **debugging.md** - Troubleshooting guide with error codes
- **performance.md** - Optimization strategies and best practices

### 📖 Examples
- **basic-authorization.md** - Simple document authorization with Python/Node.js/Go examples
- **condition-based.md** - Context-based permissions (IP whitelist, business hours, clearance levels)
- **performance-testing.md** - Benchmarking and load testing guide

### 🎯 Skills Repository
- **skills-manifest.json** - Registry of available skills
- Skills structure for downloadable patterns:
  - authorization-check
  - policy-validation
  - cache-optimization
  - monitoring-setup
  - troubleshooting

### 🤖 Agent Integration
- **agents/README.md** - API documentation for AI agents
- Agent-specific guides for:
  - Code assistant agents
  - Documentation bots
  - Debugging agents
  - Monitoring agents
- JSON API endpoints for programmatic access

### 🔄 Version Management
- **versions/version-map.json** - Git hash-based version mapping
- Short hash format (8 characters): `a1b2c3d4`
- 404 fallback strategy (no automatic fallback to latest)
- Permanent storage (all hashes kept forever)

### 🛠️ Build & Deploy Infrastructure
- **mkdocs.yml** - MkDocs configuration with Material theme
- **scripts/build-docs.sh** - Documentation build script
- **scripts/update-hash-mappings.sh** - Version mapping updater
- **.github/workflows/deploy-docs.yml** - Automated deployment workflow

## Directory Structure

```
pgauthz/
├── README.md                    # Project overview
├── docs/                        # Documentation source
│   ├── index.md
│   ├── installation.md
│   ├── quickstart.md
│   ├── api-reference.md
│   ├── configuration.md
│   ├── observability.md
│   ├── debugging.md
│   ├── performance.md
│   └── examples/
│       ├── basic-authorization.md
│       ├── condition-based.md
│       └── performance-testing.md
├── skills/                      # Skills repository
│   ├── README.md
│   ├── skills-manifest.json
│   └── skills/
├── agents/                      # Agent documentation
│   ├── README.md
│   └── agent-types/
├── versions/                    # Version management
│   └── version-map.json
├── web/                         # Web serving
│   ├── api/
│   └── config/
├── scripts/                     # Build scripts
│   ├── build-docs.sh
│   └── update-hash-mappings.sh
├── .github/workflows/           # CI/CD
│   └── deploy-docs.yml
├── mkdocs.yml                   # MkDocs config
├── package.json                 # Node dependencies
└── .gitignore
```

## Next Steps

### 1. Local Development

Install dependencies:
```bash
pip install mkdocs mkdocs-material mkdocs-git-revision-date-localized-plugin
```

Serve locally:
```bash
cd /Users/rpatel9/projects/zanzibar/pgauthz
mkdocs serve
```

Visit http://localhost:8000

### 2. GitHub Repository Setup

Initialize git repository:
```bash
cd /Users/rpatel9/projects/zanzibar/pgauthz
git init
git add .
git commit -m "Initial pgauthz documentation system"
```

Create GitHub repository and push:
```bash
git remote add origin https://github.com/your-org/pgauthz.git
git branch -M main
git push -u origin main
```

### 3. GitHub Pages Setup

1. Go to repository Settings → Pages
2. Source: Deploy from a branch
3. Branch: `gh-pages` / `root`
4. Save

The GitHub Actions workflow will automatically deploy on push.

### 4. Custom Domain Setup

To use `docs.pgauthz.dev`:

1. Purchase domain `pgauthz.dev`
2. Add DNS CNAME record:
   ```
   docs.pgauthz.dev → your-org.github.io
   ```
3. In GitHub repository Settings → Pages:
   - Custom domain: `docs.pgauthz.dev`
   - Enforce HTTPS: ✓

### 5. Update Placeholders

Replace these placeholders throughout the documentation:
- `your-org` → Your GitHub organization name
- `G-XXXXXXXXXX` → Your Google Analytics ID (in mkdocs.yml)
- Download URLs → Actual release URLs once available

### 6. Create First Release

Tag a release to create the first versioned documentation:
```bash
git tag -a v1.0.0 -m "Initial release"
git push origin v1.0.0
```

This will trigger the deployment workflow and create versioned docs at:
- `https://docs.pgauthz.dev/git/a1b2c3d4/` (hash-based)
- `https://docs.pgauthz.dev/v1.0.0/` (version-based)
- `https://docs.pgauthz.dev/latest/` (latest)

## URL Structure

Once deployed, documentation will be available at:

### User-Friendly URLs
- `https://docs.pgauthz.dev/` - Latest documentation
- `https://docs.pgauthz.dev/v1.0.0/` - Specific version
- `https://docs.pgauthz.dev/latest/` - Latest version
- `https://docs.pgauthz.dev/stable/` - Stable version

### Git Hash URLs (Actual Serving)
- `https://docs.pgauthz.dev/git/a1b2c3d4/` - Specific commit

### API Endpoints (for Agents)
- `/api/docs/{version}/{file}` - Get documentation file
- `/api/skills/{version}/skills-manifest.json` - Get skills manifest
- `/api/versions` - List all versions

## Key Features Implemented

✅ User-focused documentation (no low-level Rust/cargo details)  
✅ Complete API reference with all SQL functions  
✅ Policy viewing APIs documented (`pgauthz_read_model`, `pgauthz_read_latest_model`, `pgauthz_list_models`)  
✅ Relation reading API documented (`pgauthz_read_tuples`)  
✅ Git hash-based versioning (8-char short hashes)  
✅ 404 fallback strategy (no automatic fallback)  
✅ Permanent storage (all hashes kept forever)  
✅ Skills repository structure  
✅ Agent-specific documentation and APIs  
✅ MkDocs with Material theme  
✅ Automated GitHub Actions deployment  
✅ Multi-language examples (Python, Node.js, Go)  
✅ Comprehensive troubleshooting guides  

## Documentation Standards

All documentation follows these principles:
- **User-focused** - Written for end users, not developers
- **Example-driven** - Real-world code examples in multiple languages
- **Searchable** - Full-text search enabled
- **Versioned** - Each version has complete documentation snapshot
- **Agent-friendly** - Machine-readable formats available

## Maintenance

### Adding New Documentation

1. Create/edit markdown files in `docs/`
2. Update `mkdocs.yml` navigation if needed
3. Commit and push to main branch
4. GitHub Actions will automatically deploy

### Updating Skills

1. Edit `skills/skills-manifest.json`
2. Add skill files to `skills/skills/`
3. Commit and push
4. Skills will be available at next deployment

### Creating New Version

1. Tag a new release: `git tag -a v1.1.0 -m "Release 1.1.0"`
2. Push tag: `git push origin v1.1.0`
3. GitHub Actions will create versioned documentation
4. Version map will be automatically updated

## Support & Questions

For questions about the documentation system:
1. Check this guide first
2. Review the plan at `.windsurf/plans/pgauthz-documentation-system-e60f11.md`
3. Open an issue on GitHub

## What's Not Included (To Be Added Later)

These items were marked as "not necessary for initial documentation":
- BENCHMARKS.md content (can be added later)
- Build from source details (moved to separate developer docs)
- Multi-tenant examples (system is tenant-agnostic)

## Success Criteria Met

✅ Complete documentation covering all pgauthz features  
✅ Git hash-based versioning with permanent storage  
✅ Skills repository with downloadable patterns  
✅ Agent-specific documentation and APIs  
✅ MkDocs deployment to GitHub Pages  
✅ Automated deployment on git push/tag  
✅ Search functionality  
✅ Mobile-responsive design  
✅ Ready for custom domain (docs.pgauthz.dev)  

The pgauthz documentation system is now complete and ready for deployment! 🎉
