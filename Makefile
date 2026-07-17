# make make sane
.DELETE_ON_ERROR:
.SHELLFLAGS := --norc -euo pipefail -c
SHELL := /bin/bash

.PHONY: pegon
pegon: ## Create binary
	cargo build --release

.PHONY: wheel
wheel: ## Create python package
	uv build

.PHONY: lint
lint: ## Lint, format, test
	uv run --frozen --only-dev prek --all-files --stage pre-push

.PHONY: bench
bench: ## Run micro-benchmarks
	# run benchmark suite
	cargo bench

.PHONY: profile-queries
profile-queries: ## Profile queries
	ts_query_ls profile

export PERF_CONFIG ?= .perfconfig

.PHONY: profile
profile: ## Profile lint run with perf
	RUSTFLAGS="-C force-frame-pointers=yes" cargo build --profile profiling
	perf record -g target/profiling/pegon check ~/workspace/lucene > out.txt || true
	perf report

export LLVM_COV ?= llvm-cov
export LLVM_PROFDATA ?= llvm-profdata

.PHONY: test
test: ## Run tests with coverage report
	cargo llvm-cov --text
	cargo llvm-cov report --summary-only

version: ## Bump version to VERSION
	# check that VERSION is set
	test -n "${VERSION}"
	# bump toml files
	uvx --from toml-cli toml set --toml-path Cargo.toml package.version ${VERSION}
	uvx --from toml-cli toml set --toml-path pyproject.toml project.version ${VERSION}
	npm version ${VERSION} --no-git-tag-version
	# regenerate lock files
	cargo update pegon
	uv lock -P pegon

VSCE := npx @vscode/vsce package --target

.PHONY: vscode-packages
.NOTPARALLEL: vscode-packages
vscode-packages: win-x64 win-arm64 linux-x64 linux-arm64 alpine-x64 alpine-arm64 darwin-x64 darwin-arm64

win-%:
	rm -rf bin && mkdir bin
	unzip -p wheels-windows-*/*_$(subst x64,amd64,$*).whl "*/pegon.exe" > bin/pegon.exe
	chmod +x bin/pegon.exe
	mkdir -p dist
	$(VSCE) win32-$* -o dist/win32-$*.vsix

linux-%:
	rm -rf bin && mkdir bin
	unzip -p wheels-linux-*/*_$(subst x64,x86_64,$(subst arm64,aarch64,$*)).whl "*/pegon" > bin/pegon
	chmod +x bin/pegon
	mkdir -p dist
	$(VSCE) $@ -o dist/$@.vsix

alpine-%:
	rm -rf bin && mkdir bin
	unzip -p wheels-musllinux-*/*_$(subst x64,x86_64,$(subst arm64,aarch64,$*)).whl "*/pegon" > bin/pegon
	chmod +x bin/pegon
	mkdir -p dist
	$(VSCE) $@ -o dist/$@.vsix

darwin-%:
	rm -rf bin && mkdir bin
	unzip -p wheels-macos-*/*_$(subst x64,x86_64,$*).whl "*/pegon" > bin/pegon
	chmod +x bin/pegon
	mkdir -p dist
	$(VSCE) $@ -o dist/$@.vsix

.PHONY: help
help: ## Display this help screen
	@grep -E '^[a-z.A-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
