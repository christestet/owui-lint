#!/usr/bin/env bash
# Test suite for scripts/new-rule.sh
set -euo pipefail

SCRIPT="scripts/new-rule.sh"
RULES_FILE="src/rules.rs"

passes=0
fails=0
total=0

pass() {
  ((passes++)) || true
  ((total++)) || true
  echo "  PASS: $1"
}

fail() {
  ((fails++)) || true
  ((total++)) || true
  echo "  FAIL: $1"
}

assert_file_contains() {
  local file="$1" pattern="$2" label="$3"
  if grep -q "$pattern" "$file"; then
    pass "$label"
  else
    fail "$label — pattern not found: $pattern"
  fi
}

assert_file_not_contains() {
  local file="$1" pattern="$2" label="$3"
  if grep -q "$pattern" "$file"; then
    fail "$label — pattern unexpectedly found: $pattern"
  else
    pass "$label"
  fi
}

assert_exit_nonzero() {
  local label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    fail "$label — expected non-zero exit"
  else
    pass "$label"
  fi
}

# --- Setup ---
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

# Copy rules.rs and script into temp workspace
mkdir -p "$WORK_DIR/src" "$WORK_DIR/scripts" "$WORK_DIR/examples/rules"
cp "$RULES_FILE" "$WORK_DIR/src/rules.rs"
cp "$SCRIPT" "$WORK_DIR/scripts/new-rule.sh"
chmod +x "$WORK_DIR/scripts/new-rule.sh"

run_script() {
  (cd "$WORK_DIR" && bash scripts/new-rule.sh "$@")
}

reset_rules() {
  cp "$RULES_FILE" "$WORK_DIR/src/rules.rs"
}

echo "=== Test Suite: scripts/new-rule.sh ==="
echo ""

# --- Test 1: Happy path (warning) ---
echo "--- Test 1: Happy path (warning) ---"
reset_rules
run_script OWC600 warning "Missing cache timeout" >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" 'pub const OWC600' "Constant added"
assert_file_contains "$WORK_DIR/src/rules.rs" 'id: OWC600,' "RuleDoc entry added"
assert_file_contains "$WORK_DIR/src/rules.rs" 'Severity::Warning' "Severity is Warning"
test -f "$WORK_DIR/examples/rules/OWC600.md" && pass "Example .md created" || fail "Example .md not created"

# --- Test 2: Happy path (error) ---
echo "--- Test 2: Happy path (error) ---"
reset_rules
run_script OWC601 error "Bad config" >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" 'Severity::Error' "Severity is Error for error rule"

# --- Test 3: Array size increment ---
echo "--- Test 3: Array size increment ---"
reset_rules
# Force sized array syntax for this test
sed 's/&\[RuleDoc\] = &\[/[RuleDoc; 21] = [/' "$WORK_DIR/src/rules.rs" > "$WORK_DIR/src/rules.rs.tmp"
mv "$WORK_DIR/src/rules.rs.tmp" "$WORK_DIR/src/rules.rs"
run_script OWC602 warning "Size test" >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" '\[RuleDoc; 22\]' "Array size incremented 21 -> 22"

# --- Test 4: Slice syntax unchanged ---
echo "--- Test 4: Slice syntax unchanged ---"
reset_rules
run_script OWC603 warning "Slice test" >/dev/null
assert_file_not_contains "$WORK_DIR/src/rules.rs" '\[RuleDoc; [0-9]' "No sized array introduced for slice syntax"
assert_file_contains "$WORK_DIR/src/rules.rs" 'id: OWC603,' "Entry still added with slice syntax"

# --- Test 5: Duplicate detection ---
echo "--- Test 5: Duplicate detection ---"
reset_rules
run_script OWC604 warning "First add" >/dev/null
assert_exit_nonzero "Duplicate rule rejected" run_script OWC604 warning "Duplicate"

# --- Test 6: Invalid RULE_ID format ---
echo "--- Test 6: Invalid RULE_ID format ---"
reset_rules
assert_exit_nonzero "Rejects INVALID" run_script INVALID warning "Bad id"
assert_exit_nonzero "Rejects OW (no digits)" run_script OW warning "Bad id"
assert_exit_nonzero "Rejects abc123" run_script abc123 warning "Bad id"

# --- Test 7: Lowercase auto-uppercased ---
echo "--- Test 7: Lowercase auto-uppercased ---"
reset_rules
run_script owc605 warning "Lowercase test" >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" 'pub const OWC605' "Lowercase owc605 uppercased to OWC605"

# --- Test 8: Invalid severity ---
echo "--- Test 8: Invalid severity ---"
reset_rules
assert_exit_nonzero "Rejects 'info' severity" run_script OWC606 info "Bad severity"

# --- Test 9: Missing arguments ---
echo "--- Test 9: Missing arguments ---"
assert_exit_nonzero "0 args rejected" run_script
assert_exit_nonzero "1 arg rejected" run_script OWC607
assert_exit_nonzero "2 args rejected" run_script OWC607 warning

# --- Test 10: Custom help_url (4th arg) ---
echo "--- Test 10: Custom help_url ---"
reset_rules
run_script OWC608 warning "Custom URL" "https://example.com/docs" >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" '"https://example.com/docs"' "Custom URL appears quoted"

# --- Test 11: Constant help_url (default) ---
echo "--- Test 11: Default help_url ---"
reset_rules
run_script OWC609 warning "Default URL" >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" 'help_url: PLUGIN_OVERVIEW,' "Default PLUGIN_OVERVIEW unquoted"

# --- Test 12: Special characters in title ---
echo "--- Test 12: Special characters in title ---"
reset_rules
run_script OWC610 warning 'Quote "test" and backslash \\ end' >/dev/null
assert_file_contains "$WORK_DIR/src/rules.rs" 'id: OWC610,' "Rule with special chars created"

# --- Summary ---
echo ""
echo "=== Results: ${passes} passed, ${fails} failed, ${total} total ==="
if [[ "$fails" -gt 0 ]]; then
  exit 1
fi
