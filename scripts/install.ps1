$ErrorActionPreference = "Stop"

$repo = "christestet/owui-lint"
$installDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE ".local\bin" }

$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq "Arm64") {
    "aarch64-pc-windows-msvc"
} else {
    "x86_64-pc-windows-msvc"
}

$archive = "owui-lint-${arch}.zip"
$url = "https://github.com/${repo}/releases/latest/download/${archive}"

Write-Host "Detected target: $arch"
Write-Host "Downloading $url..."

$tmp = New-TemporaryFile | Rename-Item -NewName { $_.Name + ".zip" } -PassThru
try {
    Invoke-WebRequest -Uri $url -OutFile $tmp.FullName
    $extract = Join-Path ([System.IO.Path]::GetTempPath()) "owui-lint-extract"
    Expand-Archive -Path $tmp.FullName -DestinationPath $extract -Force

    if (-not (Test-Path $installDir)) { New-Item -ItemType Directory -Path $installDir | Out-Null }
    Copy-Item (Join-Path $extract "owui-lint.exe") -Destination (Join-Path $installDir "owui-lint.exe") -Force

    Write-Host "Installed owui-lint to $installDir\owui-lint.exe"

    $pathDirs = $env:PATH -split ";"
    if ($pathDirs -notcontains $installDir) {
        Write-Host "Warning: $installDir is not in your PATH. Add it to your user environment variables."
    }
} finally {
    Remove-Item $tmp.FullName -ErrorAction SilentlyContinue
    Remove-Item $extract -Recurse -ErrorAction SilentlyContinue
}
