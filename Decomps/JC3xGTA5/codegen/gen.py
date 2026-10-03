"""Project the sheets into C++ headers shared by both sides. Never hand-edit generated/."""
import csv, os, zlib
H = os.path.dirname(os.path.abspath(__file__))
S, G = os.path.join(H, "..", "sheets"), os.path.join(H, "..", "generated")
rows = lambda n: list(csv.DictReader(open(os.path.join(S, n), newline="")))
os.makedirs(G, exist_ok=True)

def write(name, lines):
    open(os.path.join(G, name), "w", newline="\n").write("\n".join(lines) + "\n")

# GTA natives (gta side only)
out = ["// GENERATED from sheets/natives.csv", "#pragma once", "#include <nativeCaller.h>", "#include <types.h>", "namespace N {"]
for r in rows("natives.csv"):
    ps = [p.strip() for p in r["params"].split(";") if p.strip()]
    names = [p.split()[-1] for p in ps]
    out.append("inline %s %s(%s) { return invoke<%s>(%s%s); }" % (
        r["returns"], r["name"], ", ".join(ps), r["returns"], r["hash"], "".join(", " + n for n in names)))
out.append("}")
write("natives_gen.h", out)

# tunables + controls
out = ["// GENERATED from sheets/tuning.csv + controls.csv", "#pragma once", "namespace T {"]
for r in rows("tuning.csv"):
    t, v = r["type"], r["value"]
    if t == "bool": v = "true" if v == "1" else "false"
    elif t == "float": v += "f"
    out.append("constexpr %s %s = %s; // %s" % (t, r["key"], v, r["note"]))
for r in rows("controls.csv"):
    out.append("constexpr int CTRL_%s = %s; // %s" % (r["action"], r["control_id"], r["default_binding"]))
out.append("}")
write("tuning_gen.h", out)

# bridge layout + events (both sides)
bridge = rows("bridge.csv")
version = zlib.crc32(open(os.path.join(S, "bridge.csv"), "rb").read()) & 0xFFFF
out = ["// GENERATED from sheets/bridge.csv + events.csv", "#pragma once", "#include <cstdint>",
       "namespace B {", 'constexpr const wchar_t* kMapping = L"Local\\\\JC3xGTA5Bridge";',
       "constexpr uint32_t kMagic = 0x33434A47;", "constexpr uint32_t kVersion = %d;" % version,
       "constexpr int kSlots = 2;", "enum EventType : uint32_t {"]
out += ["  EV_%s = %s, // %s" % (e["name"], e["id"], e["note"]) for e in rows("events.csv")]
out += ["};", "struct Event { uint32_t type; uint32_t seq; double pos[3]; float a, b; };"]
sections = []
for r in bridge:
    if r["section"] not in sections: sections.append(r["section"])
for s in sections:
    out.append("struct %s_t {" % s.capitalize())
    for r in (x for x in bridge if x["section"] == s):
        n = int(r["count"])
        out.append("  %s %s%s; // %s: %s" % (r["type"], r["field"], "[%d]" % n if n > 1 else "", r["writer"], r["note"]))
    out.append("};")
out.append("struct alignas(64) Shared {")
out += ["  %s_t %s;" % (s.capitalize(), s) for s in sections]
out += ["};", "// pixel slots follow Shared: per slot width*height RGBA8 colour then float32 depth (metres)",
        "constexpr uint64_t kHeaderBytes = (sizeof(Shared) + 4095) & ~4095ull;",
        "inline uint64_t slot_bytes(uint64_t px) { return px * 8; }", "}"]
write("bridge_gen.h", out)

feats = rows("features.csv")
done = [r for r in feats if r["status"].startswith("implemented")]
print("generated (bridge v%d); coverage %d/%d features" % (version, len(done), len(feats)))
