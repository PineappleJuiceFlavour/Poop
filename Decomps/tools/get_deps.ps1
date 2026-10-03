# Downloads and installs everything the JC3xGTA5 mod needs, from each tool's official site:
#   ScriptHookV + its SDK (dev-c.com), ReShade with add-on support (reshade.me), ReShade's shader headers,
#   and optionally Ghidra + a JDK (for run_ghidra.bat). Nothing here is redistributed by the repo.
param([string]$Jc3Dir, [string]$GtaDir, [switch]$Ghidra)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = 'Tls12'
$here   = Split-Path -Parent $MyInvocation.MyCommand.Path
$mod    = Join-Path $here '..\JC3xGTA5'
$cache  = Join-Path $here '..\local\deps'
New-Item -ItemType Directory -Force $cache | Out-Null
$UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36'
$ReShadeVer = '6.8.0'

function Get-File($url, $out, $referer) {
    if (Test-Path $out) { return $out }
    Write-Host "  downloading $url"
    $h = @{ 'User-Agent' = $UA }; if ($referer) { $h['Referer'] = $referer }
    Invoke-WebRequest -Uri $url -OutFile $out -Headers $h -UseBasicParsing
    $out
}
function Copy-IfChanged($src, $dst) { Copy-Item $src $dst -Force; Write-Host "  -> $dst" }

# ---------- ScriptHookV + SDK ----------
Write-Host 'ScriptHookV...'
$page = (Invoke-WebRequest 'https://www.dev-c.com/gtav/scripthookv/' -Headers @{ 'User-Agent' = $UA } -UseBasicParsing).Content
$zips = [regex]::Matches($page, '/files/ScriptHookV_[^"]*?\.zip') | ForEach-Object Value | Sort-Object -Unique
if (-not $zips) { throw 'No ScriptHookV downloads found on dev-c.com. Get them by hand from https://www.dev-c.com/gtav/scripthookv/' }
foreach ($z in $zips) { Get-File "https://www.dev-c.com$z" (Join-Path $cache (Split-Path $z -Leaf)) 'https://www.dev-c.com/gtav/scripthookv/' | Out-Null }
$sdkZip = Get-ChildItem $cache -Filter 'ScriptHookV_SDK_*.zip' | Select-Object -Last 1
$rtZip  = Get-ChildItem $cache -Filter 'ScriptHookV_*.zip' | Where-Object Name -notmatch 'SDK' | Select-Object -Last 1
$sdkTmp = Join-Path $cache 'sdk_x'; $rtTmp = Join-Path $cache 'rt_x'
Expand-Archive $sdkZip.FullName $sdkTmp -Force
Expand-Archive $rtZip.FullName  $rtTmp  -Force
$sdkDst = Join-Path $mod 'gta\sdk'
New-Item -ItemType Directory -Force "$sdkDst\inc", "$sdkDst\lib" | Out-Null
Copy-Item "$sdkTmp\inc\*" "$sdkDst\inc" -Force
Copy-Item "$sdkTmp\lib\*" "$sdkDst\lib" -Force
Write-Host "  SDK -> $sdkDst"
if ($GtaDir) {
    foreach ($f in 'ScriptHookV.dll', 'dinput8.dll') {
        $dst = Join-Path $GtaDir $f
        if ((Test-Path $dst) -and $f -eq 'dinput8.dll' -and ((Get-Item $dst).Length -ne (Get-Item "$rtTmp\bin\$f").Length)) {
            Write-Host "  keeping your existing $f (another ASI loader?)"; continue
        }
        Copy-IfChanged "$rtTmp\bin\$f" $dst
    }
}

