#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/new-rule.sh <RULE_ID> <error|warning> "<Title>" [help_url] [openwebui_version]

Example:
  scripts/new-rule.sh OWC600 warning "Missing cache timeout"
  scripts/new-rule.sh OWC601 error "Invalid config contract" https://example.com/docs

What this does:
  1) Adds a new rule ID constant in src/rules.rs
  2) Adds a new RuleDoc entry in src/rules.rs with TODO summary/remediation text
  3) Creates examples/rules/<RULE_ID>.md contributor checklist
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 3 || $# -gt 5 ]]; then
  usage
  exit 1
fi

RULE_ID="$(printf '%s' "$1" | tr '[:lower:]' '[:upper:]')"
SEVERITY_INPUT="$(printf '%s' "$2" | tr '[:upper:]' '[:lower:]')"
TITLE="$3"
HELP_URL="${4:-PLUGIN_OVERVIEW}"
OW_VERSION="${5:-0.0.0}"
RULES_FILE="src/rules.rs"
EXAMPLE_DIR="examples/rules"
EXAMPLE_FILE="${EXAMPLE_DIR}/${RULE_ID}.md"

if [[ ! -f "${RULES_FILE}" ]]; then
  echo "Error: ${RULES_FILE} not found. Run from repository root." >&2
  exit 1
fi

if [[ ! "${RULE_ID}" =~ ^OW[A-Z]{1,4}[0-9]{3}$ ]]; then
  echo "Error: RULE_ID must match pattern ^OW[A-Z]{1,4}[0-9]{3}$ (example: OWP202, OWPL501)." >&2
  exit 1
fi

TITLE_ESCAPED="${TITLE//\\/\\\\}"
TITLE_ESCAPED="${TITLE_ESCAPED//\"/\\\"}"

if [[ "${HELP_URL}" =~ ^[A-Z_][A-Z0-9_]*$ ]]; then
  HELP_URL_EXPR="${HELP_URL}"
else
  HELP_URL_ESCAPED="${HELP_URL//\\/\\\\}"
  HELP_URL_ESCAPED="${HELP_URL_ESCAPED//\"/\\\"}"
  HELP_URL_EXPR="\"${HELP_URL_ESCAPED}\""
fi

case "${SEVERITY_INPUT}" in
  error) SEVERITY="Severity::Error" ;;
  warning) SEVERITY="Severity::Warning" ;;
  *)
    echo "Error: severity must be 'error' or 'warning'." >&2
    exit 1
    ;;
esac

if grep -q "pub const ${RULE_ID}:" "${RULES_FILE}" || grep -q "id: ${RULE_ID}," "${RULES_FILE}"; then
  echo "Error: rule '${RULE_ID}' already exists in ${RULES_FILE}." >&2
  exit 1
fi

TMP_FILE="$(mktemp)"
awk \
  -v new_const="pub const ${RULE_ID}: &str = \"${RULE_ID}\";" \
  -v rule_id="${RULE_ID}" \
  -v severity="${SEVERITY}" \
  -v title="${TITLE_ESCAPED}" \
  -v help_url_expr="${HELP_URL_EXPR}" \
  -v ow_version="\"${OW_VERSION}\"" \
  -v ow_version="${OW_VERSION}" \
'
  /^const RULES:/ && !const_inserted {
    print new_const
    const_inserted = 1
    in_rules = 1
    # If fixed-size array [RuleDoc; N], increment N
    if ($0 ~ /\[RuleDoc; [0-9]+\]/) {
      n = $0
      sub(/.*\[RuleDoc; /, "", n)
      sub(/\].*/, "", n)
      new_n = n + 1
      sub(/\[RuleDoc; [0-9]+\]/, "[RuleDoc; " new_n "]")
    }
  }
  in_rules && /^];$/ && !entry_inserted {
    print "    RuleDoc {"
    print "        id: " rule_id ","
    print "        default_severity: " severity ","
    print "        title: \"" title "\","
    print "        summary: \"TODO: describe what this rule validates.\","
    print "        remediation: \"TODO: describe the fix users should apply.\","
    print "        help_url: " help_url_expr ","
    print "        openwebui_version: " ow_version ","
    print "    },"
    entry_inserted = 1
  }
  { print }
' "${RULES_FILE}" > "${TMP_FILE}"
mv "${TMP_FILE}" "${RULES_FILE}"

mkdir -p "${EXAMPLE_DIR}"
cat > "${EXAMPLE_FILE}" <<EOF
# ${RULE_ID} - ${TITLE}

## Metadata Checklist

- [x] Added rule constant and RuleDoc entry in \`src/rules.rs\`
- [ ] Replaced TODO summary text with real explanation
- [ ] Replaced TODO remediation text with actionable fix instructions
- [ ] Verified \`help_url\` points to the right docs
- [ ] Verified \`openwebui_version\` points to the right docs

## Linter Wiring

- [ ] Added detection logic in \`src/linter.rs\` using:

\`\`\`rust
issues.push(issue(${RULE_ID}, path, line, column, "Actionable message"));
\`\`\`

## Tests

- [ ] Added or updated tests in \`tests/linter_tests.rs\` (or \`tests/cli_tests.rs\`)
- [ ] Asserted rule ID and severity
- [ ] Added config override coverage if relevant
EOF

echo "Scaffolded rule ${RULE_ID} in ${RULES_FILE}"
echo "Created contributor checklist at ${EXAMPLE_FILE}"
echo "Next steps:"
echo "  1) Fill TODO summary/remediation in src/rules.rs"
echo "  2) Add lint detection in src/linter.rs using issue(${RULE_ID}, ...)"
echo "  3) Add tests and run: make check"
