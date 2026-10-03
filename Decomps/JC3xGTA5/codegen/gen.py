"""Project the sheets into C++. Never hand-edit mod/generated/."""
import csv, os
H = os.path.dirname(os.path.abspath(__file__))
S, G = os.path.join(H, "..", "sheets"), os.path.join(H, "..", "mod", "generated")
rows = lambda n: list(csv.DictReader(open(os.path.join(S, n), newline="")))
os.makedirs(G, exist_ok=True)

out = ["// GENERATED from sheets/natives.csv", "#pragma once", '#include "nativeCaller.h"', '#include "types.h"', "namespace N {"]
for r in rows("natives.csv"):
    ps = [p.strip() for p in r["params"].split(";") if p.strip()]
    names = [p.split()[-1] for p in ps]
    out.append("inline %s %s(%s) { return invoke<%s>(%s%s); }" % (
        r["returns"], r["name"], ", ".join(ps), r["returns"], r["hash"], "".join(", " + n for n in names)))
out.append("}")
open(os.path.join(G, "natives_gen.h"), "w").write("\n".join(out) + "\n")

ty = {"float": "float", "int": "int", "bool": "bool"}
out = ["// GENERATED from sheets/tuning.csv + controls.csv", "#pragma once", "namespace T {"]
for r in rows("tuning.csv"):
    v = r["value"] + ("f" if r["type"] == "float" else "")
    if r["type"] == "bool": v = "true" if r["value"] == "1" else "false"
    out.append("constexpr %s %s = %s; // %s" % (ty[r["type"]], r["key"], v, r["note"]))
for r in rows("controls.csv"):
    out.append("constexpr int CTRL_%s = %s; // %s" % (r["action"], r["control_id"], r["default_binding"]))
out.append("}")
open(os.path.join(G, "tuning_gen.h"), "w").write("\n".join(out) + "\n")

done = [r for r in rows("features.csv") if r["status"] == "implemented"]
print("generated; coverage %d/%d features" % (len(done), len(rows("features.csv"))))
