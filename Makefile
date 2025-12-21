.PHONY: fmt cargo-fmt json-fmt fmt-check cargo-fmt-check json-fmt-check vhs check changelog

vhs:
	./scripts/vhs

check:
	@$(MAKE) fmt-check
	@if command -v pinact >/dev/null 2>&1; then find .github/workflows -name '*.yml' -print0 | xargs -0 pinact run --check ; fi
	cargo check --locked --profile ci --workspace --all-targets
	cargo clippy --profile ci --workspace --all-targets -- -D warnings
	cargo test --profile ci --workspace --all-targets
	cargo build --profile ci --workspace --all-targets
	cargo package --no-verify --allow-dirty

fmt: cargo-fmt json-fmt

fmt-check: cargo-fmt-check json-fmt-check

cargo-fmt:
	cargo fmt --all

json-fmt:
	find . -name "*.json" -not -path "./target/*" -exec sh -c 'jq "." "$$1" > tmp && mv tmp "$$1"' _ {} \;

cargo-fmt-check:
	cargo fmt --all --check

json-fmt-check:
	find . -name "*.json" -not -path "./target/*" | xargs -I {} sh -c 'jq "." "{}" | diff --color=always -u0 "{}" -'

changelog:
	@if [ -z "$(crate)" ]; then echo "Error: crate parameter is required (e.g., CRATE=basalt-core)"; exit 1; fi
	@if [ -z "$(version)" ]; then echo "Error: version parameter is required (e.g., VERSION=0.1.0)"; exit 1; fi
	git-cliff -u --include-path "$(crate)/**" --tag "$(crate)/$(version)" --count-tags "$(crate)/v*" --prepend $(crate)/CHANGELOG.md
