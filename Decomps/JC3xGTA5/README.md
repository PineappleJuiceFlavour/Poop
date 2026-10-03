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

## Filling `jc3_symbols.csv` (your Ghidra session)
JC3 has no script hook, so camera sync, Rico sync and explosions need addresses from **your own**
`JustCause3.exe`. Nothing from the game ships in this repo.
1. `set GHIDRA_HOME=C:\ghidra` then `Decomps\tools\run_ghidra.bat "<JC3 folder>\JustCause3.exe" JC3`.
   The output, `Decomps\local\JC3\`, holds functions/calls/structs/strings spreadsheets plus
   `jc3_candidates.csv`: RTTI classes named Camera/PlayerManager/Character/Explosion, the functions that use
   them, and a byte pattern for each.
2. For `function` and `rip_ptr` rows, paste a unique pattern into `pattern` (`??` = wildcard). For `rip_ptr`,
   `rip_offset` is where the 4-byte displacement starts inside the pattern.
3. For `offset` rows, put the hex offset (e.g. `0x1A0`) in `pattern`.
4. For `SpawnExplosion`, `extra_offset` is the index (0-3) of the argument that points at the position.
5. Check matrix layout with a memory viewer; set `JC3_MATRIX_LAYOUT` in `tuning.csv` if needed.

Any row left empty just switches its feature off (ReShade's log names what's missing). Frame export and
compositing need no symbols at all.

## Setup
1. Install ReShade 6.8+ **with full add-on support** into both games (DirectX 11). `play.bat` moves GTA's
   `dxgi.dll` to `ReShade64.asi`, because GTA loads the system dxgi first.
2. Install ScriptHookV (`ScriptHookV.dll`, `dinput8.dll`) into GTA 5, and unzip its SDK into `gta\sdk\`.
3. ReShade depth settings in **both** games: `RESHADE_DEPTH_INPUT_IS_REVERSED=1`,
   `RESHADE_DEPTH_LINEARIZATION_FAR_PLANE=1000`. Enable `JC3Export` in JC3 (last in the list) and
   `JC3Passthrough` in GTA.
4. Pick where Los Santos sits in Medici: `JC3_ORIGIN_X/Y/Z` in `tuning.csv`.
5. Run both windowed, with GTA's "Pause game on focus loss" off. Run `play.bat` from the repo root.

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
