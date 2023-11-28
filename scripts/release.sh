#!/bin/bash
VERSION=$(git show main:Cargo.toml | grep version | head -1 | sed -nre 's/^[^0-9]*(([0-9]+\.)*[0-9]+).*/\1/p')
echo $VERSION
git tag -a $VERSION -m "Release $VERSION"
git push origin $VERSION