// GENERATED from sheets/natives.csv
#pragma once
#include <nativeCaller.h>
#include <types.h>
namespace N {
inline Ped PLAYER_PED_ID() { return invoke<Ped>(0xD80958FC74E988A6); }
inline Vector3 GET_ENTITY_COORDS(Entity e, BOOL alive) { return invoke<Vector3>(0x3FEF770D40960D5A, e, alive); }
inline Vector3 GET_ENTITY_VELOCITY(Entity e) { return invoke<Vector3>(0x4805D2B1D8CF94A9, e); }
inline void SET_ENTITY_VELOCITY(Entity e, float x, float y, float z) { return invoke<void>(0x1C99BB7B6E96D16F, e, x, y, z); }
inline void APPLY_FORCE_TO_ENTITY(Entity e, int type, float x, float y, float z, float ox, float oy, float oz, int bone, BOOL rel, BOOL ignoreUp, BOOL isForce, BOOL p12, BOOL p13) { return invoke<void>(0xC5F68BE9613E2D18, e, type, x, y, z, ox, oy, oz, bone, rel, ignoreUp, isForce, p12, p13); }
inline BOOL IS_ENTITY_IN_AIR(Entity e) { return invoke<BOOL>(0x886E37EC497200B6, e); }
inline Vector3 GET_GAMEPLAY_CAM_COORD() { return invoke<Vector3>(0x14D6F5678D8F1B37); }
inline Vector3 GET_GAMEPLAY_CAM_ROT(int order) { return invoke<Vector3>(0x837765A25378F0BB, order); }
inline int START_EXPENSIVE_SYNCHRONOUS_SHAPE_TEST_LOS_PROBE(float x1, float y1, float z1, float x2, float y2, float z2, int flags, Entity ignore, int p8) { return invoke<int>(0x377906D8A31E5586, x1, y1, z1, x2, y2, z2, flags, ignore, p8); }
inline int GET_SHAPE_TEST_RESULT(int handle, BOOL* hit, Vector3* end, Vector3* normal, Entity* ent) { return invoke<int>(0x3D87450E15D98694, handle, hit, end, normal, ent); }
inline BOOL IS_CONTROL_PRESSED(int pad, int control) { return invoke<BOOL>(0xF3A21BCD95725A4A, pad, control); }
inline BOOL IS_CONTROL_JUST_PRESSED(int pad, int control) { return invoke<BOOL>(0x580417101DDB492F, pad, control); }
inline int GET_PED_PARACHUTE_STATE(Ped p) { return invoke<int>(0x79CFD9827CC979B6, p); }
inline void GIVE_WEAPON_TO_PED(Ped p, Hash w, int ammo, BOOL hidden, BOOL equip) { return invoke<void>(0xBF0FD6E56C964FCB, p, w, ammo, hidden, equip); }
inline void ADD_EXPLOSION(float x, float y, float z, int type, float dmg, BOOL audible, BOOL invisible, float shake, BOOL noDamage) { return invoke<void>(0xE3AD2BDBAEE269AC, x, y, z, type, dmg, audible, invisible, shake, noDamage); }
inline void DRAW_LINE(float x1, float y1, float z1, float x2, float y2, float z2, int r, int g, int b, int a) { return invoke<void>(0x6B7256074AE34680, x1, y1, z1, x2, y2, z2, r, g, b, a); }
inline Vector3 GET_FINAL_RENDERED_CAM_COORD() { return invoke<Vector3>(0xA200EB1EE790F448); }
inline Vector3 GET_FINAL_RENDERED_CAM_ROT(int order) { return invoke<Vector3>(0x5B4E4C817FCC2DFB, order); }
inline float GET_FINAL_RENDERED_CAM_FOV() { return invoke<float>(0x80EC114669DAEFF4); }
inline Vector3 GET_ENTITY_FORWARD_VECTOR(Entity e) { return invoke<Vector3>(0x0A794A5A57F8DF91, e); }
inline void SET_ENTITY_VISIBLE(Entity e, BOOL visible, BOOL p2) { return invoke<void>(0xEA1C610A04DB6BBB, e, visible, p2); }
}
