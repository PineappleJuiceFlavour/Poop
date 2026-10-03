@echo off
rem Exports the JC3 models the Bevy port and RicoKit use (glTF + PNG) from your unpacked game files.
rem Output stays here (out\); never copy it into a repo or share it.
setlocal
set "HERE=%~dp0"
set "F=C:\Users\Admin\Downloads\JCtools\files"
set "E=%F%\editor\entities"
set "M=%E%\jc_environments\structures\military"
if not exist "%HERE%bin\jc3export.exe" dotnet build "%HERE%tool" -c Release -o "%HERE%bin" || exit /b 1
"%HERE%bin\jc3export.exe" "%F%" "%HERE%out\weapons" "%E%\jc_weapons"
"%HERE%bin\jc3export.exe" "%F%" "%HERE%out\gear" "%E%\jc_props\grappling_device" "%E%\gameobjects\grapplinghookwire.ee" "%E%\jc_characters\previewer\rico_preview.ee" "%E%\dlc\wingsuit_skins" "%E%\dlc\parachute_skins" "%E%\dlc\rico_skins"
"%HERE%bin\jc3export.exe" "%F%" "%HERE%out\props" "%M%\elements\fuel_barrel.ee" "%M%\elements\antennas\antenna_base.ee" "%M%\propaganda\propaganda_trailer.ee" "%F%\models\jc_environments\structures\military\radar_dish\radar_dish_hero_dish_destruct_lod1.rbm"
