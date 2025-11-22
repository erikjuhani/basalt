.PHONY: fmt cargo-fmt json-fmt fmt-check cargo-fmt-check json-fmt-check vhs check

HAS_PINACT := $(shell command -v pinact >/dev/null 2>&1 && echo true || echo false)

vhs:
	./scripts/vhs

check:
	@$(MAKE) fmt-check
ifeq ($(HAS_PINACT), true)
	@find .github/workflows -name '*.yml' -exec pinact run --check {} \;
endif
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
