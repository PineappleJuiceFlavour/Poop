#include "symbols.h"
#include <windows.h>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <reshade.hpp>

namespace sym {
Row rows[Count];

static std::vector<std::string> split_csv(const std::string& line) {
    std::vector<std::string> out; std::string cur; bool q = false;
    for (char c : line) {
        if (c == '"') q = !q;
        else if (c == ',' && !q) { out.push_back(cur); cur.clear(); }
        else cur += c;
    }
    out.push_back(cur);
    return out;
}

static uintptr_t scan(const std::string& pat) {
    std::vector<int> bytes; std::istringstream ss(pat); std::string tok;
    while (ss >> tok) bytes.push_back(tok[0] == '?' ? -1 : std::stoi(tok, nullptr, 16));
    if (bytes.empty()) return 0;
    auto base = reinterpret_cast<uint8_t*>(GetModuleHandleW(nullptr));
    auto nt = reinterpret_cast<IMAGE_NT_HEADERS*>(base + reinterpret_cast<IMAGE_DOS_HEADER*>(base)->e_lfanew);
    const size_t size = nt->OptionalHeader.SizeOfImage, n = bytes.size();
    for (size_t i = 0; i + n <= size; ++i) {
        size_t k = 0;
        while (k < n && (bytes[k] < 0 || base[i + k] == bytes[k])) ++k;
        if (k == n) return reinterpret_cast<uintptr_t>(base + i);
    }
    return 0;
}

bool readable(uintptr_t p, size_t n) {
    if (p < 0x10000) return false;
    MEMORY_BASIC_INFORMATION mbi;
    if (!VirtualQuery(reinterpret_cast<void*>(p), &mbi, sizeof mbi) || mbi.State != MEM_COMMIT) return false;
    if (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)) return false;
    return p + n <= reinterpret_cast<uintptr_t>(mbi.BaseAddress) + mbi.RegionSize;
}

uint32_t load(const std::wstring& path) {
    std::ifstream f(path);
    std::string line; std::getline(f, line); // header
    uint32_t mask = 0;
    while (std::getline(f, line)) {
        auto c = split_csv(line);
        if (c.size() < 8) continue;
        int id = std::atoi(c[0].c_str());
        if (id < 0 || id >= Count) continue;
        Row& r = rows[id];
        r.name = c[1]; r.kind = c[2]; r.pattern = c[3]; r.feature = c[6];
        r.rip_offset = std::atoi(c[4].c_str()); r.extra = std::strtoll(c[5].c_str(), nullptr, 0);
        if (r.kind == "auto") {
            continue; // resolved later by the add-on
        } else if (r.kind == "abs") {
            if (!r.pattern.empty()) { r.value = std::stoull(r.pattern, nullptr, 16); r.ok = readable(r.value, 8); }
        } else if (r.kind == "offset") {
            // pattern column holds the hex offset for plain offsets
            if (!r.pattern.empty()) { r.value = std::stoull(r.pattern, nullptr, 16); r.ok = true; }
        } else if (uintptr_t hit = scan(r.pattern)) {
            if (r.kind == "rip_ptr") { // mov reg,[rip+disp32]: address = next instruction + disp32
                int32_t disp = *reinterpret_cast<int32_t*>(hit + r.rip_offset);
                r.value = hit + r.rip_offset + 4 + disp;
            } else {
                r.value = hit + r.rip_offset;
            }
            r.ok = true;
        }
        if (id == VersionCheck && r.ok && *reinterpret_cast<uint32_t*>(r.value) != uint32_t(r.extra)) r.ok = false;
        std::string msg = "JC3xGTA5: symbol " + r.name + (r.ok ? " resolved" : " MISSING (fill sheets/jc3_symbols.csv)");
        reshade::log::message(r.ok ? reshade::log::level::info : reshade::log::level::warning, msg.c_str());
        if (r.ok) mask |= 1u << id;
    }
    // absolute addresses only hold for the build the version marker identifies
    if (!rows[VersionCheck].ok)
        for (Row& r : rows)
            if (r.kind == "abs" && r.ok) { r.ok = false; mask &= ~(1u << (&r - rows)); }
    if (!rows[VersionCheck].ok)
        reshade::log::message(reshade::log::level::warning, "JC3xGTA5: unknown JustCause3.exe build, absolute addresses disabled");
    return mask;
}

bool all(std::initializer_list<Id> ids) {
    for (Id i : ids) if (!rows[i].ok) return false;
    return true;
}
}
