# pgauthz Skills Repository

Downloadable skills and patterns for common pgauthz use cases.

## Overview

The skills repository provides pre-built patterns, code examples, and integration guides for common authorization scenarios. Each skill is a self-contained package with documentation, examples, and tests.

## Available Skills

### Authorization Check Pattern
Learn how to implement authorization checks in your application.
- **Skill ID**: `authorization-check`
- **Download**: [authorization-check.json](skills/authorization-check/skill.json)

### Policy Validation
Validate authorization policies before deployment.
- **Skill ID**: `policy-validation`
- **Download**: [policy-validation.json](skills/policy-validation/skill.json)

### Cache Optimization
Optimize cache configuration for your workload.
- **Skill ID**: `cache-optimization`
- **Download**: [cache-optimization.json](skills/cache-optimization/skill.json)

### Monitoring Setup
Set up OpenTelemetry monitoring and alerts.
- **Skill ID**: `monitoring-setup`
- **Download**: [monitoring-setup.json](skills/monitoring-setup/skill.json)

### Policy Evolution
Learn how to evolve authorization policies as your business needs change.
- **Skill ID**: `policy-evolution`
- **Download**: [policy-evolution.json](skills/policy-evolution/skill.json)
- **Topics**: Versioning, migration strategies, adding types/relations/permissions

## Using Skills

### Download a Skill

```bash
# Download skill manifest
curl -O https://skills.pgauthz.dev/authorization-check/skill.json

# Or use wget
wget https://skills.pgauthz.dev/authorization-check/skill.json
```

### Install Dependencies

Each skill lists its dependencies in the manifest:

```json
{
  "dependencies": ["pgauthz-core", "postgresql-16"]
}
```

### Follow the Guide

Each skill includes:
- **skill.md** - Step-by-step guide
- **examples/** - Code examples in multiple languages
- **tests/** - Test cases to verify implementation

## Skill Structure

Each skill follows this structure:

```
skills/authorization-check/
├── skill.json          # Skill metadata
├── skill.md            # Documentation
├── examples/           # Code examples
│   ├── python/
│   ├── nodejs/
│   └── go/
└── tests/              # Test cases
    └── test_cases.sql
```

## Creating Custom Skills

You can create your own skills following the same structure. See [Cloud Integration Guide](cloud-integration.md) for deploying custom skills.

## Cloud Integration

Deploy skills to cloud environments:
- [AWS Deployment](cloud-integration.md#aws)
- [Google Cloud Deployment](cloud-integration.md#gcp)
- [Azure Deployment](cloud-integration.md#azure)

## Skill Manifest Format

```json
{
  "id": "skill-id",
  "name": "Skill Name",
  "version": "1.0.0",
  "description": "Skill description",
  "author": "Author Name",
  "license": "MIT",
  "dependencies": ["dependency1", "dependency2"],
  "tags": ["tag1", "tag2"],
  "download_url": "https://skills.pgauthz.dev/skill-id/skill.json",
  "documentation_url": "https://docs.pgauthz.dev/skills/skill-id",
  "repository_url": "https://github.com/your-org/pgauthz-skills"
}
```

## Contributing Skills

To contribute a new skill:
1. Fork the repository
2. Create your skill following the structure above
3. Test thoroughly
4. Submit a pull request

## Support

- **Documentation**: [docs.pgauthz.dev](https://docs.pgauthz.dev)
- **Issues**: [GitHub Issues](https://github.com/your-org/pgauthz/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/pgauthz/discussions)
