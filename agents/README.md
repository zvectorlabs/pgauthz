# Agent Integration Documentation

Documentation for AI agents and automated systems integrating with pgauthz.

## Overview

pgauthz provides agent-friendly APIs and documentation formats for:
- Code completion agents
- Documentation Q&A bots
- Debugging assistants
- Monitoring agents

## Agent API

### Markdown Serving

Access documentation as JSON for programmatic consumption:

```bash
# Get documentation as JSON
curl https://docs.pgauthz.dev/api/docs/latest/installation.md

# Get specific version
curl https://docs.pgauthz.dev/api/docs/v1.0.0/api-reference.md
```

### Search API

Search documentation programmatically:

```bash
# Search for content
curl -X POST https://docs.pgauthz.dev/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "cache configuration", "version": "latest"}'
```

### Skills API

Access skills programmatically:

```bash
# Get skills manifest
curl https://skills.pgauthz.dev/latest/skills-manifest.json

# Get specific skill
curl https://skills.pgauthz.dev/latest/authorization-check/skill.json
```

## Agent Types

### Code Assistant Agents
Agents that help with code completion and generation.
- [Integration Guide](agent-types/code-assistant.md)

### Documentation Bots
Q&A bots that answer questions about pgauthz.
- [Integration Guide](agent-types/documentation-bot.md)

### Debugging Agents
Agents that help troubleshoot issues.
- [Integration Guide](agent-types/debugging-agent.md)

### Monitoring Agents
Agents that monitor and alert on pgauthz metrics.
- [Integration Guide](agent-types/monitoring-agent.md)

## API Endpoints

### Documentation Endpoints

```
GET /api/docs/{version}/{file}
  - Returns markdown file as JSON
  - version: "latest", "stable", or "v1.0.0"
  - file: path to documentation file

GET /api/versions
  - Returns list of available versions

GET /api/versions/{version}
  - Returns version metadata
```

### Skills Endpoints

```
GET /api/skills/{version}/skills-manifest.json
  - Returns skills manifest

GET /api/skills/{version}/{skill_id}/skill.json
  - Returns skill metadata

GET /api/skills/{version}/{skill_id}/{file}
  - Returns skill file
```

### Search Endpoints

```
POST /api/search
  - Body: {"query": "search term", "version": "latest"}
  - Returns: {"results": [...]}
```

## Response Formats

### Documentation Response

```json
{
  "version": "v1.0.0",
  "git_hash": "a1b2c3d4",
  "file_path": "installation.md",
  "content": "# Installation Guide\n...",
  "metadata": {
    "title": "Installation Guide",
    "last_updated": "2026-02-24T15:00:00Z"
  }
}
```

### Search Response

```json
{
  "query": "cache configuration",
  "version": "latest",
  "results": [
    {
      "file": "configuration.md",
      "title": "Configuration Guide",
      "excerpt": "...cache configuration...",
      "score": 0.95,
      "url": "https://docs.pgauthz.dev/latest/configuration"
    }
  ]
}
```

## Error Handling

All API endpoints return 404 if content is not found (no fallback to latest).

```json
{
  "error": "Content not found",
  "version": "v1.0.0",
  "file": "nonexistent.md",
  "message": "The requested file does not exist in this version"
}
```

## Rate Limiting

API endpoints are rate-limited:
- 100 requests per minute per IP (documentation)
- 1000 requests per minute per IP (search)

## Authentication

Currently no authentication required. Future versions may add API keys for higher rate limits.

## Examples

### Python Agent

```python
import requests

class PgAuthzAgent:
    def __init__(self, base_url="https://docs.pgauthz.dev"):
        self.base_url = base_url
    
    def get_documentation(self, file_path, version="latest"):
        """Get documentation file."""
        url = f"{self.base_url}/api/docs/{version}/{file_path}"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()
    
    def search(self, query, version="latest"):
        """Search documentation."""
        url = f"{self.base_url}/api/search"
        response = requests.post(url, json={
            "query": query,
            "version": version
        })
        response.raise_for_status()
        return response.json()
    
    def get_skill(self, skill_id, version="latest"):
        """Get skill metadata."""
        url = f"https://skills.pgauthz.dev/{version}/{skill_id}/skill.json"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()

# Usage
agent = PgAuthzAgent()

# Get documentation
doc = agent.get_documentation("installation.md")
print(doc["content"])

# Search
results = agent.search("cache configuration")
for result in results["results"]:
    print(f"{result['title']}: {result['excerpt']}")

# Get skill
skill = agent.get_skill("authorization-check")
print(f"Skill: {skill['name']}")
```

### Node.js Agent

```javascript
class PgAuthzAgent {
  constructor(baseUrl = 'https://docs.pgauthz.dev') {
    this.baseUrl = baseUrl;
  }
  
  async getDocumentation(filePath, version = 'latest') {
    const url = `${this.baseUrl}/api/docs/${version}/${filePath}`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  }
  
  async search(query, version = 'latest') {
    const url = `${this.baseUrl}/api/search`;
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query, version })
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  }
  
  async getSkill(skillId, version = 'latest') {
    const url = `https://skills.pgauthz.dev/${version}/${skillId}/skill.json`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  }
}

// Usage
const agent = new PgAuthzAgent();

// Get documentation
const doc = await agent.getDocumentation('installation.md');
console.log(doc.content);

// Search
const results = await agent.search('cache configuration');
results.results.forEach(result => {
  console.log(`${result.title}: ${result.excerpt}`);
});
```

## Best Practices

1. **Cache responses** - Documentation doesn't change frequently
2. **Handle 404s gracefully** - Content may not exist in all versions
3. **Respect rate limits** - Implement backoff strategies
4. **Use specific versions** - Pin to versions for reproducibility
5. **Parse markdown** - Use markdown parsers for content extraction

## Support

- **Documentation**: [docs.pgauthz.dev](https://docs.pgauthz.dev)
- **API Issues**: [GitHub Issues](https://github.com/your-org/pgauthz/issues)
- **Agent Integration Help**: [GitHub Discussions](https://github.com/your-org/pgauthz/discussions)
