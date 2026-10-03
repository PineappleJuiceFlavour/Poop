#include "compositor.h"
#include <windows.h>
#include <atomic>
#include <reshade.hpp>
#include "../../shared/bridge.h"

using namespace reshade::api;

namespace {
constexpr const char* kEffect = "JC3Passthrough.fx";
std::atomic<bool> g_registered{false}, g_active{T::PASSTHROUGH_START_ON};
B::Shared* g_s = nullptr;
uint64_t g_last = ~0ull;
struct Layer { resource tex = {}; resource_view srv = {}; } g_color, g_depth;
uint32_t g_w = 0, g_h = 0;

void destroy(device* d) {
    for (Layer* l : {&g_color, &g_depth}) {
        if (l->srv.handle) d->destroy_resource_view(l->srv);
        if (l->tex.handle) d->destroy_resource(l->tex);
        *l = {};
    }
    g_w = g_h = 0;
}
bool make(device* d, Layer& l, uint32_t w, uint32_t h, format f) {
    return d->create_resource(resource_desc(w, h, 1, 1, f, 1, memory_heap::default_, resource_usage::shader_resource | resource_usage::copy_dest),
                              nullptr, resource_usage::shader_resource, &l.tex) &&
           d->create_resource_view(l.tex, resource_usage::shader_resource, resource_view_desc(f), &l.srv);
}
void bind(effect_runtime* rt) {
    rt->update_texture_bindings("JC3COLOR", g_color.srv, g_color.srv);
    rt->update_texture_bindings("JC3DEPTH", g_depth.srv, g_depth.srv);
}
void set_bool(effect_runtime* rt, const char* n, bool v) {
    if (auto u = rt->find_uniform_variable(kEffect, n); u.handle) rt->set_uniform_value_bool(u, v);
}

void on_begin_effects(effect_runtime* rt, command_list*, resource_view, resource_view) {
    if (!g_s) g_s = bridge::open();
    const bool live = g_s && g_active && bridge::alive(g_s->header.jc3_heartbeat) && g_s->frame.publish;
    if (live && g_s->frame.publish != g_last) {
        const uint32_t w = g_s->frame.width, h = g_s->frame.height;
        const int slot = g_s->frame.slot;
        device* d = rt->get_device();
        if (w != g_w || h != g_h) {
            destroy(d);
            if (!make(d, g_color, w, h, format::r8g8b8a8_unorm) || !make(d, g_depth, w, h, format::r32_float)) { destroy(d); return; }
            g_w = w; g_h = h;
            bind(rt);
        }
        const uint8_t* base = bridge::slot(g_s, slot);
        subresource_data data = {const_cast<uint8_t*>(base), w * 4, w * h * 4};
        d->update_texture_region(data, g_color.tex, 0);
        data.data = const_cast<uint8_t*>(base + size_t(w) * h * 4);
        d->update_texture_region(data, g_depth.tex, 0);
        g_last = g_s->frame.publish;
    }
    set_bool(rt, "JC3Active", live && g_w);
    if (auto u = rt->find_uniform_variable(kEffect, "MaxDist"); u.handle) rt->set_uniform_value_float(u, T::COMPOSITE_MAX_DIST);
}
void on_reloaded(effect_runtime* rt) { if (g_w) bind(rt); }
void on_destroy(effect_runtime* rt) { destroy(rt->get_device()); }
}

namespace compositor {
bool try_register(void* m) {
    if (g_registered) return true;
    if (!reshade::register_addon(static_cast<HMODULE>(m))) return false;
    reshade::register_event<reshade::addon_event::reshade_begin_effects>(on_begin_effects);
    reshade::register_event<reshade::addon_event::reshade_reloaded_effects>(on_reloaded);
    reshade::register_event<reshade::addon_event::destroy_effect_runtime>(on_destroy);
    return g_registered = true;
}
void unregister(void* m) { if (g_registered.exchange(false)) reshade::unregister_addon(static_cast<HMODULE>(m)); }
void set_active(bool on) { g_active = on; }
}
