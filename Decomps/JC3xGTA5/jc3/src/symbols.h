// Runtime resolver for sheets/jc3_symbols.csv. Rows are absolute addresses from public JC3 mods (checked
// against a version marker), offsets, or byte patterns found in the running JustCause3.exe.
#pragma once
#include <cstdint>
#include <initializer_list>
#include <string>

namespace sym {
enum Id { CameraManager, Camera_Transform, Camera_Active, Camera_Fov, CameraUpdate, PlayerManager, LocalPlayer,
          LocalCharacter, Character_Transform, SpawnExplosion, ExplosionCtor, VersionCheck, Camera_Flags, Count };
struct Row { std::string name, kind, pattern, feature; int rip_offset = 0; long long extra = 0; uintptr_t value = 0; bool ok = false; };
extern Row rows[Count];
uint32_t load(const std::wstring& csv_path); // returns bitmask of resolved ids
inline bool ok(Id i) { return rows[i].ok; }
inline uintptr_t v(Id i) { return rows[i].value; }
bool all(std::initializer_list<Id> ids);
bool readable(uintptr_t p, size_t n);
}
