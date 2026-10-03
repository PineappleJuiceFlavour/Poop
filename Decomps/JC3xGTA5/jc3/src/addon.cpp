// JC3xGTA5 guest add-on, loaded by ReShade inside Just Cause 3.
//  - exports JC3's colour + depth (from JC3Export.fx) into the shared bridge every frame;
//  - with symbols resolved: renders JC3 from GTA's camera, puts Rico where GTA's player is,
//    and swaps explosions both ways.
#include <windows.h>
#include <cmath>
#include <cstring>
#include <reshade.hpp>
#include <MinHook.h>
#include "symbols.h"
#include "../../shared/bridge.h"

using namespace reshade::api;

static B::Shared* g_s = nullptr;
static uint32_t g_tail = 0;
static resource g_staging[2] = {};
static uint32_t g_w = 0, g_h = 0, g_frame = 0;

// ---------- camera / character writes ----------
static void write_matrix(float* m, const float fwd[3], const float up[3], const double pos[3]) {
    // right = up x back, rows: right, up, back(or fwd), pos -- layout from tuning.csv JC3_MATRIX_LAYOUT
    float back[3] = {-fwd[0], -fwd[1], -fwd[2]};
    const float* z = T::JC3_MATRIX_LAYOUT == 0 ? back : fwd;
    float r[3] = {up[1] * back[2] - up[2] * back[1], up[2] * back[0] - up[0] * back[2], up[0] * back[1] - up[1] * back[0]};
    float rows[4][4] = {{r[0], r[1], r[2], 0}, {up[0], up[1], up[2], 0}, {z[0], z[1], z[2], 0},
                        {float(pos[0]), float(pos[1]), float(pos[2]), 1}};
    std::memcpy(m, rows, sizeof rows);
}

static uintptr_t active_camera() {
    uintptr_t mgr = *reinterpret_cast<uintptr_t*>(sym::v(sym::CameraManager));
    return mgr ? *reinterpret_cast<uintptr_t*>(mgr + sym::v(sym::Camera_Active)) : 0;
}
static uintptr_t local_character() {
    uintptr_t mgr = *reinterpret_cast<uintptr_t*>(sym::v(sym::PlayerManager));
    return mgr ? *reinterpret_cast<uintptr_t*>(mgr + sym::v(sym::LocalCharacter)) : 0;
}

// Generic 4-register-arg post-hook: let JC3 update its camera, then overwrite with GTA's.
using Fn4 = void*(__fastcall*)(void*, void*, void*, void*);
static Fn4 o_camera_update = nullptr;
static void* __fastcall hk_camera_update(void* a, void* b, void* c, void* d) {
    void* ret = o_camera_update(a, b, c, d);
    if (g_s && bridge::alive(g_s->header.gta_heartbeat) && (g_s->player.flags & 1)) {
        uint64_t seq = g_s->cam.seq;
        if (!(seq & 1)) {
            B::Cam_t cam = g_s->cam;
            if (g_s->cam.seq == seq)
                if (uintptr_t c0 = active_camera()) {
                    write_matrix(reinterpret_cast<float*>(c0 + sym::v(sym::Camera_Transform)), cam.fwd, cam.up, cam.pos);
                    *reinterpret_cast<float*>(c0 + sym::v(sym::Camera_Fov)) = cam.fov * 3.14159265f / 180.f;
                }
        }
    }
    return ret;
}

// SpawnExplosion: report every JC3 explosion to GTA, and remember the call so GTA explosions can be replayed.
static Fn4 o_spawn_explosion = nullptr;
static void* g_last_args[4] = {};
static float g_last_pos[4] = {};
static thread_local bool g_replaying = false;
static float* pos_arg(void** args) { return static_cast<float*>(args[sym::rows[sym::SpawnExplosion].extra]); }
static void* __fastcall hk_spawn_explosion(void* a, void* b, void* c, void* d) {
    void* args[4] = {a, b, c, d};
    if (!g_replaying && g_s) {
        float* p = pos_arg(args);
        double jp[3] = {p[0], p[1], p[2]};
        bridge::push(g_s->ev_jc3, B::EV_EXPLOSION, jp, 6.f, 1.f);
        std::memcpy(g_last_args, args, sizeof args);
        std::memcpy(g_last_pos, p, sizeof g_last_pos);
    }
    return o_spawn_explosion(a, b, c, d);
}
static void replay_explosion(const double p[3]) {
    if (!o_spawn_explosion || !g_last_args[0]) return; // needs one real JC3 explosion first
    void* args[4]; std::memcpy(args, g_last_args, sizeof args);
    float pos[4] = {float(p[0]), float(p[1]), float(p[2]), g_last_pos[3]};
    args[sym::rows[sym::SpawnExplosion].extra] = pos;
    g_replaying = true;
    o_spawn_explosion(args[0], args[1], args[2], args[3]);
    g_replaying = false;
}

static void hook(sym::Id id, void* detour, Fn4* orig) {
    auto target = reinterpret_cast<void*>(sym::v(id));
    if (MH_CreateHook(target, detour, reinterpret_cast<void**>(orig)) == MH_OK) MH_EnableHook(target);
}

// ---------- per-frame work (game's render thread, inside ReShade's present) ----------
static void sync_world() {
    g_s->header.jc3_heartbeat = GetTickCount64();
    if (sym::all({sym::PlayerManager, sym::LocalCharacter, sym::Character_Transform}))
        if (uintptr_t ch = local_character()) {
            auto* m = reinterpret_cast<float*>(ch + sym::v(sym::Character_Transform));
            if (bridge::alive(g_s->header.gta_heartbeat) && (g_s->player.flags & 2)) {
                float up[3] = {0, 1, 0};
                write_matrix(m, g_s->player.fwd, up, g_s->player.pos);
            }
            g_s->rico.pos[0] = m[12]; g_s->rico.pos[1] = m[13]; g_s->rico.pos[2] = m[14];
            g_s->rico.flags = 1;
        }
    bridge::drain(g_s->ev_gta, g_tail, [](const B::Event& e) {
        if (e.type == B::EV_EXPLOSION || e.type == B::EV_GRAPPLE_HIT) replay_explosion(e.pos);
    });
}

