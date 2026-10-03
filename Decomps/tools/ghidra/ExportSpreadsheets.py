# Ghidra headless post-script: export the program as relational CSV sheets.
# @category Export
import csv, os
from ghidra.program.model.data import Structure, Enum

out = getScriptArgs()[0] if getScriptArgs() else os.getcwd()
if not os.path.isdir(out):
    os.makedirs(out)
prog = currentProgram
fm, rm, dtm = prog.getFunctionManager(), prog.getReferenceManager(), prog.getDataTypeManager()

def sheet(name, header):
    f = open(os.path.join(out, name), "wb")
    w = csv.writer(f)
    w.writerow(header)
    return f, w

f, w = sheet("functions.csv", ["id", "address", "name", "signature", "size", "calling_conv", "params", "status"])
for i, fn in enumerate(fm.getFunctions(True)):
    if monitor.isCancelled(): break
    w.writerow([i, fn.getEntryPoint(), fn.getName(), fn.getPrototypeString(False, False),
                fn.getBody().getNumAddresses(), fn.getCallingConventionName(),
                fn.getParameterCount(), "unimplemented"])
f.close()

f, w = sheet("calls.csv", ["caller", "callee"])
for fn in fm.getFunctions(True):
    if monitor.isCancelled(): break
    for c in fn.getCalledFunctions(monitor):
        w.writerow([fn.getEntryPoint(), c.getEntryPoint()])
f.close()

fs, ws = sheet("structs.csv", ["struct", "offset", "field", "type", "size"])
fe, we = sheet("enums.csv", ["enum", "name", "value"])
for dt in dtm.getAllDataTypes():
    if isinstance(dt, Structure):
        for c in dt.getComponents():
            ws.writerow([dt.getPathName(), c.getOffset(), c.getFieldName(), c.getDataType().getName(), c.getLength()])
    elif isinstance(dt, Enum):
        for n in dt.getNames():
            we.writerow([dt.getPathName(), n, dt.getValue(n)])
fs.close(); fe.close()

f, w = sheet("strings.csv", ["address", "value", "xref_count"])
for d in prog.getListing().getDefinedData(True):
    if d.hasStringValue():
        w.writerow([d.getAddress(), unicode(d.getValue()).encode("utf-8"), rm.getReferenceCountTo(d.getAddress())])
f.close()
print("Exported sheets to " + out)
