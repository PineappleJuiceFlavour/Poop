@echo off
REM Put the ScriptHookV SDK (inc\ and lib\) in mod\sdk first.
cd /d "%~dp0"
cmake -S . -B build -A x64 || exit /b 1
cmake --build build --config Release || exit /b 1
echo Built build\Release\JC3xGTA5.asi
