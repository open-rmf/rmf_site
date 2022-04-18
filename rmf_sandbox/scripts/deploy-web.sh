#!/bin/bash
set -o errexit

read -p "Do you really want to deploy to GitHub Pages? (y/N) "
echo  # send a carriage return
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Exiting..."
    exit 1
fi

echo "Starting deployment process..."

set -o verbose

git clone ssh://git@github.com/osrf/rmf_sandbox temp-deploy-checkout
cd temp-deploy-checkout/rmf_sandbox
git checkout --orphan gh-pages
git reset
scripts/build-web.sh

git add -f web
cd ..
cp rmf_sandbox/web/root_index.html index.html
git add index.html

git commit -a -m "publish to github pages"

git push origin gh-pages --force
