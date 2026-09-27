.PHONY: fmt cargo-fmt json-fmt fmt-check cargo-fmt-check json-fmt-check gifs gifs-dark gifs-light avifs avifs-dark avifs-light release-build check changelog

DARK_TAPES := $(wildcard tapes/dark/*.tape)
DARK_GIFS := $(DARK_TAPES:tapes/dark/%.tape=assets/dark/%.gif)
DARK_AVIFS := $(DARK_TAPES:tapes/dark/%.tape=assets/dark/%.avif)

LIGHT_TAPES := $(wildcard tapes/light/*.tape)
LIGHT_GIFS := $(LIGHT_TAPES:tapes/light/%.tape=assets/light/%.gif)
LIGHT_AVIFS := $(LIGHT_TAPES:tapes/light/%.tape=assets/light/%.avif)

AVIF_QUALITY ?= 50
AVIF_SPEED ?= 6
AVIF_FPS ?= 25
AVIF_SOURCE_FPS ?= 50

gifs: gifs-dark gifs-light

gifs-dark: release-build $(DARK_GIFS)

gifs-light: release-build $(LIGHT_GIFS)

avifs: avifs-dark avifs-light

avifs-dark: release-build $(DARK_AVIFS)

avifs-light: release-build $(LIGHT_AVIFS)

release-build:
	cargo build --release -q

assets/dark/%.gif: tapes/dark/%.tape
	@mkdir -p assets/dark
	vhs $<

assets/light/%.gif: tapes/light/%.tape
	@mkdir -p assets/light
	vhs $<

define record-avif
	@command -v avifenc >/dev/null || { echo "avifenc not found. Install it with: brew install libavif"; exit 1; }
	@mkdir -p $(dir $@)
	@tmp=$$(mktemp -d); mkdir -p $$tmp/comp; \
	width=$$(awk '/^Set Width/ {print $$3}' $<); \
	height=$$(awk '/^Set Height/ {print $$3}' $<); \
	pad=$$(awk '/^Set Padding/ {print $$3}' $<); \
	theme=$$(awk '/^Source/ {print $$2; exit}' $<); \
	bg=$$(sed -n 's/.*"background": *"\([^"]*\)".*/\1/p' $$theme | head -1); \
	sed 's|^Output .*|Output "'"$$tmp"'/frames/"|' $< > $$tmp/record.tape; \
	vhs $$tmp/record.tape >/dev/null && \
	ffmpeg -v error -y \
		-r $(AVIF_SOURCE_FPS) -start_number 1 -i $$tmp/frames/frame-text-%05d.png \
		-r $(AVIF_SOURCE_FPS) -start_number 1 -i $$tmp/frames/frame-cursor-%05d.png \
		-filter_complex "[0][1]overlay[merged];\
			[merged]scale=$$((width - 2 * pad)):$$((height - 2 * pad)):force_original_aspect_ratio=1[scaled];\
			[scaled]fps=$(AVIF_FPS),setpts=PTS/1[speed];\
			[speed]pad=$$width:$$height:(ow-iw)/2:(oh-ih)/2:$$bg[padded];\
			[padded]fillborders=left=$$pad:right=$$pad:top=$$pad:bottom=$$pad:mode=fixed:color=$$bg[out]" \
		-map "[out]" -pix_fmt rgb24 $$tmp/comp/f-%05d.png && \
	avifenc --fps $(AVIF_FPS) --keyframe 0 --jobs all --yuv 444 --qcolor $(AVIF_QUALITY) \
		--speed $(AVIF_SPEED) -a tune-content=screen $$tmp/comp/*.png $@ >/dev/null; \
	status=$$?; rm -rf $$tmp; exit $$status
endef

assets/dark/%.avif: tapes/dark/%.tape
	$(record-avif)

assets/light/%.avif: tapes/light/%.tape
	$(record-avif)

check:
	@$(MAKE) fmt-check
	@if command -v pinact >/dev/null 2>&1; then find .github/workflows -name '*.yml' -print0 | xargs -0 pinact run --check ; fi
	cargo check --locked --profile ci --workspace --all-targets
	cargo clippy --profile ci --workspace --all-targets -- -D warnings
	cargo nextest run --cargo-profile ci --workspace --all-targets
	cargo test --profile ci --workspace --doc
	cargo build --profile ci --workspace --all-targets
	cargo package --no-verify --allow-dirty

fmt: cargo-fmt json-fmt

fmt-check: cargo-fmt-check json-fmt-check

cargo-fmt:
	cargo fmt --all

json-fmt:
	find . -name "*.json" -not -path "./target/*" -not -path "*/.*/*" -exec sh -c 'jq "." "$$1" > tmp && mv tmp "$$1"' _ {} \;

cargo-fmt-check:
	cargo fmt --all --check

json-fmt-check:
	find . -name "*.json" -not -path "./target/*" -not -path "*/.*/*" | xargs -I {} sh -c 'jq "." "{}" | diff --color=always -u0 "{}" -'

changelog:
	@if [ -z "$(crate)" ]; then echo "Error: crate parameter is required (e.g., CRATE=basalt-core)"; exit 1; fi
	@if [ -z "$(version)" ]; then echo "Error: version parameter is required (e.g., VERSION=0.1.0)"; exit 1; fi
	GITHUB_TOKEN=$${GITHUB_TOKEN:-$$(gh auth token)} git-cliff -u --include-path "$(crate)/**" --tag-pattern "$(crate)/v.*" --tag "$(crate)/$(version)" --count-tags "$(crate)/v*" --prepend $(crate)/CHANGELOG.md
