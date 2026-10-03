# Ghidra script: candidate rows for JC3xGTA5/sheets/jc3_symbols.csv, from your own JustCause3.exe.
# Lists MSVC RTTI type names that look camera/player/explosion related, the vftables that use them,
# and the functions referencing them, with a ready-to-paste unique byte pattern for each function.
# Run after ExportSpreadsheets.py (same headless command, -postScript FindJC3Symbols.py <outdir>).
# @category Export
import csv, os
from ghidra.program.model.mem import MemoryAccessException

out = getScriptArgs()[0] if getScriptArgs() else os.getcwd()
prog, mem = currentProgram, currentProgram.getMemory()
listing, refs, fm = prog.getListing(), prog.getReferenceManager(), prog.getFunctionManager()
KEYS = ["Camera", "PlayerManager", "Character", "Explosion"]

def pattern(fn, n=24):
    """First n bytes of fn with rel32 operands wildcarded; grown until unique in .text."""
    addr = fn.getEntryPoint()
    toks, a = [], addr
    while len(toks) < n:
        ins = listing.getInstructionAt(a)
        if ins is None: break
        b = [("%02X" % (x & 0xFF)) for x in ins.getBytes()]
        if ins.getLength() >= 5 and (ins.getFlowType().isCall() or ins.getFlowType().isJump() or "[RIP" in str(ins).upper() or "0x" in str(ins)):
            b = b[:-4] + ["??"] * 4
        toks += b
        a = ins.getMaxAddress().next()
    return " ".join(toks[:max(n, len(toks))])

f = open(os.path.join(out, "jc3_candidates.csv"), "wb")
w = csv.writer(f)
w.writerow(["rtti_name", "ref_function", "function_address", "pattern"])
for d in listing.getDefinedData(True):
    if not d.hasStringValue(): continue
    s = unicode(d.getValue())
    if not s.startswith(".?AV") or not any(k in s for k in KEYS): continue
    seen = 0
    for r in refs.getReferencesTo(d.getAddress()):
        fn = fm.getFunctionContaining(r.getFromAddress())
        w.writerow([s, fn.getName() if fn else "", fn.getEntryPoint() if fn else r.getFromAddress(),
                    pattern(fn) if fn else ""])
        seen += 1
        if seen > 20: break
f.close()
print("Wrote jc3_candidates.csv to " + out)
