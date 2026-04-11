// local MCP Server — Post-Bash Logger (PostToolUse hook)
// Logs all Bash commands to an audit trail.
//
// Install: Add to Claude Code settings.json as a PostToolUse hook matching
// the Bash tool.
//
// State path: {{STATE_PATH}} (doctor.ps1 substitutes the real path on install)

const fs = require('fs');
const path = require('path');

const LOG_DIR = '{{STATE_PATH}}';
const LOG_FILE = path.join(LOG_DIR, 'bash_commands.log');

try {
    const input = fs.readFileSync(0, 'utf8');
    const ts = new Date().toISOString().replace('T', ' ').substring(0, 19);
    if (!fs.existsSync(LOG_DIR)) fs.mkdirSync(LOG_DIR, { recursive: true });
    const data = JSON.parse(input);
    const cmd = data.tool_input && data.tool_input.command;
    if (cmd) {
        fs.appendFileSync(LOG_FILE, '[' + ts + '] ' + cmd + '\n');
    }
} catch (e) {}
process.exit(0);
