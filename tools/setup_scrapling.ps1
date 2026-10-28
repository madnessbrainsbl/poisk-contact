$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$venv = Join-Path $projectRoot '.scrapling'
if (!(Test-Path -LiteralPath (Join-Path $venv 'Scripts/python.exe'))) {
    python -m venv $venv
    if ($LASTEXITCODE -ne 0) { throw 'Python 3.10+ is required to create the Scrapling environment.' }
}
$python = Join-Path $venv 'Scripts/python.exe'
& $python -m pip install 'scrapling[fetchers]==0.5'
if ($LASTEXITCODE -ne 0) { throw 'Scrapling installation failed.' }
$chrome = Join-Path $env:ProgramFiles 'Google/Chrome/Application/chrome.exe'
if (!(Test-Path -LiteralPath $chrome)) {
    & $python -m playwright install chromium
    if ($LASTEXITCODE -ne 0) { throw 'Scrapling Chromium installation failed.' }
}
Write-Host 'Scrapling is ready. New collection runs will render JavaScript-only company pages automatically.'
