# Keep Cargo switches verbatim; advanced PowerShell parameter binding would
# interpret Cargo's -p as an ambiguous common PowerShell parameter.
$CargoArguments = @($args)
$ErrorActionPreference = 'Stop'
$taskCache = 'D:/BuildCache/hagency-client'
$env:CARGO_TARGET_DIR = "$taskCache/target"
$env:CARGO_HOME = "$taskCache/cargo"
$env:TEMP = "$taskCache/tmp"
$env:TMP = $env:TEMP
$env:npm_config_cache = "$taskCache/npm"
$env:PYTHONDONTWRITEBYTECODE = '1'
New-Item -ItemType Directory -Force $env:CARGO_TARGET_DIR,$env:CARGO_HOME,$env:TEMP,$env:npm_config_cache | Out-Null
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $taskMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve Cargo workspace' }
    if ($taskMetadata.target_directory.Replace('\','/') -ne $env:CARGO_TARGET_DIR) {
        throw "Unexpected Cargo output directory: $($taskMetadata.target_directory)"
    }
    if ($CargoArguments.Count -eq 0) { $CargoArguments = @('run','--locked','-p','rinx','--bin','rinx') }
    & cargo @CargoArguments
    $taskExit = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $taskExit
