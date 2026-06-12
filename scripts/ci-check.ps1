# AgentPulse Local CI Check
# Mirrors .github/workflows/ci.yml for pre-merge validation.
# Usage:
#   .\scripts\ci-check.ps1           # fast checks (skip audit)
#   .\scripts\ci-check.ps1 -Full     # all checks including audit
#   .\scripts\ci-check.ps1 -Quick    # only compile + test

param(
    [switch]$Full,
    [switch]$Quick
)

# Use Continue so native stderr (cargo compile output, vitest error logs)
# doesn't become a terminating exception in PS 5.1.
$ErrorActionPreference = "Continue"
$script:failed = @()
$script:passed = @()
$startTime = Get-Date

$desktopDir = "apps/desktop"
$tauriDir = "$desktopDir/src-tauri"

function Step($name, $workDir, [scriptblock]$script) {
    Write-Host "`n=== $name ===" -ForegroundColor Cyan
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Push-Location $workDir
    try {
        & $script
        $exitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    $sw.Stop()
    if ($exitCode -ne 0) {
        $script:failed += $name
        Write-Host "  FAIL  ${name}  ($($sw.Elapsed.TotalSeconds.ToString('0.0'))s) - exit code: $exitCode" -ForegroundColor Red
    } else {
        $script:passed += $name
        Write-Host "  PASS  ${name}  ($($sw.Elapsed.TotalSeconds.ToString('0.0'))s)" -ForegroundColor Green
    }
}

# ---- TypeScript type check ----
Step "vue-tsc --noEmit" $desktopDir {
    npx vue-tsc --noEmit
}

# ---- Frontend tests ----
Step "npm test (Vitest)" $desktopDir {
    npm test
}

# ---- Python tests ----
Step "pytest" $PWD {
    python -m pytest tests/ -q
}

# ---- Rust format check ----
Step "cargo fmt --check" $tauriDir {
    cargo fmt --check
}

# ---- Rust clippy ----
Step "cargo clippy -- -D warnings" $tauriDir {
    cargo clippy -- -D warnings
}

# ---- Rust tests ----
Step "cargo test" $tauriDir {
    cargo test
}

# ---- Audit checks (only with -Full) ----
if ($Full -and -not $Quick) {
    Step "cargo audit" $tauriDir {
        cargo audit
        if ($LASTEXITCODE -ne 0) {
            cargo install cargo-audit --locked
            cargo audit
        }
    }

    Step "npm audit" $desktopDir {
        npm audit --audit-level=high
    }
}

# ---- Summary ----
$elapsed = ((Get-Date) - $startTime).TotalSeconds.ToString('0.0')
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "  CI Check Summary  (${elapsed}s)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Passed: $($script:passed.Count)" -ForegroundColor Green
if ($script:failed.Count -gt 0) {
    Write-Host "  Failed: $($script:failed.Count)" -ForegroundColor Red
    foreach ($f in $script:failed) {
        Write-Host "    - $f" -ForegroundColor Red
    }
    Write-Host "`nFix the failures above before pushing or merging." -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "  All checks passed! Safe to push/merge." -ForegroundColor Green
    exit 0
}
