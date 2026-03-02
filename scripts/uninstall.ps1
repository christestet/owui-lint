$ErrorActionPreference = "Stop"

$installDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE ".local\bin" }
$binary = Join-Path $installDir "owui-lint.exe"

if (-not (Test-Path $binary)) {
    $found = Get-Command owui-lint -ErrorAction SilentlyContinue
    if ($found) {
        $binary = $found.Source
    } else {
        Write-Error "owui-lint not found."
        exit 1
    }
}

Remove-Item $binary -Force
Write-Host "Removed $binary"
