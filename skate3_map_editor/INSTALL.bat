@echo off
setlocal
rem Builds Skate 3 Rust Engine (experimental build 49) with the map editor.
rem Put this folder inside your game folder (next to skate3rust.exe and data) and run it.
cd /d "%~dp0"

set "GAME_COMMIT=2e16697797c8358d16f747a8259354ff2921041a"
set "SOURCE=%~dp0..\skate-3-rust-engine"

where git >nul 2>nul || (
    echo Git is not installed. Install it from https://git-scm.com/download/win and run this again.
    pause & exit /b 1
)
if not exist "%ProgramFiles%\LLVM\bin\llvm-readobj.exe" (
    echo The game's build needs LLVM. Installing it, accept the Windows prompt...
    where winget >nul 2>nul && (
        winget install -e --id LLVM.LLVM --accept-package-agreements --accept-source-agreements
    ) || (
        rem No winget: fetch the official installer from LLVM's GitHub releases and run it silently.
        powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol = 'Tls12'; $ProgressPreference = 'SilentlyContinue'; $release = Invoke-RestMethod 'https://api.github.com/repos/llvm/llvm-project/releases/latest'; $asset = $release.assets | Where-Object { $_.name -like 'LLVM-*-win64.exe' } | Select-Object -First 1; $file = Join-Path $env:TEMP $asset.name; Write-Host ('Downloading ' + $asset.name); Invoke-WebRequest $asset.browser_download_url -OutFile $file; Start-Process $file -ArgumentList '/S' -Verb RunAs -Wait"
    )
    if not exist "%ProgramFiles%\LLVM\bin\llvm-readobj.exe" (
        echo Install LLVM from https://github.com/llvm/llvm-project/releases in its default folder, then run this again.
        pause & exit /b 1
    )
)
if not exist "%~dp0..\data" (
    echo Put the skate3_map_editor folder inside your game folder, next to skate3rust.exe and data.
    pause & exit /b 1
)

if not exist "%SOURCE%\.git" (
    echo Downloading the game's source code...
    git clone https://github.com/SK8-ENGINE/skate-3-rust-engine "%SOURCE%" || (pause & exit /b 1)
)
pushd "%SOURCE%"
rem Start from the game's build 49 and add the latest editor patch (every run, so updates apply).
git checkout -q -f -B map-editor %GAME_COMMIT% || (popd & pause & exit /b 1)
rem Applied to the index, so Windows line endings in the checkout don't matter.
git apply --cached --whitespace=nowarn "%~dp0skate3-map-editor.patch" || (
    echo The patch did not apply. Delete the skate-3-rust-engine folder and run this again.
    popd & pause & exit /b 1
)
git -c user.name=map-editor -c user.email=map-editor@localhost commit -q -m "Add the map editor" || (popd & pause & exit /b 1)
git reset -q --hard
rem MSBuild cannot write paths over 260 characters, and this folder can be deep
rem (Downloads\...\skate-3-rust-engine\target\...), so compile in a short folder.
rem The finished game is still copied to skate-3-rust-engine\bin for PLAY.bat.
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "scripts\Build.ps1" -TargetDirectory "%LOCALAPPDATA%\sk8t" || (
    echo Build failed; see the messages above.
    popd & pause & exit /b 1
)
popd
echo.
echo Done. Start the game with skate-3-rust-engine\PLAY.bat and press F1 for the map editor.
pause
