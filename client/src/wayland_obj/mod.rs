//! Wrappers around Wayland objects

mod layer_shell;
mod mousegrabber;
mod utils;
mod wl_compositor;
mod wl_output;
mod wl_shm;

pub use self::{
    layer_shell::{
        create_layer_surface, layer_shell_init, Layer, LayerSurface,
        LAYER_SHELL_VERSION
    },
    mousegrabber::{
        grab_mouse, mousegrabber_init, release_mouse, MOUSEGRABBER_VERSION
    },
    utils::instantiate_global,
    wl_compositor::{
        create_surface, WlCompositorManager, WL_COMPOSITOR_VERSION
    },
    wl_output::{Output, WlOutputManager, WL_OUTPUT_VERSION},
    wl_shm::{create_buffer, WlShmManager, WL_SHM_VERSION}
};
