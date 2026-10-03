@echo off
REM Runs the spreadsheet + symbol-candidate scripts on a Ghidra project you ALREADY analyzed (no re-analysis).
REM Usage: export_existing_project.bat "C:\path\to\ghidra_projects" JC3_JustCause3
REM   (the folder holding JC3_JustCause3.gpr, and the project name without .gpr)
setlocal
if "%~2"=="" (echo Usage: %~nx0 ^<project folder^> ^<project name^> & exit /b 1)
set "TOOLS=%~dp0..\local\tools"
if "%GHIDRA_HOME%"=="" (
  if not exist "%TOOLS%\ghidra_*" powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0get_deps.ps1" -Ghidra || exit /b 1
  for /d %%G in ("%TOOLS%\ghidra_*") do set "GHIDRA_HOME=%%G"
)
for /d %%J in ("%TOOLS%\jdk-*") do set "JAVA_HOME=%%J"
if defined JAVA_HOME set "PATH=%JAVA_HOME%\bin;%PATH%"
set "OUT=%~dp0..\local\JC3"
mkdir "%OUT%" 2>nul
call "%GHIDRA_HOME%\support\analyzeHeadless.bat" "%~1" %~2 -process -noanalysis -readOnly ^
  -scriptPath "%~dp0ghidra" -postScript ExportSpreadsheets.py "%OUT%" -postScript FindJC3Symbols.py "%OUT%"
echo.
echo Done. Attach this ONE file in the chat (don't commit it): %OUT%\jc3_candidates.csv
