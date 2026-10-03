@echo off
REM Builds (if needed), installs JC3xGTA5.asi into GTA 5, and launches the game.
REM Set GTA5_DIR if your install isn't the default Steam path.
if "%GTA5_DIR%"=="" set "GTA5_DIR=C:\Program Files (x86)\Steam\steamapps\common\Grand Theft Auto V"
set "ASI=%~dp0Decomps\JC3xGTA5\mod\build\Release\JC3xGTA5.asi"
if not exist "%ASI%" call "%~dp0Decomps\JC3xGTA5\mod\build.bat" || (pause & exit /b 1)
if not exist "%GTA5_DIR%\ScriptHookV.dll" echo WARNING: ScriptHookV.dll + dinput8.dll not found in "%GTA5_DIR%".
copy /y "%ASI%" "%GTA5_DIR%\" >nul
if exist "%GTA5_DIR%\PlayGTAV.exe" (start "" "%GTA5_DIR%\PlayGTAV.exe") else (start "" steam://rungameid/271590)
