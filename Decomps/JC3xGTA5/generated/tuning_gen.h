// GENERATED from sheets/tuning.csv + controls.csv
#pragma once
namespace T {
constexpr float GRAPPLE_RANGE = 100.0f; // max hook distance (m)
constexpr float GRAPPLE_PULL = 38.0f; // pull speed (m/s)
constexpr float GRAPPLE_STOP_DIST = 2.5f; // release distance (m)
constexpr float WINGSUIT_GLIDE = 0.22f; // forward lift per tick
constexpr float WINGSUIT_SINK = -6.0f; // max fall speed while gliding
constexpr float WINGSUIT_SPEED = 42.0f; // glide speed (m/s)
constexpr float CHUTE_BOOST = 20.0f; // grapple-into-chute boost (m/s)
constexpr bool INFINITE_CHUTES = true; // re-give parachute every tick
constexpr int CHAOS_EXPLOSION_TYPE = 2; // explosion type on detonate / mirrored JC3 explosions
constexpr float CHAOS_DAMAGE = 1.0f; // 
constexpr double JC3_ORIGIN_X = 0.0; // JC3 x for GTA (0 0 0); pick a spot in Medici and set these
constexpr double JC3_ORIGIN_Y = 0.0; // JC3 height (y-up) for GTA z=0
constexpr double JC3_ORIGIN_Z = 0.0; // JC3 z for GTA (0 0 0)
constexpr float COMPOSITE_MAX_DIST = 80.0f; // JC3 pixels farther than this (m) are dropped so JC3 terrain doesn't cover Los Santos
constexpr int FRAME_MAX_PIXELS = 2073600; // cap on exported frame size (1080p)
constexpr bool PASSTHROUGH_START_ON = true; // start with the passthrough on
constexpr int JC3_MATRIX_LAYOUT = 0; // JC3 matrix rows: 0 = right,up,back,pos  1 = right,up,forward,pos (check in a memory viewer)
constexpr int CTRL_GRAPPLE_FIRE = 24; // LMB / RT
constexpr int CTRL_GRAPPLE_AIM = 25; // RMB / LT
constexpr int CTRL_WINGSUIT_TOGGLE = 22; // Space / A
constexpr int CTRL_CHAOS_DETONATE = 47; // G / DPad-Left
constexpr int CTRL_PASSTHROUGH_TOGGLE = 0x76; // F7
constexpr int CTRL_RICO_MIRROR_TOGGLE = 0x77; // F8
}
