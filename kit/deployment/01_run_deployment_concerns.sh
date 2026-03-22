#!/usr/bin/env bash

# Basic deployment concerns for Node.js applications

# Check for .nvmrc and install the specified Node.js version
if [ -f "./.nvmrc" ]; then
	if [ -d "$HOME/.config/nvm" ]; then
		source "$HOME/.config/nvm/nvm.sh"
	fi
	nvm install
fi	

# Refresh node modules
rm -rf node_modules

# Check for yarn.lock or package-lock.json and install dependencies accordingly
if [ -f "./yarn.lock" ]; then
	npm install -g yarn
	yarn
	yarn build
fi

if [ -f "./package-lock.json" ]; then
	npm install
	npm run build
fi