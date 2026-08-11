# make make sane
.DELETE_ON_ERROR:
.SHELLFLAGS := --norc -euo pipefail -c
SHELL := /bin/bash

# if we hit one doing this stuff, we want it
export RUST_BACKTRACE ?= 1
# configuration to use for "perf"
export PERF_CONFIG ?= .perfconfig
# nightly toolchain used for sanitizers
export NIGHTLY_TOOLCHAIN ?= nightly
# target used for sanitizers, pgo, etc
export TARGET := $(shell rustc --print=host-tuple)
# location of internal tools
export SYSROOT := $(shell rustc --print sysroot)
# llvm tool used for coverage
export LLVM_COV ?= $(SYSROOT)/lib/rustlib/$(TARGET)/bin/llvm-cov
# llvm tool used for profile data
export LLVM_PROFDATA ?= $(SYSROOT)/lib/rustlib/$(TARGET)/bin/llvm-profdata
# directory used to gather profile data
export PROFILE_DIR := $(abspath target/pgo-data)
# raw filenames used for profiled output
export LLVM_PROFILE_FILE := ${PROFILE_DIR}/%m_%p.profraw

# extra output when running in CI
ifdef CI
PREK_FLAGS = --verbose --color always --no-progress
endif

.PHONY: pegon
build: ## Create binary
	cargo build --release

.PHONY: build-pgo
build-pgo: corpus pgo-generate pgo-train pgo-merge pgo-use ## Create profile-guided binary

.PHONY: build-wheel
build-wheel: ## Create python package
	uv build

.PHONY: lint
lint: test

.PHONY: test
test: ## Lint, format, test
	uv run --frozen --only-dev prek --all-files --stage pre-push ${PREK_FLAGS}

.PHONY: test-cov
test-cov: ## Run tests with coverage report
	cargo llvm-cov --text
	cargo llvm-cov report --summary-only

.PHONY: test-asan
test-asan: export CFLAGS=-fsanitize=address,undefined -O1
test-asan: export RUSTFLAGS=-Zsanitizer=address
test-asan: export CXXFLAGS=${CFLAGS}
test-asan: export RUSTDOCFLAGS=${RUSTFLAGS}
test-asan: export CARGO_PROFILE_SANITIZE_BUILD_OVERRIDE_RUSTFLAGS=-C linker=clang -Clink-arg=-fsanitize=address,undefined
test-asan:  ## Run tests with asan
	cargo +${NIGHTLY_TOOLCHAIN} test -Z profile-rustflags -Z build-std --profile sanitize --target ${TARGET}

.PHONY: test-tsan
test-tsan: export CFLAGS=-fsanitize=thread -O1
test-tsan: export RUSTFLAGS=-Zsanitizer=thread
test-tsan: export CXXFLAGS=${CFLAGS}
test-tsan: export RUSTDOCFLAGS=${RUSTFLAGS}
test-tsan: export CARGO_PROFILE_SANITIZE_BUILD_OVERRIDE_RUSTFLAGS=-C linker=clang -Clink-arg=-fsanitize=thread
test-tsan:  ## Run tests with tsan
	cargo +${NIGHTLY_TOOLCHAIN} test -Z profile-rustflags -Z build-std --profile sanitize --target ${TARGET}

.PHONY: corpus
corpus:
	rm -rf target/corpus
	mkdir -p target/corpus
	(cd target/corpus && git clone --depth 1 --single-branch --filter=blob:none https://github.com/apache/lucene.git --branch releases/lucene/10.2.2)

.PHONY: pgo-generate
pgo-generate:
	rm -rf ${PROFILE_DIR}
	RUSTFLAGS="-Cprofile-generate=${PROFILE_DIR}" cargo build --release --target ${TARGET}

.PHONY: pgo-train
pgo-train:
	target/${TARGET}/release/pegon check target/corpus > /dev/null || true

.PHONY: pgo-merge
pgo-merge:
	${LLVM_PROFDATA} merge -o ${PROFILE_DIR}/merged.profdata ${PROFILE_DIR}/*.profraw

.PHONY: pgo-use
pgo-use:
	RUSTFLAGS="-Cprofile-use=${PROFILE_DIR}/merged.profdata" cargo build --release --target=${TARGET}

.PHONY: profile
profile: ## Profile run with perf
	RUSTFLAGS="-C force-frame-pointers=yes" cargo build --profile profiling
	perf record -g target/profiling/pegon check ~/workspace/lucene > /dev/null || true
	perf report

.PHONY: profile-queries
profile-queries: ## Profile queries
	ts_query_ls profile

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
