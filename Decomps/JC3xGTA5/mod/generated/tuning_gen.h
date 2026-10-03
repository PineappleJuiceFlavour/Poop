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
constexpr int CHAOS_EXPLOSION_TYPE = 2; // explosion type on grapple+jump
constexpr float CHAOS_DAMAGE = 1.0f; // 
constexpr int CTRL_GRAPPLE_FIRE = 24; // LMB / RT
constexpr int CTRL_GRAPPLE_AIM = 25; // RMB / LT
constexpr int CTRL_WINGSUIT_TOGGLE = 22; // Space / A
constexpr int CTRL_CHAOS_DETONATE = 47; // G / DPad-Left
}
