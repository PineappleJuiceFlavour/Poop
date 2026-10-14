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
if not exist "%~dp0..\data" (
    echo Put the skate3_map_editor folder inside your game folder, next to skate3rust.exe and data.
    pause & exit /b 1
)

if not exist "%SOURCE%\.git" (
    echo Downloading the game's source code...
    git clone https://github.com/SK8-ENGINE/skate-3-rust-engine "%SOURCE%" || (pause & exit /b 1)
)
pushd "%SOURCE%"
if exist "crates\skate-map-editor\Cargo.toml" (
    echo The map editor is already added. Rebuilding...
) else (
    git checkout -q -f -B map-editor %GAME_COMMIT% || (popd & pause & exit /b 1)
    rem Applied to the index, so Windows line endings in the checkout don't matter.
    git apply --cached --whitespace=nowarn "%~dp0skate3-map-editor.patch" || (
        echo The patch did not apply. Delete the skate-3-rust-engine folder and run this again.
        popd & pause & exit /b 1
    )
    git -c user.name=map-editor -c user.email=map-editor@localhost commit -q -m "Add the map editor" || (popd & pause & exit /b 1)
    git reset -q --hard
)
call BUILD.bat || (popd & pause & exit /b 1)
popd
echo.
echo Done. Start the game with skate-3-rust-engine\PLAY.bat and press F1 for the map editor.
pause
