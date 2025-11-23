#!/bin/bash

# Desktop App Release Script
# This script bumps the version, creates a git tag, and pushes it to trigger the build workflow
# Usage: scripts/release-desktop-app.sh [version]
# If no version is provided, it will bump the patch version automatically

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the project root directory (parent of scripts directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Change to project root
cd "$PROJECT_ROOT"

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
fi

# Check if we're on the main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo -e "${RED}Error: You must be on the main branch to create a release${NC}"
    echo "Current branch: $CURRENT_BRANCH"
    exit 1
fi

# Check if there are uncommitted changes (ignore untracked files)
if [ -n "$(git diff --name-only)" ] || [ -n "$(git diff --cached --name-only)" ]; then
    echo -e "${RED}Error: You have uncommitted changes. Please commit or stash your changes.${NC}"
    git status --short
    exit 1
fi

# Pull latest changes
echo -e "${YELLOW}Pulling latest changes from origin/main...${NC}"
git pull origin main

# Path to Cargo.toml (relative to project root)
CARGO_TOML="desktop-app/Cargo.toml"

if [ ! -f "$CARGO_TOML" ]; then
    echo -e "${RED}Error: $CARGO_TOML not found${NC}"
    exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')
echo -e "Current version: ${GREEN}$CURRENT_VERSION${NC}"

# Determine new version
if [ -n "$1" ]; then
    # Version override provided as argument
    NEW_VERSION="$1"
    echo -e "Using provided version: ${GREEN}$NEW_VERSION${NC}"
else
    # Auto-bump patch version
    IFS='.' read -r -a VERSION_PARTS <<< "$CURRENT_VERSION"
    MAJOR="${VERSION_PARTS[0]}"
    MINOR="${VERSION_PARTS[1]}"
    PATCH="${VERSION_PARTS[2]}"

    NEW_PATCH=$((PATCH + 1))
    NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"
    echo -e "Auto-bumping patch version to: ${GREEN}$NEW_VERSION${NC}"
fi

# Validate version format (basic check)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format. Use semantic versioning (e.g., 1.2.3)${NC}"
    exit 1
fi

# Check if tag already exists
TAG_NAME="desktop-app-v$NEW_VERSION"
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
    echo -e "${RED}Error: Tag $TAG_NAME already exists${NC}"
    exit 1
fi

# Update version in Cargo.toml
echo -e "${YELLOW}Updating version in $CARGO_TOML...${NC}"
sed -i "0,/^version = .*/s//version = \"$NEW_VERSION\"/" "$CARGO_TOML"

# Update Cargo.lock
echo -e "${YELLOW}Updating Cargo.lock...${NC}"
cd desktop-app
cargo check --quiet
cd ..

# Commit version bump
echo -e "${YELLOW}Committing version bump...${NC}"
git add "$CARGO_TOML" "Cargo.lock"
git commit -m "chore(desktop-app): bump version to $NEW_VERSION [skip ci]"

# Create and push tag
echo -e "${YELLOW}Creating tag $TAG_NAME...${NC}"
git tag -a "$TAG_NAME" -m "Release desktop-app v$NEW_VERSION"

# Push commits and tags
echo -e "${YELLOW}Pushing to origin...${NC}"
git push origin main
git push origin "$TAG_NAME"

echo -e "${GREEN}✓ Successfully created release $TAG_NAME${NC}"
echo -e "${GREEN}✓ GitHub Actions will now build the release${NC}"
echo -e "\nMonitor the build at: https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')/actions"
