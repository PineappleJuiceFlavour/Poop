@echo off
REM Usage: run_ghidra.bat "C:\path\to\GTA5.exe" GTA5
REM Needs GHIDRA_HOME set to your Ghidra install.
if "%~2"=="" (echo Usage: %~nx0 ^<exe^> ^<name^> & exit /b 1)
set OUT=%~dp0..\local\%~2
mkdir "%OUT%" 2>nul
"%GHIDRA_HOME%\support\analyzeHeadless.bat" "%OUT%\proj" %~2 -import "%~1" -overwrite ^
  -scriptPath "%~dp0ghidra" -postScript ExportSpreadsheets.py "%OUT%" -postScript FindJC3Symbols.py "%OUT%" -max-cpu 8
echo Spreadsheets written to %OUT%
