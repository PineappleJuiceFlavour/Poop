@echo off
REM Builds both halves. Needs Visual Studio 2022 (C++ x64), CMake, Python 3 and git on PATH,
REM and the ScriptHookV SDK unzipped into gta\sdk (inc\ and lib\) from https://www.dev-c.com/gtav/scripthookv/
setlocal
cd /d "%~dp0"
if not exist gta\sdk\lib\ScriptHookV.lib (echo Missing gta\sdk: unzip the ScriptHookV SDK there first.& exit /b 1)
python codegen\gen.py || exit /b 1
cmake -S gta -B build\gta -A x64 && cmake --build build\gta --config Release || exit /b 1
cmake -S jc3 -B build\jc3 -A x64 && cmake --build build\jc3 --config Release || exit /b 1
echo Built build\gta\Release\JC3xGTA5.asi and build\jc3\Release\JC3xGTA5.addon64
