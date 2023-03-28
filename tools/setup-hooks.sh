#!/bin/bash
# This script sets up the recommended git hooks.

if [ ! -d ".git" ]; then
  echo "No \".git\" folder found. Please run this from the repository root, like: "
  echo "  tools/setup-hooks.sh"
  exit
fi

echo "Setting up Git hooks..."

# This option prints the commands when they're invoked which enables
# transparency, even when the script source isn't directly read.
set -o xtrace

echo -e "Creating pre-push hook..."
cp -i "./tools/git-hooks/pre-push" "./.git/hooks/pre-push"

set +o xtrace
echo "Done!"
