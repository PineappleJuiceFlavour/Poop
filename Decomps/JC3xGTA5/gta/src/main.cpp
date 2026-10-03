// JC3xGTA5 GTA side (ScriptHookV). JC3 moves natively in GTA, plus the passthrough link to the running JC3.
// Hashes and tunables come from generated/ (sheets); never hard-code them here.
#include <windows.h>
#include <cmath>
#include <main.h>
#include "../../generated/natives_gen.h"
#include "../../shared/bridge.h"
#include "compositor.h"

static const float D2R = 3.14159265f / 180.f;
static bool  hooked = false, gliding = false;
static Vector3 hookPt{};
static bool passthrough = T::PASSTHROUGH_START_ON, ricoMirror = false;
static uint32_t evTail = 0;
static HMODULE g_mod = nullptr;

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
        if (hit) {
            hooked = true; hookPt = end; gliding = false;
            if (B::Shared* s = bridge::open()) {
                float g[3] = {end.x, end.y, end.z}; double j[3]; bridge::gta_to_jc3(g, j);
                bridge::push(s->ev_gta, B::EV_GRAPPLE_HIT, j);
            }
        }
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

static bool key_pressed(int vk) {
    static bool down[256];
    bool now = (GetAsyncKeyState(vk) & 0x8000) != 0, edge = now && !down[vk];
    down[vk] = now;
    return edge;
}

// Publish GTA's camera and player to JC3; act out JC3's events in GTA.
static void link(Ped me, Vector3 pos) {
    compositor::try_register(g_mod);
    if (key_pressed(T::CTRL_PASSTHROUGH_TOGGLE)) passthrough = !passthrough;
    if (key_pressed(T::CTRL_RICO_MIRROR_TOGGLE)) ricoMirror = !ricoMirror;
    compositor::set_active(passthrough);
    B::Shared* s = bridge::open();
    if (!s) return;
    s->header.gta_heartbeat = GetTickCount64();

    Vector3 c = N::GET_FINAL_RENDERED_CAM_COORD(), r = N::GET_FINAL_RENDERED_CAM_ROT(2);
    float p = r.x * D2R, y = r.z * D2R;
    float gf[3] = {-sinf(y) * cosf(p), cosf(y) * cosf(p), sinf(p)};
    float gu[3] = {sinf(y) * sinf(p), -cosf(y) * sinf(p), cosf(p)};
    float gc[3] = {c.x, c.y, c.z};
    s->cam.seq++;
    MemoryBarrier();
    bridge::gta_to_jc3(gc, s->cam.pos);
    bridge::gta_dir_to_jc3(gf, s->cam.fwd);
    bridge::gta_dir_to_jc3(gu, s->cam.up);
    s->cam.fov = N::GET_FINAL_RENDERED_CAM_FOV();
    MemoryBarrier();
    s->cam.seq++;

    Vector3 f = N::GET_ENTITY_FORWARD_VECTOR(me);
    float gp[3] = {pos.x, pos.y, pos.z - 1.0f}, gfw[3] = {f.x, f.y, 0};
    bridge::gta_to_jc3(gp, s->player.pos);
    bridge::gta_dir_to_jc3(gfw, s->player.fwd);
    s->player.flags = (passthrough ? 1u : 0u) | (ricoMirror ? 6u : 0u);
    N::SET_ENTITY_VISIBLE(me, !(passthrough && ricoMirror && bridge::alive(s->header.jc3_heartbeat)), FALSE);

    bridge::drain(s->ev_jc3, evTail, [](const B::Event& e) {
        float g[3]; bridge::jc3_to_gta(e.pos, g);
        if (e.type == B::EV_EXPLOSION)
            N::ADD_EXPLOSION(g[0], g[1], g[2], T::CHAOS_EXPLOSION_TYPE, T::CHAOS_DAMAGE, TRUE, FALSE, 1.0f, FALSE);
        else if (e.type == B::EV_BULLET_HIT)
            N::ADD_EXPLOSION(g[0], g[1], g[2], 38 /*EXP_TAG_BULLET-ish small*/, 0.1f, FALSE, TRUE, 0.f, FALSE);
    });
}

static void tick() {
    Ped me = N::PLAYER_PED_ID();
    Vector3 pos = N::GET_ENTITY_COORDS(me, TRUE);
    if (T::INFINITE_CHUTES) N::GIVE_WEAPON_TO_PED(me, 0xFBAB5776 /*GADGET_PARACHUTE*/, 1, FALSE, FALSE);
    grapple(me, pos);
    wingsuit(me);
    if (N::IS_CONTROL_JUST_PRESSED(0, T::CTRL_CHAOS_DETONATE) && (hookPt.x || hookPt.y || hookPt.z))
    {
        N::ADD_EXPLOSION(hookPt.x, hookPt.y, hookPt.z, T::CHAOS_EXPLOSION_TYPE, T::CHAOS_DAMAGE, TRUE, FALSE, 1.0f, FALSE);
        if (B::Shared* s = bridge::open()) {
            float g[3] = {hookPt.x, hookPt.y, hookPt.z}; double j[3]; bridge::gta_to_jc3(g, j);
            bridge::push(s->ev_gta, B::EV_EXPLOSION, j, 6.f, 1.f);
        }
    }
    link(me, pos);
}

static void ScriptMain() { for (;;) { tick(); WAIT(0); } }

BOOL APIENTRY DllMain(HMODULE mod, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) { g_mod = mod; scriptRegister(mod, ScriptMain); }
    else if (reason == DLL_PROCESS_DETACH) { compositor::unregister(mod); scriptUnregister(mod); }
    return TRUE;
}
