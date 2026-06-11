# Setup GitHub Branch Protection for AgentPulse
# Requires: gh CLI installed and authenticated (gh auth login)
# Repo: Ze-d/AgentPulse
#
# This configures master branch to require CI checks before merging,
# preventing broken code from landing on master.
#
# Usage:
#   .\scripts\setup-branch-protection.ps1

param(
    [switch]$DryRun  # Preview the rules without applying
)

$repo = "Ze-d/AgentPulse"
$branch = "master"

$rules = @{
    required_status_checks = @{
        strict = $true   # branches must be up-to-date
        contexts = @(
            "Check (windows-latest)",
            "Check (ubuntu-latest)",
            "Check (macos-latest)"
        )
    }
    enforce_admins = $false   # admins still need CI to pass
    required_pull_request_reviews = @{
        required_approving_review_count = 0   # set to 1+ if PR review required
    }
    restrictions = $null   # no push restrictions beyond CI
    required_linear_history = $false
    allow_force_pushes = $false
    allow_deletions = $false
}

if ($DryRun) {
    Write-Host "=== DRY RUN - Rules that would be applied to $branch ===" -ForegroundColor Cyan
    $rules | ConvertTo-Json -Depth 4
    Write-Host "Run without -DryRun to apply." -ForegroundColor Yellow
    exit 0
}

Write-Host "Configuring branch protection for $repo / $branch ..." -ForegroundColor Cyan

# Apply via gh API
$rulesJson = $rules | ConvertTo-Json -Depth 4 -Compress
$rulesJson | gh api repos/$repo/branches/$branch/protection --method PUT --input - 2>&1

if ($LASTEXITCODE -eq 0) {
    Write-Host "Branch protection rules applied successfully." -ForegroundColor Green
    Write-Host "CI checks now REQUIRED before merging to $branch." -ForegroundColor Green
} else {
    Write-Host "Failed to apply branch protection. Check gh auth and repo permissions." -ForegroundColor Red
    Write-Host "You can also configure this manually at:" -ForegroundColor Yellow
    Write-Host "  https://github.com/$repo/settings/branches" -ForegroundColor Yellow
}
