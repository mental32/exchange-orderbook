#!/bin/bash

set -e

# Clean up previous generation (except the OpenAPI spec and ignore file)
rm -v src/apis/* src/models/* src/lib.rs Cargo.toml .travis.yml git_push.sh README.md .gitignore 2>/dev/null
rm -v .openapi-generator/FILES .openapi-generator/VERSION 2>/dev/null
rmdir docs src/apis src/models src .openapi-generator 2>/dev/null

rm -v .openapi-generator-ignore

# Generate JWKS-only client
openapi-generator generate \
  -i clerk-backend-api.yaml \
  -g rust \
  --library reqwest-trait \
  -o . \
  --additional-properties packageName=clerk-client,packageVersion=0.1.0,topLevelApiClient=true

# Build to ensure everything is fine
cargo check
