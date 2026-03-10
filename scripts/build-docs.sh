#!/bin/bash
# Build documentation with MkDocs

set -e

# Get git information
GIT_HASH=${GIT_HASH:-$(git rev-parse HEAD)}
GIT_SHORT=${GIT_SHORT:-${GIT_HASH:0:8}}
RELEASE_DATE=${RELEASE_DATE:-$(date -u +"%Y-%m-%dT%H:%M:%SZ")}

echo "Building documentation..."
echo "Git Hash: $GIT_HASH"
echo "Short Hash: $GIT_SHORT"
echo "Release Date: $RELEASE_DATE"

# Install dependencies if needed
if ! command -v mkdocs &> /dev/null; then
    echo "Installing MkDocs..."
    pip install mkdocs mkdocs-material mkdocs-git-revision-date-localized-plugin
fi

# Build main documentation
echo "Building main documentation..."
mkdocs build -f mkdocs.yml -d site/git/$GIT_SHORT

# Copy source files for API access
echo "Copying source files..."
mkdir -p site/git/$GIT_SHORT/source
cp -r docs/ site/git/$GIT_SHORT/source/docs/
cp -r skills/ site/git/$GIT_SHORT/source/skills/
cp -r agents/ site/git/$GIT_SHORT/source/agents/

# Copy version map
echo "Copying version map..."
cp versions/version-map.json site/versions/

echo "Build complete!"
echo "Output directory: site/git/$GIT_SHORT"
