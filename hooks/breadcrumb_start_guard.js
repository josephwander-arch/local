// local MCP Server — Breadcrumb Start Guard (PreToolUse hook)
// Blocks vague breadcrumb titles and warns on active-operation overlap.
//
// Install: Add to Claude Code settings.json as a PreToolUse hook matching
// mcp__local__breadcrumb_start.
//
// State path: {{STATE_PATH}} (doctor.ps1 substitutes the real path on install)

const fs = require('fs');
const path = require('path');

const STATE_DIR = '{{STATE_PATH}}';
const ACTIVE_BREADCRUMB_FILE = path.join(STATE_DIR, 'breadcrumb_active.json');

const GENERIC_PATTERNS = [
    /^fix(\s|$)/i,
    /^update(\s|$)/i,
    /^work(\s|$)/i,
    /^task(\s|$)/i,
    /^changes?(\s|$)/i,
    /^cleanup(\s|$)/i,
    /^repair(\s|$)/i,
    /^misc(\s|$)/i,
    /^investigate(\s|$)/i,
    /^debug(\s|$)/i,
    /^continue(\s|$)/i
];

function readJson(filePath) {
    try {
        if (!fs.existsSync(filePath)) return null;
        const content = fs.readFileSync(filePath, 'utf8').trim();
        return content ? JSON.parse(content) : null;
    } catch (_error) {
        return null;
    }
}

function normalizeName(name) {
    return (name || '').toLowerCase().replace(/\s+/g, ' ').trim();
}

function nameHasExplicitTargets(name) {
    return /\b(targets?|files?|paths?|topics?|component)\s*:/i.test(name);
}

function nameLooksSpecific(name) {
    const trimmed = (name || '').trim();
    if (!trimmed) return false;
    if (GENERIC_PATTERNS.some(pattern => pattern.test(trimmed))) return false;

    const tokenCount = trimmed.split(/\s+/).filter(Boolean).length;
    const hasPathLikeTarget =
        /[A-Za-z]:\\/.test(trimmed) ||
        /[/\\][^/\\]+/.test(trimmed) ||
        /\.[A-Za-z0-9]{1,6}\b/.test(trimmed);
    const hasDelimiter = trimmed.includes('|') || trimmed.includes(' - ') || trimmed.includes(': ');

    return nameHasExplicitTargets(trimmed) || hasPathLikeTarget || (tokenCount >= 4 && hasDelimiter);
}

function deny(reason) {
    console.log(JSON.stringify({
        hookSpecificOutput: {
            hookEventName: 'PreToolUse',
            permissionDecision: 'deny',
            permissionDecisionReason: reason
        }
    }));
}

try {
    const data = JSON.parse(fs.readFileSync(0, 'utf8'));
    const toolInput = data.tool_input || {};
    const name = typeof toolInput.operation_name === 'string' ? toolInput.operation_name.trim()
        : typeof toolInput.name === 'string' ? toolInput.name.trim() : '';

    if (!nameLooksSpecific(name)) {
        deny('Breadcrumb title too generic. Use a title that names the component plus mutable targets, for example "local deploy v2.1 | targets: C:\\\\CPC\\\\servers\\\\local.exe".');
        process.exit(0);
    }

    if (!nameHasExplicitTargets(name)) {
        deny('Breadcrumb title must declare mutable targets with a "targets:" segment so other agents can see what files are being touched.');
        process.exit(0);
    }

    // Check for active breadcrumb overlap
    const active = readJson(ACTIVE_BREADCRUMB_FILE);
    if (active) {
        const activeName = active.name || active.operation_name || 'unnamed';
        if (normalizeName(activeName) !== normalizeName(name)) {
            deny(`Active breadcrumb already in progress: "${activeName}". Resume, abort, or take over before starting a new operation.`);
            process.exit(0);
        }
    }

    process.exit(0);
} catch (_error) {
    process.exit(0);
}
