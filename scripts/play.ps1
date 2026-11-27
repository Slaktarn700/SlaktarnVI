# Starts the title with the settings this fork is played with (README.md, "Windows").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
#   scripts\play.ps1 -Exe <...> -S70 -FidelityFX <folder with the FidelityFX DLLs>
# -S70          the S70 preset of the Linux launcher: 2560x1440 output, FSR at 0.6667 scale,
#               frame generation, 60 fps pacing, the info panel (needs -FidelityFX)
# -FidelityFX   folder with amd_fidelityfx_upscaler_dx12.dll and amd_fidelityfx_framegeneration_dx12.dll
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Build        target\diag (the default, the build all measurements are of) or target\release
# -More         other settings: -More @{VICTORY_PANEL='1'}
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$S70,
    [string]$FidelityFX = '',
    [switch]$Safe,
    [switch]$RayTracing,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{}
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'
    VICTORY_PORT_DESCRIPTORS = '1'; VICTORY_PORT_CONSTANTS = '1'; VICTORY_PORT_PRIVATE = '1'; VICTORY_PORT_CBV = '1'
    VICTORY_FRAME_INTERVAL_MS = '16.667'; VICTORY_PIPELINE_WAIT_MS = '30'
}
# Characters stretched to a point without it (the title writes skinning constants after it sends).
if (-not $Safe) { $settings.VICTORY_CARD_STEPS_LEARN = '1' }
if ($S70) {
    if (-not $FidelityFX -or -not (Test-Path (Join-Path $FidelityFX 'amd_fidelityfx_upscaler_dx12.dll'))) {
        Write-Host "-S70 needs -FidelityFX <folder with amd_fidelityfx_upscaler_dx12.dll>."; exit 1
    }
    $settings.VICTORY_OUTPUT = '2560x1440'; $settings.VICTORY_UPSCALER = '7'; $settings.VICTORY_FFX_SCALE = '0.6667'
    $settings.VICTORY_FIDELITYFX_DIR = (Resolve-Path $FidelityFX).Path; $settings.VICTORY_FIDELITYFX_FRAMEGEN = '1'
    $settings.VICTORY_AMBIENT_REFLECT = '1'; $settings.VICTORY_CARD_BUFFERS_MIN_KB = '64'; $settings.VICTORY_PANEL = '1'
}
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
}
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

# The program is valhalla.exe (victory.exe in older builds); built natively or cross-built.
$victory = @("target\$Build", "target\x86_64-pc-windows-msvc\$Build") |
    ForEach-Object { $dir = $_; 'valhalla.exe', 'victory.exe' | ForEach-Object { Join-Path $PSScriptRoot "..\$dir\$_" } } |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $victory) { Write-Host "No valhalla.exe under target\$Build. Build it first (README.md)."; exit 1 }
$series = if ($S70) { @('--console', 'series-x') } else { @() }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; exit 1 }
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR'
$inherited = @(Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths })
foreach ($entry in $inherited) { Remove-Item "Env:$($entry.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$discarded = $inherited | Where-Object {
    $_.Name -notin $More.Keys -and -not $_.Value.Equals([Environment]::GetEnvironmentVariable($_.Name))
} | Sort-Object Name
if ($discarded) {
    Write-Host "Discarded inherited values for $($discarded.Name -join ', '). Use -More to pass them explicitly."
}
$Exe = (Resolve-Path $Exe).Path
Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
try { & $victory run $Exe --execute @series } finally { Pop-Location }
