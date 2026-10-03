// GENERATED from sheets/bridge.csv + events.csv
#pragma once
#include <cstdint>
namespace B {
constexpr const wchar_t* kMapping = L"Local\\JC3xGTA5Bridge";
constexpr uint32_t kMagic = 0x33434A47;
constexpr uint32_t kVersion = 50627;
constexpr int kSlots = 2;
enum EventType : uint32_t {
  EV_EXPLOSION = 1, // an explosion at pos
  EV_BULLET_HIT = 2, // a JC3 bullet landed at pos
  EV_VEHICLE_DESTROYED = 3, // 
  EV_TELEPORT = 4, // move Rico to pos
  EV_GRAPPLE_HIT = 5, // GTA grapple anchor point
};
struct Event { uint32_t type; uint32_t seq; double pos[3]; float a, b; };
struct Header_t {
  uint32_t magic; // both: 0x33434A47 'GJC3'
  uint32_t version; // both: bumped by gen.py when this sheet changes
  uint64_t gta_heartbeat; // gta: GetTickCount64 of last GTA write
  uint64_t jc3_heartbeat; // jc3: GetTickCount64 of last JC3 write
  uint32_t jc3_symbols_ok; // jc3: bitmask of resolved rows in jc3_symbols.csv
};
struct Cam_t {
  uint64_t seq; // gta: odd while writing
  double pos[3]; // gta: GTA camera position in JC3 space
  float fwd[3]; // gta: camera forward unit vector in JC3 space
  float up[3]; // gta: camera up unit vector in JC3 space
  float fov; // gta: vertical fov degrees
};
struct Player_t {
  double pos[3]; // gta: GTA player feet in JC3 space
  float fwd[3]; // gta: player facing in JC3 space
  uint32_t flags; // gta: 1=active 2=rico_mirror 4=hide_gta_ped
};
struct Rico_t {
  double pos[3]; // jc3: Rico position in JC3 space
  uint32_t flags; // jc3: 1=valid
};
struct Frame_t {
  uint64_t publish; // jc3: increments per published slot
  int32_t slot; // jc3: newest complete slot
  uint32_t width; // jc3: 
  uint32_t height; // jc3: 
  float near; // jc3: 
  float far; // jc3: 
  double pose_pos[3]; // jc3: camera the frame was rendered with
  float pose_fwd[3]; // jc3: 
  float pose_up[3]; // jc3: 
  float pose_fov; // jc3: 
};
struct Ev_jc3_t {
  uint32_t head; // jc3: ring write index (events JC3 -> GTA)
  Event items[64]; // jc3: 
};
struct Ev_gta_t {
  uint32_t head; // gta: ring write index (events GTA -> JC3)
  Event items[64]; // gta: 
};
struct alignas(64) Shared {
  Header_t header;
  Cam_t cam;
  Player_t player;
  Rico_t rico;
  Frame_t frame;
  Ev_jc3_t ev_jc3;
  Ev_gta_t ev_gta;
};
// pixel slots follow Shared: per slot width*height RGBA8 colour then float32 depth (metres)
constexpr uint64_t kHeaderBytes = (sizeof(Shared) + 4095) & ~4095ull;
inline uint64_t slot_bytes(uint64_t px) { return px * 8; }
}
