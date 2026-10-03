// Shared memory link between the two games. Layout comes from generated/bridge_gen.h (sheets/bridge.csv).
#pragma once
#include <windows.h>
#include <cstring>
#include "../generated/bridge_gen.h"
#include "../generated/tuning_gen.h"

namespace bridge {
inline uint64_t total_bytes() { return B::kHeaderBytes + B::kSlots * B::slot_bytes(T::FRAME_MAX_PIXELS); }

// Either game may start first: both create-or-open the same named mapping.
inline B::Shared* open() {
    static B::Shared* s = nullptr;
    if (s) return s;
    const uint64_t n = total_bytes();
    HANDLE h = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, DWORD(n >> 32), DWORD(n), B::kMapping);
    if (!h) return nullptr;
    const bool fresh = GetLastError() != ERROR_ALREADY_EXISTS;
    s = static_cast<B::Shared*>(MapViewOfFile(h, FILE_MAP_ALL_ACCESS, 0, 0, SIZE_T(n)));
    if (!s) return nullptr;
    if (fresh || s->header.magic != B::kMagic || s->header.version != B::kVersion) {
        std::memset(s, 0, sizeof(B::Shared));
        s->header.version = B::kVersion;
        s->header.magic = B::kMagic;
    }
    return s;
}

inline uint8_t* slot(B::Shared* s, int i) {
    return reinterpret_cast<uint8_t*>(s) + B::kHeaderBytes + i * B::slot_bytes(T::FRAME_MAX_PIXELS);
}

inline bool alive(uint64_t heartbeat) { return heartbeat && GetTickCount64() - heartbeat < 2000; }

// Single-producer ring: the writer bumps head after filling the item; readers keep their own tail.
template <class Ring> void push(Ring& r, B::EventType t, const double p[3], float a = 0, float b = 0) {
    uint32_t h = r.head;
    B::Event& e = r.items[h % 64];
    e.type = t; e.seq = h; e.pos[0] = p[0]; e.pos[1] = p[1]; e.pos[2] = p[2]; e.a = a; e.b = b;
    MemoryBarrier();
    r.head = h + 1;
}
template <class Ring, class F> void drain(Ring& r, uint32_t& tail, F&& f) {
    uint32_t h = r.head;
    if (h - tail > 64) tail = h - 64;
    for (; tail != h; ++tail) f(r.items[tail % 64]);
}

// GTA (x east, y north, z up) <-> JC3 (y up). Origins come from tuning.csv.
// ASSUMPTION to verify in game: JC3 uses x east, y up, z south (right-handed, like most DX engines with y-up).
inline void gta_to_jc3(const float g[3], double j[3]) {
    j[0] = g[0] + T::JC3_ORIGIN_X; j[1] = g[2] + T::JC3_ORIGIN_Y; j[2] = -g[1] + T::JC3_ORIGIN_Z;
}
inline void gta_dir_to_jc3(const float g[3], float j[3]) { j[0] = g[0]; j[1] = g[2]; j[2] = -g[1]; }
inline void jc3_to_gta(const double j[3], float g[3]) {
    g[0] = float(j[0] - T::JC3_ORIGIN_X); g[1] = float(-(j[2] - T::JC3_ORIGIN_Z)); g[2] = float(j[1] - T::JC3_ORIGIN_Y);
}
}
