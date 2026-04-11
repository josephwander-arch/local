# local MCP Server — Health Check (doctor.ps1)
# Verifies binary, state directory, and git availability.

$ErrorActionPreference = "Continue"
$passed = 0
$failed = 0
$warnings = 0

function Write-Check {
    param([string]$Label, [string]$Status, [string]$Detail)
    switch ($Status) {
        "PASS" {
            Write-Host "  [PASS] " -ForegroundColor Green -NoNewline
            $script:passed++
        }
        "FAIL" {
            Write-Host "  [FAIL] " -ForegroundColor Red -NoNewline
            $script:failed++
        }
        "WARN" {
            Write-Host "  [WARN] " -ForegroundColor Yellow -NoNewline
            $script:warnings++
        }
    }
    Write-Host "$Label" -NoNewline
    if ($Detail) { Write-Host " — $Detail" -ForegroundColor DarkGray } else { Write-Host "" }
}

Write-Host ""
Write-Host "local MCP Server — Doctor" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan
Write-Host ""

# --- Check 1: Binary exists ---
$binaryPaths = @(
    "C:\CPC\servers\local.exe",
    "C:\CPC\servers\local-arm64.exe"
)
$binaryFound = $false
foreach ($bp in $binaryPaths) {
    if (Test-Path $bp) {
        $size = [math]::Round((Get-Item $bp).Length / 1KB)
        Write-Check "Binary found" "PASS" "$bp ($size KB)"
        $binaryFound = $true
        break
    }
}
if (-not $binaryFound) {
    Write-Check "Binary not found" "FAIL" "Expected at $($binaryPaths -join ' or ')"
}

# --- Check 2: State directory writable ---
$stateDir = Join-Path $env:LOCALAPPDATA "CPC\state"
if (-not (Test-Path $stateDir)) {
    try {
        New-Item -ItemType Directory -Path $stateDir -Force | Out-Null
        Write-Check "State directory created" "PASS" $stateDir
    } catch {
        Write-Check "Cannot create state directory" "FAIL" $stateDir
    }
} else {
    Write-Check "State directory exists" "PASS" $stateDir
}

# Test write
$testFile = Join-Path $stateDir "doctor_test.tmp"
try {
    Set-Content -Path $testFile -Value "doctor" -ErrorAction Stop
    Remove-Item $testFile -ErrorAction SilentlyContinue
    Write-Check "State directory writable" "PASS" ""
} catch {
    Write-Check "State directory not writable" "FAIL" $stateDir
}

# --- Check 3: Git available ---
$gitVersion = $null
try {
    $gitVersion = & git --version 2>&1
} catch {}

if ($gitVersion -and $gitVersion -match "git version") {
    Write-Check "Git available" "PASS" "$gitVersion"
} else {
    Write-Check "Git not found" "WARN" "Some session tools use git — install from https://git-scm.com"
}

# --- Check 4: Breadcrumb retention config ---
$retentionDays = $env:LOCAL_BREADCRUMB_RETENTION_DAYS
if ($retentionDays) {
    Write-Check "Breadcrumb retention" "PASS" "$retentionDays days (from env)"
} else {
    Write-Check "Breadcrumb retention" "PASS" "30 days (default)"
}

# --- Check 5: Logs directory ---
$logDir = "C:\CPC\logs"
if (Test-Path $logDir) {
    Write-Check "Logs directory exists" "PASS" $logDir
} else {
    Write-Check "Logs directory missing" "WARN" "Will be created on first tool call"
}

# --- Summary ---
Write-Host ""
Write-Host "Results: " -NoNewline
Write-Host "$passed passed" -ForegroundColor Green -NoNewline
if ($failed -gt 0) {
    Write-Host ", $failed failed" -ForegroundColor Red -NoNewline
}
if ($warnings -gt 0) {
    Write-Host ", $warnings warnings" -ForegroundColor Yellow -NoNewline
}
Write-Host ""

if ($failed -gt 0) {
    Write-Host "Fix the failures above before using local." -ForegroundColor Red
    exit 1
} else {
    Write-Host "local is ready." -ForegroundColor Green
    exit 0
}
