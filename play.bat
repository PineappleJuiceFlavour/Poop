@echo off
REM JC3 x GTA 5 passthrough launcher: builds if needed, installs both halves, starts Just Cause 3 then GTA 5.
REM Set JC3_DIR / GTA5_DIR if your games aren't in the default Steam folder.
REM The first run downloads ScriptHookV (+SDK) and ReShade and installs them into both games for you.
setlocal
if "%JC3_DIR%"=="" set "JC3_DIR=C:\Program Files (x86)\Steam\steamapps\common\Just Cause 3"
if "%GTA5_DIR%"=="" set "GTA5_DIR=C:\Program Files (x86)\Steam\steamapps\common\Grand Theft Auto V"
set "M=%~dp0Decomps\JC3xGTA5"
if not exist "%JC3_DIR%\JustCause3.exe" (echo JC3 not found at "%JC3_DIR%". Set JC3_DIR.& pause & exit /b 1)
if not exist "%GTA5_DIR%\GTA5.exe" (echo GTA 5 not found at "%GTA5_DIR%". Set GTA5_DIR.& pause & exit /b 1)
if not exist "%~dp0Decomps\JC3xGTA5\gta\sdk\lib\ScriptHookV.lib" set NEED_DEPS=1
if not exist "%GTA5_DIR%\ScriptHookV.dll" set NEED_DEPS=1
if not exist "%JC3_DIR%\dxgi.dll" set NEED_DEPS=1
if defined NEED_DEPS powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Decomps\tools\get_deps.ps1" -Jc3Dir "%JC3_DIR%" -GtaDir "%GTA5_DIR%" || (pause & exit /b 1)
set "ASI=%M%\build\gta\Release\JC3xGTA5.asi"
set "ADDON=%M%\build\jc3\Release\JC3xGTA5.addon64"
if not exist "%ASI%" call "%M%\build.bat" || (pause & exit /b 1)
if not exist "%ADDON%" call "%M%\build.bat" || (pause & exit /b 1)

REM --- Just Cause 3 half (guest) ---
copy /y "%ADDON%" "%JC3_DIR%\" >nul
copy /y "%M%\sheets\jc3_symbols.csv" "%JC3_DIR%\" >nul
if not exist "%JC3_DIR%\reshade-shaders\Shaders" mkdir "%JC3_DIR%\reshade-shaders\Shaders"
copy /y "%M%\jc3\shaders\JC3Export.fx" "%JC3_DIR%\reshade-shaders\Shaders\" >nul

REM --- GTA 5 half (host) ---
REM GTA loads the system dxgi.dll first, so ReShade must load as an ASI there.
if exist "%GTA5_DIR%\dxgi.dll" if not exist "%GTA5_DIR%\ReShade64.asi" move /y "%GTA5_DIR%\dxgi.dll" "%GTA5_DIR%\ReShade64.asi" >nul
copy /y "%ASI%" "%GTA5_DIR%\" >nul
if not exist "%GTA5_DIR%\reshade-shaders\Shaders" mkdir "%GTA5_DIR%\reshade-shaders\Shaders"
copy /y "%M%\gta\shaders\JC3Passthrough.fx" "%GTA5_DIR%\reshade-shaders\Shaders\" >nul
if not exist "%GTA5_DIR%\args.txt" echo -nobattleye> "%GTA5_DIR%\args.txt"

echo Starting Just Cause 3...
start "" steam://rungameid/225540
timeout /t 20 /nobreak >nul
echo Starting GTA 5 (pick Story Mode yourself, never Online)...
start "" steam://rungameid/271590
