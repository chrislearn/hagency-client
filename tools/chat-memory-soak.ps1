# Isolated native RoomScreen soak. The instrument example never starts a real
# Matrix worker and only connects to its own loopback history fixture.
param(
    [int]$Seconds = 1800,
    [string]$Output = 'D:/BuildCache/hagency-client/chat-soak-20261021',
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
if ($Seconds -lt 30) { throw 'Use at least 30 seconds' }
$taskOutput = [IO.Path]::GetFullPath($Output).Replace('\','/')
if (-not $taskOutput.StartsWith('D:/BuildCache/hagency-client/')) { throw 'Output must stay on D:/BuildCache/hagency-client/' }
if (Test-Path -LiteralPath $taskOutput) { throw 'Use a fresh output directory' }
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if (-not $SkipBuild) {
        & ./tools/dev.ps1 build --locked --release -j 8 -p hagency-app --example chat_memory_soak --features chat-stress
        if ($LASTEXITCODE -ne 0) { throw 'Native fixture build failed' }
    }
    New-Item -ItemType Directory -Path $taskOutput | Out-Null
    $env:TEMP = 'D:/BuildCache/hagency-client/tmp'
    $env:TMP = $env:TEMP
    $env:HAGENCY_APP_DATA_DIR = "$taskOutput/profile"
    $env:HAGENCY_STRESS_OUTPUT = $taskOutput
    $env:HAGENCY_STRESS_SECONDS = "$Seconds"
    $env:HAGENCY_STRESS_SHOW = '1'
    $env:HAGENCY_RENDER_TRACE = '1'
    $env:RAYON_NUM_THREADS = '2'
    Remove-Item Env:MAKEPAD_HIDE_WINDOWS -ErrorAction SilentlyContinue
    Remove-Item Env:MAKEPAD_REMOTE -ErrorAction SilentlyContinue
    $env:MAKEPAD_NO_FOCUS = '1'
    $binary = 'D:/BuildCache/hagency-client/target/release/examples/chat_memory_soak.exe'
    $dependencyMetadata = & ./tools/dev.ps1 metadata --locked --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cannot record resolved renderer sources' }
    $rendererSources = @()
    foreach ($renderer in @(@('makepad-widgets', 'src/widget_tree.rs'), @('makepad-draw', 'src/image_cache.rs'))) {
        $package = @($dependencyMetadata.packages | Where-Object name -EQ $renderer[0])
        if ($package.Count -ne 1) { throw "Expected one resolved $($renderer[0]) package" }
        $rendererSources += Join-Path (Split-Path $package[0].manifest_path -Parent) $renderer[1]
    }
    $manifest = [ordered]@{ binary = $binary; sha256 = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash; seconds = $Seconds; sources = @{} }
    foreach($source in @('Cargo.toml','Cargo.lock','crates/hagency-app/examples/chat_memory_soak.rs','crates/hagency-app/src/memory_budget.rs','crates/hagency-app/src/media_transport.rs','crates/hagency-app/src/render_maintenance.rs','crates/hagency-app/src/shared/image_viewer.rs','crates/hagency-app/src/shared/file_upload_modal.rs','crates/hagency-app/src/home/room_image_viewer.rs','crates/hagency-app/src/avatar_cache.rs','crates/hagency-app/src/media_cache.rs','crates/hagency-app/src/sliding_sync.rs','crates/hagency-app/src/timeline_updates.rs','crates/hagency-app/src/home/room_screen.rs') + $rendererSources) {
        $manifest.sources[$source] = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
    }
    $manifest | ConvertTo-Json -Depth 4 | Set-Content "$taskOutput/manifest.json"
    $launchTime = [DateTime]::UtcNow
    $soakProcess = Start-Process -FilePath $binary -ArgumentList '--remote' `
        -WorkingDirectory (Get-Location).Path -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput "$taskOutput/native.stdout.log" -RedirectStandardError "$taskOutput/native.stderr.log"
    $soakProcess.Id | Set-Content "$taskOutput/pid.txt"
    Write-Output "Native soak pid=$($soakProcess.Id) output=$taskOutput seconds=$Seconds"
    while (-not $soakProcess.HasExited) {
        $soakProcess.Refresh()
        if ($soakProcess.HasExited) { break }
        $gpuDedicated = $null
        $gpuShared = $null
        try {
            $gpuSamples = (Get-Counter -Counter @("\GPU Process Memory(pid_$($soakProcess.Id)_*)\Dedicated Usage", "\GPU Process Memory(pid_$($soakProcess.Id)_*)\Shared Usage") -ErrorAction Stop).CounterSamples
            $gpuDedicated = [long](($gpuSamples | Where-Object Path -Like '*\dedicated usage' | Measure-Object CookedValue -Sum).Sum)
            $gpuShared = [long](($gpuSamples | Where-Object Path -Like '*\shared usage' | Measure-Object CookedValue -Sum).Sum)
        } catch { } # Missing/unsupported driver counters remain unknown.
        [ordered]@{
            utc = [DateTime]::UtcNow.ToString('o')
            pid = $soakProcess.Id
            working_set = $soakProcess.WorkingSet64
            private_bytes = $soakProcess.PrivateMemorySize64
            gpu_dedicated_bytes = $gpuDedicated
            gpu_shared_bytes = $gpuShared
            peak_working_set = $soakProcess.PeakWorkingSet64
            cpu_seconds = $soakProcess.TotalProcessorTime.TotalSeconds
            handles = $soakProcess.HandleCount
            threads = $soakProcess.Threads.Count
        } | ConvertTo-Json -Compress | Add-Content "$taskOutput/process.jsonl"
        Start-Sleep -Seconds 5
    }
    $soakProcess.WaitForExit()
    $elapsed = ([DateTime]::UtcNow - $launchTime).TotalSeconds
    [ordered]@{ exit_code = $soakProcess.ExitCode; requested_seconds = $Seconds; elapsed_seconds = $elapsed; ended_utc = [DateTime]::UtcNow.ToString('o') } |
        ConvertTo-Json | Set-Content "$taskOutput/completion.json"
    if ($soakProcess.ExitCode -ne 0) { throw "Native fixture exited $($soakProcess.ExitCode)" }
    if ($elapsed -lt $Seconds) { throw 'Native fixture was closed before the requested duration' }
} finally { Pop-Location }
