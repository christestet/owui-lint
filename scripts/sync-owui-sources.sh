#!/usr/bin/env bash
# Ground truth for the OWSEC security rules: the upstream Open WebUI source and
# docs that each rule is designed against. We pull them here so a rule is always
# validated against *the real implementation*, never a stale assumption.
#
# A pinned copy rots silently (we already caught the bundled plugin.py drifting
# from upstream main). So:
#   - Citations in rule docs reference upstream by SYMBOL (e.g. `exec(content,
#     module.__dict__)`, `module.Tools()`), never by line number.
#   - This script records provenance (URL + fetch date + sha256) in SOURCES.md.
#   - `--check` re-fetches and fails when upstream drifts, so we get an alarm to
#     re-verify the rules instead of trusting a frozen snapshot.
#
#   sync-owui-sources.sh            # fetch + refresh provenance (default)
#   sync-owui-sources.sh --write    # same as default
#   sync-owui-sources.sh --check    # fail if any local copy differs from upstream (CI)
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

dest_dir=".agents/openwebui-extensions/references"
manifest="$dest_dir/SOURCES.md"

owui_main="https://raw.githubusercontent.com/open-webui/open-webui/refs/heads/main"
docs_main="https://raw.githubusercontent.com/open-webui/docs/main"
pipelines_main="https://raw.githubusercontent.com/open-webui/pipelines/main"

# local_filename|upstream_url  — the files the OWSEC/structural rules are validated
# against. Add a source here the moment a rule starts depending on upstream behavior.
# (Delimiter is '|' because the URLs themselves contain ':'.)
#
# Two distinct loaders govern import-time execution and must both be tracked:
#   - plugin.py        : Tools/Pipe/Filter/Action via exec() + module.Tools()
#   - pipelines main.py: Pipeline via importlib exec_module() + module.Pipeline()
#
# The five source `.py` files are the upstream loaders/executors the OWSEC rules AND
# the openwebui-extensions skill depend on for exact behavior (signatures, reserved-arg
# injection, valve instantiation, class detection). They replace older hand-pinned
# copies (which carried stale `version: openwebui vX` headers) — version is now tracked
# centrally in SOURCES.md, not per file.
#
# Deliberately NOT pulled (revisit only when a rule's correctness hinges on their
# *code*, not their docs):
#   - utils/middleware.py (~5.3k lines): reserved-arg injection + event flow. Huge +
#     high-churn; covered well enough by events.mdx/valves.mdx. Drift = constant noise.
#   - models/*.py: DB models, no code-parsing relevance.
#   - frontend: the `execute` event's `new Function()` is Svelte/TS — events.mdx is the
#     citable source; there is no Python ground truth to pull.
mappings=(
  # source code (open-webui backend + pipelines)
  "plugin.py|$owui_main/backend/open_webui/utils/plugin.py"
  "tools.py|$owui_main/backend/open_webui/utils/tools.py"
  "filter.py|$owui_main/backend/open_webui/utils/filter.py"
  "actions.py|$owui_main/backend/open_webui/utils/actions.py"
  "events.py|$owui_main/backend/open_webui/events.py"
  "pipelines-main.py|$pipelines_main/main.py"
  # docs (raw upstream, ground truth for citations)
  "plugin-overview.mdx|$docs_main/docs/features/extensibility/plugin/index.mdx"
  "tools-development.mdx|$docs_main/docs/features/extensibility/plugin/tools/development.mdx"
  "events.mdx|$docs_main/docs/features/extensibility/plugin/development/events.mdx"
  "event-function.mdx|$docs_main/docs/features/extensibility/plugin/functions/event.mdx"
  "valves.mdx|$docs_main/docs/features/extensibility/plugin/development/valves.mdx"
)

mode="write"
case "${1:-}" in
  --check) mode="check" ;;
  --write | "") mode="write" ;;
  *)
    echo "usage: $0 [--write|--check]" >&2
    exit 2
    ;;
esac

sha() { shasum -a 256 "$1" | awk '{print $1}'; }

status=0
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

manifest_rows=()
for pair in "${mappings[@]}"; do
  name="${pair%%|*}"
  url="${pair#*|}"
  dst="$dest_dir/$name"
  fetched="$tmp/$name"

  if ! curl -fsSL "$url" -o "$fetched"; then
    echo "error: failed to fetch $url" >&2
    exit 1
  fi

  if [[ "$mode" == "check" ]]; then
    if ! cmp -s "$fetched" "$dst" 2>/dev/null; then
      echo "DRIFT: $name differs from upstream ($url)" >&2
      echo "       re-verify the OWSEC rules that cite it, then run: make owui-sources-sync" >&2
      status=1
    fi
  else
    mkdir -p "$dest_dir"
    if cmp -s "$fetched" "$dst" 2>/dev/null; then
      echo "up to date: $name"
    else
      cp "$fetched" "$dst"
      echo "synced: $name <- $url"
    fi
    manifest_rows+=("| \`$name\` | <$url> | \`$(sha "$dst")\` |")
  fi
done

if [[ "$mode" == "write" ]]; then
  owui_lint_version="$(grep -m1 '^version' "$repo_root/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"
  remote_sha() { git ls-remote "https://github.com/$1.git" main 2>/dev/null | awk 'NR==1{print substr($1,1,12)}'; }
  owui_release="$(curl -fsSL https://api.github.com/repos/open-webui/open-webui/releases/latest 2>/dev/null \
    | grep -oE '"tag_name"[[:space:]]*:[[:space:]]*"[^"]+"' | head -1 | sed -E 's/.*"([^"]+)"$/\1/' || true)"
  {
    echo "# Open WebUI upstream sources"
    echo "# (ground truth for owui-lint OWSEC rules + the openwebui-extensions skill)"
    echo
    echo "> Generated by \`scripts/sync-owui-sources.sh\` — do not edit by hand."
    echo "> Refresh: \`make owui-sources-sync\`  •  Detect drift: \`make owui-sources-check\`"
    echo
    echo "| Provenance | Value |"
    echo "|---|---|"
    echo "| Synced (UTC) | $(date -u +%Y-%m-%dT%H:%M:%SZ) |"
    echo "| owui-lint version | ${owui_lint_version:-unknown} |"
    echo "| open-webui | \`main@$(remote_sha open-webui/open-webui)\` (latest release ${owui_release:-n/a}) |"
    echo "| pipelines | \`main@$(remote_sha open-webui/pipelines)\` |"
    echo "| docs | \`main@$(remote_sha open-webui/docs)\` |"
    echo
    echo "> Cite these by symbol (function/identifier), never by line number — line"
    echo "> numbers drift across upstream releases."
    echo
    echo "| Local copy | Upstream URL | sha256 |"
    echo "|---|---|---|"
    printf '%s\n' "${manifest_rows[@]}"
  } > "$manifest"
  echo "wrote provenance: $manifest"
fi

if [[ "$mode" == "check" && "$status" -ne 0 ]]; then
  echo "upstream OWUI sources drifted; run 'make owui-sources-sync' and re-verify rules" >&2
fi

exit "$status"
