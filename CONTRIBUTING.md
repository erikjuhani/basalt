Basalt is **open for code contributions**, primarily for bug fixes. Feature work can bring long-term maintenance overhead, and I'd like to keep that to a minimum. One reason for limiting feature work is that I want to build features myself, as this is a fun side project—and I would like to keep it that way.

That said, open source projects flourish with multiple contributors. I won't say no to feature work, but please open an issue first so we can discuss it. This avoids unnecessary effort or bikeshedding over architectural or stylistic choices.

> [!INFO]
>
> I want this project to feel low-barrier, so don't be discouraged from opening an issue, whether it's about existing features, ideas, or anything else!

## What you can do right now

### Found a typo?

Open a PR directly with the correction!

### Found a bug and know how to fix it?

Open a PR with the fix!

### Found a bug but not sure how to fix it or don't want to do it yourself?

Open an issue with steps to reproduce!

### Want to contribute a feature?

Open an issue first so we can chat about the feature work or claim an existing issue for yourself!

## How to make your contribution

1. Fork the `basalt` repository
2. Create a branch
3. Open a pull request against basalt's main branch with your changes
4. I'll review your pull request as soon as possible and either leave comments or merge it

If you find mistakes in the documentation or need simple code fixes, please go ahead and open a pull request with the changes!

## Development

### Setup

Clone the repository and build:

```sh
git clone https://github.com/erikjuhani/basalt.git
cd basalt
cargo build
```

The project uses Rust 1.92.0 (specified in `rust-toolchain.toml`). Rustup will automatically use the correct version.

### Running Locally

```sh
cargo run
```

### Code Style

Format your code before committing:

```sh
make fmt
```

### Running Checks

Run the full CI check suite locally:

```sh
make check
```

This runs formatting checks, clippy, tests, and build verification.

### Git Pre-push Hook

There's a useful pre-push git hook under `scripts`, which you can enable by running the following command:

```sh
cp scripts/pre-push .git/hooks/
```

The script runs the same test commands as in the `test.yml` workflow.

### Pinact

To pin GitHub Action dependencies, you can use [pinact](https://github.com/suzuki-shunsuke/pinact).

Install it using [`shm`](https://github.com/erikjuhani/shm?tab=readme-ov-file#installation):

```sh
shm get --bin suzuki-shunsuke/pinact
```

## Changelog Trailers

When making changes that should appear in the changelog, add a `Changelog:` trailer to your commit message. This helps automatically categorize your changes when generating release notes.

Available changelog trailers:

- `Changelog: breaking` - Breaking changes
- `Changelog: added` - New features
- `Changelog: changed` - Changes to existing functionality
- `Changelog: deprecated` - Soon-to-be removed features
- `Changelog: removed` - Removed features
- `Changelog: fixed` - Bug fixes
- `Changelog: security` - Security-related changes
- `Changelog: dependencies` - Dependency updates

Example commit message:

```gitcommit
Add support for custom keybindings

This commit introduces the ability to define custom keybindings
in the configuration file.

Changelog: added
```

If your change doesn't need to appear in the changelog (typo fixes, internal refactoring, etc.), simply omit the `Changelog:` trailer.

## CI

Pull requests and pushes to `main` trigger the following CI workflows:

### Test Workflow (`test.yml`)

The primary CI workflow runs on every pull request and push to main:

| Step | Description |
|------|-------------|
| **Format check** | Verifies code formatting with `make fmt-check` |
| **Cargo check** | Validates the code compiles without errors |
| **Clippy** | Runs the Rust linter with warnings treated as errors |
| **Tests** | Executes the full test suite across the workspace |
| **Build & Package** | Builds the binary and verifies it can be packaged |

### Workflow Security (`workflow-security.yml`)

A security-focused workflow that checks the GitHub Actions configuration:

- **actionlint**: Validates workflow syntax and best practices
- **pinact**: Ensures action versions are pinned to specific commits
- **zizmor**: Scans for security issues in workflow definitions

### Pull Requests from Forks

For security reasons, GitHub requires maintainer approval before workflows run on pull requests from first-time contributors. If your PR shows "Waiting for approval", a maintainer will review and approve the workflow run.

Once you've had a PR approved and merged, future PRs will run workflows automatically.

## Creating a Release

> [!NOTE]
>
> This section is primarily for maintainers, but it's documented here for transparency and in case contributors are curious about the release process.

The release process involves the following steps:

### 1. Generate the Changelog

Before creating a release tag, generate the changelog for each crate using the `make changelog` target:

```sh
make changelog crate=basalt version=X.Y.Z
make changelog crate=basalt-core version=X.Y.Z
make changelog crate=basalt-widgets version=X.Y.Z
```

The generated changelogs will be grouped by category (Breaking, Added, Changed, Fixed, etc.) and include links to commits and PRs. If a crate has no commits with `Changelog:` trailers since the last release, the changelog file will remain unchanged (no empty version sections will be added).

### 2. Commit the Changelog Updates

Commit only the changelogs that were actually modified:

```sh
git add basalt/CHANGELOG.md basalt-core/CHANGELOG.md basalt-widgets/CHANGELOG.md
git commit -m "Update changelogs for vX.Y.Z"
```

### 3. Create and Push the Release Tag

```sh
git tag basalt/vX.Y.Z
git push origin basalt/vX.Y.Z
```

The tag must follow the pattern `basalt/vX.Y.Z` (e.g., `basalt/v0.4.0`) to trigger the release workflow.

### 4. Create the GitHub Release

The GitHub Actions workflow will automatically build binaries for multiple platforms. Once the artifacts are uploaded, create a GitHub release manually through the GitHub UI and copy the relevant section from the generated CHANGELOG.md.

