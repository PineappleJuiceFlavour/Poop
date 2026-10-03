#pragma once
// ReShade add-on half of the GTA plugin: uploads JC3's newest exported frame for JC3Passthrough.fx.
namespace compositor {
bool try_register(void* module); // call until true (ReShade loads as an ASI, maybe after us)
void unregister(void* module);
void set_active(bool on);
}
