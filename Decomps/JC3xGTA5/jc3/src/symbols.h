// Runtime resolver for sheets/jc3_symbols.csv. Rows are found by byte pattern in the running JustCause3.exe,
// so no game code or retail addresses ship with the mod: you fill the patterns from your own Ghidra session.
#pragma once
#include <cstdint>
#include <string>
#include <vector>

namespace sym {
enum Id { CameraManager, Camera_Transform, Camera_Active, Camera_Fov, CameraUpdate,
          PlayerManager, LocalCharacter, Character_Transform, SpawnExplosion, ExplosionCtor, Count };
struct Row { std::string name, kind, pattern, feature; int rip_offset = 0, extra = 0; uintptr_t value = 0; bool ok = false; };
extern Row rows[Count];
uint32_t load(const std::wstring& csv_path); // returns bitmask of resolved ids
inline bool ok(Id i) { return rows[i].ok; }
inline uintptr_t v(Id i) { return rows[i].value; }
template <class T> T* deref(uintptr_t base, Id off) { return reinterpret_cast<T*>(base + v(off)); }
bool all(std::initializer_list<Id> ids);
}
