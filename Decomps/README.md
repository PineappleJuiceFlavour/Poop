# Decomps

Spreadsheet Method workspace. Spreadsheets (CSV) are the source of truth; code is generated from them.

## Layout
- `tools/ghidra/ExportSpreadsheets.py` — Ghidra headless script. Dumps functions, structs, enums, strings and xrefs from **your own local** game binary to CSV.
- `tools/run_ghidra.bat` — runs that export. Output goes to `Decomps/local/<name>/` (gitignored, so decompiled game data never gets committed).
- `JC3xGTA5/` — the Just Cause 3-style mod for GTA 5: sheets → codegen → ScriptHookV `.asi`.

## Rules (advice for agents)
1. Read the sheets, not the code. Every native, tunable and feature is one row.
2. Never invent an interface: if a native isn't in `natives.csv`, add the row first (with `verified=no`).
3. `coverage.csv` is the only answer to "what's left?" Update it when a feature changes.
4. Parallel agents each own whole rows or whole sheets. Never hand-edit `mod/generated/`.
