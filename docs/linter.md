# Linter

![CLI screenshot](https://github.com/user-attachments/assets/674bab2d-f713-4bdd-aa79-9a711ad1e061)

## General

The linter implements some checks from the [Google Java Style Guide](https://google.github.io/styleguide/javaguide.html).

It currently does NOT check formatting but it does check imports order,
naming conventions, program structure, etc.

## CLI

You can check your repository with the CLI:

```sh
uvx pegon check
```

### Output format

Load all problems into vim's quickfix list:

```sh
uvx pegon check --output-format concise | vim -q -
```

## Github Actions

You can use `setup-uv` to run in CI and cache the binary.

Example:

```yaml
jobs:
  pegon:
    env:
      # renovate: datasource=pypi depName=pegon
      PEGON_VERSION: "1.0.3"
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v7

      - name: Install uv
        uses: astral-sh/setup-uv@v8.3.2
        with:
          enable-cache: true
          prune-cache: false
          save-cache: ${{ github.event_name != 'pull_request' }}
          cache-suffix: ${{ env.PEGON_VERSION }}

      - name: Run pegon
        run: uvx "pegon@${PEGON_VERSION}" check
```

If you prefer to manage dependencies with dependabot,
or if you care about security at all in general,
you can instead add proper `pyproject.toml` file and have
`uv.lock` pinned/hashed dependencies:

```sh
uv init --bare
uv add --dev pegon
```

Then you can use `uv run pegon check` to run pinned version.

## Pre-Commit

To use with prek or pre-commit, use the `pypi` package with a local hook.
You don't want users compiling rust or anything.

Example:

```yaml
- repo: local
  hooks:
    - id: pegon
      name: Check Java style
      require_serial: true
      language: python
      additional_dependencies: ["pegon"]
      entry: pegon
      args: ['check']
      types: ['java']
```

Alternatively, consider adding to a `pyproject.toml` and using a `system`
hook that invokes `uv run`, same as described above for github actions.
