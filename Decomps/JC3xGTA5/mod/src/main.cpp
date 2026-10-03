// JC3xGTA5 runtime. Logic only; all hashes/tunables come from generated/ (sheets).
#include <windows.h>
#include <cmath>
#include <main.h>
#include "../generated/natives_gen.h"
#include "../generated/tuning_gen.h"

static const float D2R = 3.14159265f / 180.f;
static bool  hooked = false, gliding = false;
static Vector3 hookPt{};

static Vector3 camDir() {
    Vector3 r = N::GET_GAMEPLAY_CAM_ROT(2);
    float p = r.x * D2R, y = r.z * D2R;
    return Vector3{ -sinf(y) * cosf(p), 0, cosf(y) * cosf(p), 0, sinf(p), 0 };
}

static void grapple(Ped me, Vector3 pos) {
    if (N::IS_CONTROL_PRESSED(0, T::CTRL_GRAPPLE_AIM) && N::IS_CONTROL_JUST_PRESSED(0, T::CTRL_GRAPPLE_FIRE)) {
        Vector3 c = N::GET_GAMEPLAY_CAM_COORD(), d = camDir();
        int h = N::START_EXPENSIVE_SYNCHRONOUS_SHAPE_TEST_LOS_PROBE(c.x, c.y, c.z,
            c.x + d.x * T::GRAPPLE_RANGE, c.y + d.y * T::GRAPPLE_RANGE, c.z + d.z * T::GRAPPLE_RANGE, -1, me, 7);
        BOOL hit = 0; Vector3 end{}, nrm{}; Entity e = 0;
        N::GET_SHAPE_TEST_RESULT(h, &hit, &end, &nrm, &e);
        if (hit) { hooked = true; hookPt = end; gliding = false; }
    }
    if (!hooked) return;
    float dx = hookPt.x - pos.x, dy = hookPt.y - pos.y, dz = hookPt.z - pos.z;
    float len = sqrtf(dx * dx + dy * dy + dz * dz);
    if (len < T::GRAPPLE_STOP_DIST) { hooked = false; return; }
    float s = T::GRAPPLE_PULL;
    if (N::GET_PED_PARACHUTE_STATE(me) == 2) s += T::CHUTE_BOOST;
    N::SET_ENTITY_VELOCITY(me, dx / len * s, dy / len * s, dz / len * s);
    N::DRAW_LINE(pos.x, pos.y, pos.z, hookPt.x, hookPt.y, hookPt.z, 20, 20, 20, 255);
}

static void wingsuit(Ped me) {
    bool air = N::IS_ENTITY_IN_AIR(me);
    if (!air) { gliding = false; return; }
    if (N::IS_CONTROL_JUST_PRESSED(0, T::CTRL_WINGSUIT_TOGGLE) && N::GET_PED_PARACHUTE_STATE(me) <= 0)
        gliding = !gliding;
    if (!gliding || hooked) return;
    Vector3 v = N::GET_ENTITY_VELOCITY(me), d = camDir();
    float z = v.z + T::WINGSUIT_GLIDE;
    if (z < T::WINGSUIT_SINK) z = T::WINGSUIT_SINK;
    if (z > 0) z = 0;
    N::SET_ENTITY_VELOCITY(me, d.x * T::WINGSUIT_SPEED, d.y * T::WINGSUIT_SPEED, z);
}

static void tick() {
    Ped me = N::PLAYER_PED_ID();
    Vector3 pos = N::GET_ENTITY_COORDS(me, TRUE);
    if (T::INFINITE_CHUTES) N::GIVE_WEAPON_TO_PED(me, 0xFBAB5776 /*GADGET_PARACHUTE*/, 1, FALSE, FALSE);
    grapple(me, pos);
    wingsuit(me);
    if (N::IS_CONTROL_JUST_PRESSED(0, T::CTRL_CHAOS_DETONATE) && (hookPt.x || hookPt.y || hookPt.z))
        N::ADD_EXPLOSION(hookPt.x, hookPt.y, hookPt.z, T::CHAOS_EXPLOSION_TYPE, T::CHAOS_DAMAGE, TRUE, FALSE, 1.0f, FALSE);
}

static void ScriptMain() { for (;;) { tick(); WAIT(0); } }

BOOL APIENTRY DllMain(HMODULE mod, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) scriptRegister(mod, ScriptMain);
    else if (reason == DLL_PROCESS_DETACH) scriptUnregister(mod);
    return TRUE;
}
