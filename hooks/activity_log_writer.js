// local MCP Server — Activity Log Writer (PostToolUse hook)
// Writes tool calls to a JSONL file consumed by dashboard.html.
//
// Install: Add to Claude Code settings.json as a PostToolUse hook.
//
// State path: {{STATE_PATH}} (doctor.ps1 substitutes the real path on install)

const fs = require('fs');
const path = require('path');

const LOG_DIR = '{{STATE_PATH}}';
const ACTIVITY_LOG = path.join(LOG_DIR, 'local_activity.jsonl');

try {
    const input = fs.readFileSync(0, 'utf8');
    const data = JSON.parse(input);
    const toolName = data.tool_name || 'unknown';
    const toolInput = data.tool_input || {};
    const ts = new Date().toISOString();

    if (!fs.existsSync(LOG_DIR)) fs.mkdirSync(LOG_DIR, { recursive: true });

    const entry = {
        timestamp: ts,
        tool: toolName,
        params: Object.keys(toolInput),
        status: 'completed'
    };

    fs.appendFileSync(ACTIVITY_LOG, JSON.stringify(entry) + '\n');
} catch (e) {}
process.exit(0);
