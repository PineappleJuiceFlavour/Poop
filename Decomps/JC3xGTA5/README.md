# JC3xGTA5: a Just Cause 3 × GTA 5 passthrough mod

Both real games run at the same time. GTA 5 (story mode) is the **host** you play in; Just Cause 3 is the
**guest**, rendered from GTA's camera and depth-composited into GTA's picture. Explosions cross over both ways.
On top of that, Rico's moves (grappling hook, wingsuit, infinite parachutes, C4-style detonation) are built
natively into GTA. Modelled on universal-modder's Minecraft × GTA V passthrough example.

```
GTA 5 (host)                                         Just Cause 3 (guest)
  JC3xGTA5.asi  (ScriptHookV + ReShade add-on)         JC3xGTA5.addon64 (ReShade add-on + MinHook)
    camera, player, GTA events  ---- shared memory "Local\JC3xGTA5Bridge" ---->  camera override, Rico sync
    JC3Passthrough.fx  <-------- JC3 colour + depth (JC3Export.fx), JC3 events --
```

## Everything is a spreadsheet
| sheet | what it drives |
|---|---|
| `sheets/bridge.csv` | the shared-memory layout both games use (generated into `generated/bridge_gen.h`) |
| `sheets/events.csv` | event types that cross between games |
| `sheets/natives.csv` | every GTA native the GTA half calls, by hash |
| `sheets/jc3_symbols.csv` | every JC3 address the JC3 half needs, as byte patterns found at runtime. **Read at launch, no rebuild needed** |
| `sheets/tuning.csv`, `controls.csv` | numbers and keys |
| `sheets/features.csv` | coverage: what's done, what's left, which symbols each feature needs |

`python codegen/gen.py` turns the sheets into `generated/`. Don't edit `generated/` by hand.

## JC3 addresses (`jc3_symbols.csv`)
JC3 has no script hook, so the JC3 half reads the game's memory directly. The addresses come from open-source
JC3 mods, for the final Steam build (1.05, also the current Denuvo-free build):
- **Camera:** `CCameraManager` at `0x142ED0E20`, active camera `+0x5C0`, fov `+0x580` (radians), flags `+0x55E`.
  From [BakuStorm/JC3FOVFixer](https://github.com/BakuStorm/JC3FOVFixer) and
  [Mrsuss60/JustC3_FOVChanger](https://github.com/Mrsuss60/JustC3_FOVChanger).
- **Rico:** `CNetworkPlayerManager` at `0x142F36958` → `+0x48` local player → `+0x138` character →
  `+0x2830` world matrix. From [aaronkirkham/jc3-console-thingy](https://github.com/aaronkirkham/jc3-console-thingy).
- **Version check:** `0x142305658` must read `Aval`, or every absolute address is switched off (other builds).
- **Camera matrix:** not public anywhere, so the add-on finds it itself. It scans the active camera for 4x4
  matrices near Rico and logs each offset to `ReShade.log` ("camera matrix candidate at +0x..."). It then
  overwrites them from a background thread, which can flicker. To pin it, put a logged offset in the
  `Camera_Transform` row (`kind` = `offset`). Filling `CameraUpdate` with that function's pattern
  removes the flicker.
- **Explosions crossing over:** need `SpawnExplosion`, which no public source has. Until it's filled
  (from your Ghidra project: `tools\export_existing_project.bat`), explosions stay in GTA only.

Rows are read at launch, so editing the sheet needs no rebuild. Any missing row switches its feature off, and
ReShade's log names it.

## Setup
1. Install Visual Studio 2022 (C++ desktop), CMake, Python 3 and Git.
2. Run `play.bat` from the repo root. The first run downloads ScriptHookV, its SDK and ReShade (add-on build)
   from their official sites and installs them: the SDK into `gta\sdk\`, ScriptHookV + ReShade (as
   `ReShade64.asi`) into GTA 5, ReShade (as `dxgi.dll`) into JC3. It also writes `ReShade.ini` and
   `ReShadePreset.ini` with the depth settings and the right effect turned on (only if you have none).
   Then it builds, installs and launches both games.
3. Pick where Los Santos sits in Medici: `JC3_ORIGIN_X/Y/Z` in `tuning.csv`.
4. Run both windowed, with GTA's "Pause game on focus loss" off.

`Decomps\tools\run_ghidra.bat` likewise downloads Ghidra and a JDK into `Decomps\local\tools` the first time.

## Controls (in GTA)
| key | action |
|---|---|
| F7 | passthrough on/off |
| F8 | Rico mirror: Rico stands in for your GTA character (GTA ped hidden) |
| RMB + LMB | grappling hook (also sent to JC3) |
| Space in the air | wingsuit |
| G | detonate at the hook point, in both games |

## Safety
Story mode only. `play.bat` writes `-nobattleye` to `args.txt`; never take a modded GTA online.
JC3 is single-player only here. The bridge is local shared memory with no network.
