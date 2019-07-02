//! Wrapper around a wl_compositor.

use std::cell::RefCell;

use wayland_client::{
    protocol::{wl_compositor::WlCompositor, wl_surface::WlSurface},
    GlobalImplementor, NewProxy
};

/// The minimum version of the wl_compositor global to bind to.
pub const WL_COMPOSITOR_VERSION: u32 = 3;

thread_local! {
    static WL_COMPOSITOR: RefCell<Option<WlCompositor>> = RefCell::new(None);
}

#[derive(Default)]
pub struct WlCompositorManager {
    compositor: Option<WlCompositor>
}

impl WlCompositorManager {
    pub fn create_surface(&self) -> Result<WlSurface, ()> {
        use wayland_client::Interface; // for ::NAME
        let compositor = self.compositor.as_ref().expect(&format!(
            "No WlCompositor registered. Make sure your compositor supports the {} protocol",
            WlCompositor::NAME
        ));
        compositor.create_surface(NewProxy::implement_dummy)
    }
}

impl GlobalImplementor<WlCompositor> for WlCompositorManager {
    fn new_global(&mut self, new_proxy: NewProxy<WlCompositor>) -> WlCompositor {
        let res = new_proxy.implement_dummy();

        WL_COMPOSITOR.with(|wl_compositor| {
            *wl_compositor.borrow_mut() = Some(res.clone());
        });

        res
    }
}

pub fn create_surface() -> Result<WlSurface, ()> {
    WL_COMPOSITOR.with(|wl_compositor| {
        let wl_compositor = wl_compositor.borrow();
        let wl_compositor = wl_compositor.as_ref().expect("WL_COMPOSITOR was not initilized");
        wl_compositor.create_surface(NewProxy::implement_dummy)
    })
}