static void export_frame(effect_runtime* rt, command_list* cmd) {
    effect_texture_variable var = rt->find_texture_variable("JC3Export.fx", "JC3OutTex");
    if (var.handle == 0) return;
    resource_view srv = {}, srv_srgb = {};
    rt->get_texture_binding(var, &srv, &srv_srgb);
    if (srv.handle == 0) return;
    device* dev = rt->get_device();
    resource src = dev->get_resource_from_view(srv);
    resource_desc d = dev->get_resource_desc(src);
    const uint32_t w = d.texture.width, h = d.texture.height;
    if (uint64_t(w) * h > uint64_t(T::FRAME_MAX_PIXELS)) return;
    if (w != g_w || h != g_h) {
        for (auto& s : g_staging) if (s.handle) { dev->destroy_resource(s); s = {}; }
        for (auto& s : g_staging)
            dev->create_resource(resource_desc(w, h, 1, 1, format::r32g32b32a32_float, 1, memory_heap::gpu_to_cpu, resource_usage::copy_dest),
                                 nullptr, resource_usage::copy_dest, &s);
        g_w = w; g_h = h; g_frame = 0;
    }
    const int cur = g_frame & 1;
    cmd->barrier(src, resource_usage::shader_resource, resource_usage::copy_source);
    cmd->copy_resource(src, g_staging[cur]);
    cmd->barrier(src, resource_usage::copy_source, resource_usage::shader_resource);
    // read back the previous frame's copy (one frame of latency, no GPU stall)
    if (g_frame++ == 0) return;
    subresource_data m = {};
    if (!dev->map_texture_region(g_staging[cur ^ 1], 0, nullptr, map_access::read_only, &m)) return;
    const int slot = (g_s->frame.slot + 1) % B::kSlots;
    uint8_t* rgba = bridge::slot(g_s, slot);
    float* depth = reinterpret_cast<float*>(rgba + size_t(w) * h * 4);
    for (uint32_t y = 0; y < h; ++y) {
        const float* row = reinterpret_cast<const float*>(static_cast<const uint8_t*>(m.data) + size_t(y) * m.row_pitch);
        for (uint32_t x = 0; x < w; ++x) {
            const float* px = row + x * 4; const size_t i = size_t(y) * w + x;
            for (int c = 0; c < 3; ++c) rgba[i * 4 + c] = uint8_t(std::fmin(std::fmax(px[c], 0.f), 1.f) * 255.f + .5f);
            rgba[i * 4 + 3] = 255;
            depth[i] = px[3];
        }
    }
    dev->unmap_texture_region(g_staging[cur ^ 1], 0);
    g_s->frame.width = w; g_s->frame.height = h;
    g_s->frame.near = 0.1f; g_s->frame.far = 1000.f;
    std::memcpy(g_s->frame.pose_pos, g_s->cam.pos, sizeof g_s->cam.pos);
    std::memcpy(g_s->frame.pose_fwd, g_s->cam.fwd, sizeof g_s->cam.fwd);
    std::memcpy(g_s->frame.pose_up, g_s->cam.up, sizeof g_s->cam.up);
    g_s->frame.pose_fov = g_s->cam.fov;
    MemoryBarrier();
    g_s->frame.slot = slot;
    g_s->frame.publish++;
}

static void on_finish_effects(effect_runtime* rt, command_list* cmd, resource_view, resource_view) {
    if (!g_s && !(g_s = bridge::open())) return;
    sync_world();
    export_frame(rt, cmd);
}
static void on_destroy_runtime(effect_runtime* rt) {
    for (auto& s : g_staging) if (s.handle) { rt->get_device()->destroy_resource(s); s = {}; }
    g_w = g_h = 0;
}

static void init_hooks(HMODULE mod) {
    wchar_t path[MAX_PATH]; GetModuleFileNameW(mod, path, MAX_PATH);
    std::wstring csv(path); csv = csv.substr(0, csv.find_last_of(L"\\/") + 1) + L"jc3_symbols.csv";
    uint32_t mask = sym::load(csv);
    if ((g_s = bridge::open())) g_s->header.jc3_symbols_ok = mask;
    MH_Initialize();
    if (sym::all({sym::CameraManager, sym::Camera_Active, sym::Camera_Transform, sym::Camera_Fov, sym::CameraUpdate}))
        hook(sym::CameraUpdate, &hk_camera_update, &o_camera_update);
    if (sym::ok(sym::SpawnExplosion))
        hook(sym::SpawnExplosion, &hk_spawn_explosion, &o_spawn_explosion);
}

extern "C" __declspec(dllexport) const char* NAME = "JC3xGTA5";
extern "C" __declspec(dllexport) const char* DESCRIPTION = "Just Cause 3 side of the JC3 x GTA 5 passthrough.";

BOOL APIENTRY DllMain(HMODULE mod, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        if (!reshade::register_addon(mod)) return FALSE;
        reshade::register_event<reshade::addon_event::reshade_finish_effects>(on_finish_effects);
        reshade::register_event<reshade::addon_event::destroy_effect_runtime>(on_destroy_runtime);
        init_hooks(mod);
    } else if (reason == DLL_PROCESS_DETACH) {
        MH_DisableHook(MH_ALL_HOOKS); MH_Uninitialize();
        reshade::unregister_addon(mod);
    }
    return TRUE;
}
