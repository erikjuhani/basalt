.PHONY: fmt cargo-fmt json-fmt fmt-check cargo-fmt-check json-fmt-check vhs

vhs:
	./scripts/vhs

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