# ---------- ReShade with add-on support ----------
Write-Host 'ReShade...'
$setup = Get-File "https://reshade.me/downloads/ReShade_Setup_${ReShadeVer}_Addon.exe" (Join-Path $cache "ReShade_${ReShadeVer}_Addon.exe") 'https://reshade.me/'
$rsTmp = Join-Path $cache 'reshade_x'
New-Item -ItemType Directory -Force $rsTmp | Out-Null
# the setup exe carries its DLLs as an appended zip; Windows' tar (libarchive) reads it
tar -xf $setup -C $rsTmp ReShade64.dll 2>$null
$rs = Join-Path $rsTmp 'ReShade64.dll'
if (-not (Test-Path $rs)) { throw "Couldn't unpack ReShade64.dll from $setup. Run that installer by hand for both games (DirectX 11, add-on support)." }
$fxh = @()
foreach ($f in 'ReShade.fxh', 'ReShadeUI.fxh') {
    $fxh += Get-File "https://raw.githubusercontent.com/crosire/reshade-shaders/slim/Shaders/$f" (Join-Path $cache $f)
}
$defs = 'RESHADE_DEPTH_INPUT_IS_REVERSED=1,RESHADE_DEPTH_LINEARIZATION_FAR_PLANE=1000,RESHADE_DEPTH_INPUT_IS_UPSIDE_DOWN=0,RESHADE_DEPTH_INPUT_IS_LOGARITHMIC=0'
function Install-ReShade($dir, $dllName, $fx, $technique) {
    Copy-IfChanged $rs (Join-Path $dir $dllName)
    $sh = Join-Path $dir 'reshade-shaders\Shaders'
    New-Item -ItemType Directory -Force $sh | Out-Null
    $fxh | ForEach-Object { Copy-Item $_ $sh -Force }
    Copy-IfChanged $fx $sh
    $ini = Join-Path $dir 'ReShade.ini'
    if (-not (Test-Path $ini)) {
        "[GENERAL]`r`nEffectSearchPaths=.\reshade-shaders\Shaders\**`r`nPresetPath=.\ReShadePreset.ini`r`nPreprocessorDefinitions=$defs`r`n" | Set-Content $ini -Encoding ASCII
        Write-Host "  -> $ini"
    }
    $preset = Join-Path $dir 'ReShadePreset.ini'
    if (-not (Test-Path $preset)) {
        "Techniques=$technique`r`nTechniqueSorting=$technique`r`n" | Set-Content $preset -Encoding ASCII
        Write-Host "  -> $preset"
    }
}
if ($Jc3Dir) { Install-ReShade $Jc3Dir 'dxgi.dll' (Join-Path $mod 'jc3\shaders\JC3Export.fx') 'JC3Export@JC3Export.fx' }
# GTA loads the system dxgi.dll ahead of one in its folder, so ReShade goes in as an ASI there
if ($GtaDir) { Install-ReShade $GtaDir 'ReShade64.asi' (Join-Path $mod 'gta\shaders\JC3Passthrough.fx') 'JC3Passthrough@JC3Passthrough.fx' }

# ---------- Ghidra + JDK (optional) ----------
if ($Ghidra) {
    Write-Host 'Ghidra...'
    $tools = Join-Path $here '..\local\tools'
    New-Item -ItemType Directory -Force $tools | Out-Null
    if (-not (Get-ChildItem $tools -Directory -Filter 'jdk-*' -ErrorAction SilentlyContinue)) {
        $jz = Get-File 'https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jdk/hotspot/normal/eclipse' (Join-Path $cache 'jdk21.zip')
        Expand-Archive $jz $tools -Force
    }
    if (-not (Get-ChildItem $tools -Directory -Filter 'ghidra_*' -ErrorAction SilentlyContinue)) {
        $rel = Invoke-RestMethod 'https://api.github.com/repos/NationalSecurityAgency/ghidra/releases/latest' -Headers @{ 'User-Agent' = $UA }
        $asset = $rel.assets | Where-Object name -like 'ghidra_*_PUBLIC_*.zip' | Select-Object -First 1
        $gz = Get-File $asset.browser_download_url (Join-Path $cache $asset.name)
        Expand-Archive $gz $tools -Force
    }
    Write-Host "  Ghidra and JDK in $tools"
}
Write-Host 'Dependencies ready.'
