// local MCP Server — Breadcrumb Enforcer (PostToolUse hook)
// Lifecycle: TodoWrite (plan) -> breadcrumb_start -> breadcrumb_step -> breadcrumb_complete
//
// Nudges Claude through the breadcrumb lifecycle based on todo progress and
// work-tool counting. Matchers target mcp__local__* tools.
//
// Install: Add to Claude Code settings.json as a PostToolUse hook.
//
// State path: {{STATE_PATH}} (doctor.ps1 substitutes the real path on install)

const fs = require('fs');
const path = require('path');

const STATE_DIR = '{{STATE_PATH}}';
const STATE_FILE = path.join(STATE_DIR, 'cpc_breadcrumb_state.json');
const ACTIVE_BREADCRUMB_FILE = path.join(STATE_DIR, 'breadcrumb_active.json');

const START_TOOLS = new Set([
    'mcp__local__breadcrumb_start'
]);
const STEP_TOOLS = new Set([
    'mcp__local__breadcrumb_step'
]);
const COMPLETE_TOOLS = new Set([
    'mcp__local__breadcrumb_complete'
]);

function hasBreadcrumb() {
    try {
        if (fs.existsSync(ACTIVE_BREADCRUMB_FILE)) {
            const content = fs.readFileSync(ACTIVE_BREADCRUMB_FILE, 'utf8').trim();
            if (content && content.length > 2) {
                const bc = JSON.parse(content);
                if (bc.started_at || bc.started) {
                    const hoursOld = (Date.now() - new Date(bc.started_at || bc.started)) / 3600000;
                    if (hoursOld > 4) return false;
                }
                return true;
            }
        }
    } catch (e) {}
    return false;
}

function loadState() {
    try {
        if (fs.existsSync(STATE_FILE)) {
            const state = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
            if (state.lastTool && (Date.now() - new Date(state.lastTool)) > 1800000) {
                return freshState();
            }
            return state;
        }
    } catch (e) {}
    return freshState();
}

function freshState() {
    return {
        workCount: 0,
        nudgeCount: 0,
        lastTool: new Date().toISOString(),
        hasTodos: false,
        todoTotal: 0,
        todoDone: 0,
        lastStepAt: 0,
        phase: 'idle'
    };
}

function saveState(state) {
    try {
        if (!fs.existsSync(STATE_DIR)) fs.mkdirSync(STATE_DIR, { recursive: true });
        fs.writeFileSync(STATE_FILE, JSON.stringify(state));
    } catch (e) {}
}

function isWorkTool(name) {
    if (!name) return false;
    const skip = ['Read', 'Glob', 'Grep', 'Agent', 'Skill', 'ToolSearch', 'TodoWrite'];
    if (skip.includes(name)) return false;
    if (['Edit', 'Write', 'Bash', 'NotebookEdit'].includes(name)) return true;
    if (name.startsWith('mcp__') && !name.includes('read') && !name.includes('search')
        && !name.includes('list') && !name.includes('status') && !name.includes('get_')) {
        return true;
    }
    return false;
}

try {
    const input = fs.readFileSync(0, 'utf8');
    const data = JSON.parse(input);
    const toolName = data.tool_name || '';
    const toolInput = data.tool_input || {};

    const state = loadState();
    state.lastTool = new Date().toISOString();

    // --- PHASE TRANSITIONS ---

    if (toolName === 'TodoWrite') {
        const todos = toolInput.todos || [];
        if (todos.length > 0) {
            state.hasTodos = true;
            state.todoTotal = todos.length;
            state.todoDone = todos.filter(t => t.status === 'completed').length;

            if (state.todoDone === 0 && todos.length >= 2) {
                state.phase = 'planned';
                saveState(state);
                if (!hasBreadcrumb()) {
                    console.log(JSON.stringify({
                        hookSpecificOutput: {
                            hookEventName: 'PostToolUse',
                            additionalContext: `[BREADCRUMB] You just created a ${todos.length}-item task list. ` +
                                `Start a breadcrumb now with a specific title that names the component and mutable targets, ` +
                                `for example "local config repair | targets: config.toml, settings.json", ` +
                                `and ${Math.min(todos.length, 8)} steps derived from the todo items. ` +
                                `Then mark breadcrumb_step as you complete milestone groups.`
                        }
                    }));
                    process.exit(0);
                }
            }

            if (state.todoDone > 0 && state.todoDone < state.todoTotal) {
                state.phase = 'tracking';
                const stepInterval = Math.max(1, Math.floor(state.todoTotal / 4));
                if (state.todoDone - state.lastStepAt >= stepInterval && hasBreadcrumb()) {
                    state.lastStepAt = state.todoDone;
                    saveState(state);
                    console.log(JSON.stringify({
                        hookSpecificOutput: {
                            hookEventName: 'PostToolUse',
                            additionalContext: `[BREADCRUMB STEP] ${state.todoDone}/${state.todoTotal} tasks done. ` +
                                `Log a breadcrumb_step for this milestone before continuing.`
                        }
                    }));
                    process.exit(0);
                }
            }

            if (state.todoDone === state.todoTotal && state.todoTotal > 0) {
                state.phase = 'done';
                saveState(state);
                if (hasBreadcrumb()) {
                    console.log(JSON.stringify({
                        hookSpecificOutput: {
                            hookEventName: 'PostToolUse',
                            additionalContext: `[BREADCRUMB COMPLETE] All ${state.todoTotal} tasks are done. ` +
                                `Call breadcrumb_complete now to close out this operation.`
                        }
                    }));
                    process.exit(0);
                }
            }

            saveState(state);
            console.log('{}');
            process.exit(0);
        }
    }

    // --- BREADCRUMB LIFECYCLE RESETS ---

    if (START_TOOLS.has(toolName)) {
        state.phase = 'tracking';
        state.workCount = 0;
        state.nudgeCount = 0;
        state.lastStepAt = state.todoDone || 0;
        saveState(state);
        console.log('{}');
        process.exit(0);
    }

    if (STEP_TOOLS.has(toolName)) {
        state.workCount = 0;
        state.lastStepAt = state.todoDone || 0;
        saveState(state);
        console.log('{}');
        process.exit(0);
    }

    if (COMPLETE_TOOLS.has(toolName)) {
        saveState(freshState());
        console.log('{}');
        process.exit(0);
    }

    // --- FALLBACK: WORK-TOOL COUNTING ---

    if (!isWorkTool(toolName)) {
        saveState(state);
        console.log('{}');
        process.exit(0);
    }

    state.workCount++;

    if (!state.hasTodos && !hasBreadcrumb() && state.workCount >= 4) {
        state.nudgeCount++;
        saveState(state);
        if (state.nudgeCount <= 2) {
            console.log(JSON.stringify({
                hookSpecificOutput: {
                    hookEventName: 'PostToolUse',
                    additionalContext: `[BREADCRUMB REQUIRED] ${state.workCount} file mutations without a breadcrumb or task list. ` +
                        `Create a TodoWrite plan first, then breadcrumb_start with a specific title declaring component and targets.`
                }
            }));
        } else {
            console.log('{}');
        }
        process.exit(0);
    }

    saveState(state);
    console.log('{}');
} catch (e) {
    console.log('{}');
}
process.exit(0);
