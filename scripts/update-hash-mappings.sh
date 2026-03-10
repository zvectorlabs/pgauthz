#!/bin/bash
# Update version mappings with git hash information

set -e

GIT_HASH=$1
GIT_REF=$2

if [ -z "$GIT_HASH" ] || [ -z "$GIT_REF" ]; then
    echo "Usage: $0 <git_hash> <git_ref>"
    exit 1
fi

# Get short hash (8 characters)
GIT_SHORT=${GIT_HASH:0:8}

# Get commit info
COMMIT_DATE=$(git show -s --format=%cI "$GIT_HASH" 2>/dev/null || date -u +"%Y-%m-%dT%H:%M:%SZ")
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "main")
TAG=$(git describe --tags --exact-match "$GIT_HASH" 2>/dev/null || echo "")

# Determine status
if [ -n "$TAG" ]; then
    STATUS="stable"
else
    STATUS="development"
fi

# Update version-map.json
jq --arg hash "$GIT_SHORT" \
   --arg full_hash "$GIT_HASH" \
   --arg commit_date "$COMMIT_DATE" \
   --arg branch "$BRANCH" \
   --arg tag "$TAG" \
   --arg status "$STATUS" \
   '
   .hash_metadata[$hash] = {
     "full_hash": $full_hash,
     "commit_date": $commit_date,
     "branch": $branch,
     "tag": $tag,
     "status": $status,
     "documentation": {
       "url": "https://docs.pgauthz.dev/git/" + $hash,
       "path": "/git/" + $hash
     },
     "skills": {
       "url": "https://skills.pgauthz.dev/git/" + $hash,
       "manifest": "https://skills.pgauthz.dev/git/" + $hash + "/skills-manifest.json"
     },
     "agents": {
       "url": "https://agents.pgauthz.dev/git/" + $hash
     }
   } |
   if $tag != "" then 
     .mappings[$tag] = $hash |
     .mappings["latest"] = $hash |
     .mappings["stable"] = $hash
   elif $branch == "main" then
     .mappings["latest"] = $hash |
     .mappings["main"] = $hash
   else
     .mappings[$branch] = $hash
   end |
   .generated_at = (now | strftime("%Y-%m-%dT%H:%M:%SZ"))
   ' \
   versions/version-map.json > versions/version-map.tmp.json && \
mv versions/version-map.tmp.json versions/version-map.json

echo "Updated version map with hash $GIT_SHORT"
cat versions/version-map.json
