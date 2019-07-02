//! Wrappers around Wayland objects

mod layer_shell;
mod manager;
mod output;
mod wl_compositor;
mod wl_seat;
mod wl_shm;

pub use self::{
    layer_shell::{
        create_layer_surface, layer_shell_init, Layer, LayerShellManager, LayerSurface,
        WLR_LAYER_SHELL_VERSION
    },
    manager::{create_buffer, global_callback, WaylandManager, WAYLAND},
    output::{Output, OutputEventHandler, WlOutputManager, WL_OUTPUT_VERSION},
    wl_compositor::{create_surface, WlCompositorManager, WL_COMPOSITOR_VERSION},
    wl_shm::{Buffer, WlShmManager, WL_SHM_VERSION}
};
