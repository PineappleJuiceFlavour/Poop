@echo off
REM Usage: run_ghidra.bat "C:\path\to\JustCause3.exe" JC3
REM Downloads Ghidra and a JDK into Decomps\local\tools the first time if GHIDRA_HOME isn't set.
setlocal
if "%~2"=="" (echo Usage: %~nx0 ^<exe^> ^<name^> & exit /b 1)
set "TOOLS=%~dp0..\local\tools"
if "%GHIDRA_HOME%"=="" (
  if not exist "%TOOLS%\ghidra_*" powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0get_deps.ps1" -Ghidra || exit /b 1
  for /d %%G in ("%TOOLS%\ghidra_*") do set "GHIDRA_HOME=%%G"
)
for /d %%J in ("%TOOLS%\jdk-*") do set "JAVA_HOME=%%J"
if defined JAVA_HOME set "PATH=%JAVA_HOME%\bin;%PATH%"
set "OUT=%~dp0..\local\%~2"
mkdir "%OUT%" 2>nul
call "%GHIDRA_HOME%\support\analyzeHeadless.bat" "%OUT%\proj" %~2 -import "%~1" -overwrite ^
  -scriptPath "%~dp0ghidra" -postScript ExportSpreadsheets.py "%OUT%" -postScript FindJC3Symbols.py "%OUT%" -max-cpu 8
echo Spreadsheets written to %OUT%  (send me jc3_candidates.csv)
