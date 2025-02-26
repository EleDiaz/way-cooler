//! A grab-bag of static Awesome API functions, including restart and shutdown.

use std::{
    default::Default,
    ffi::{CStr, CString},
    fmt::{self, Display, Formatter},
    mem,
    process::{Command, Stdio},
    ptr, thread
};

use {
    cairo::{self, ImageSurface, ImageSurfaceData},
    gdk_pixbuf::{Pixbuf, PixbufExt},
    glib::translate::{FromGlibPtrNone, ToGlibPtr},
    nix::{self, libc},
    rlua::{
        self, AnyUserData, LightUserData, MetaMethod, Table, ToLua, UserData,
        UserDataMethods, Value
    },
    xcb::{
        ffi::{self, xproto},
        xkb
    }
};

use crate::{
    common::{
        signal,
        xproperty::{XProperty, XPropertyType, PROPERTIES}
    },
    lua::NEXT_LUA
};

// TODO this isn't defined in the xcb crate, even though it should be.
// A patch should be open adding this to its generation scheme
extern "C" {
    fn xcb_xkb_get_names_value_list_unpack(
        buffer: *mut libc::c_void,
        nTypes: u8,
        indicators: u32,
        virtualMods: u16,
        groupNames: u8,
        nKeys: u8,
        nKeyAliases: u8,
        nRadioGroups: u8,
        which: u32,
        aux: *mut ffi::xkb::xcb_xkb_get_names_value_list_t
    ) -> libc::c_int;
}

#[derive(Clone, Default, Debug)]
pub struct AwesomeState {
    preferred_icon_size: u32
}

impl Display for AwesomeState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Awesome: {:p}", self)
    }
}

impl UserData for AwesomeState {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        fn index<'lua>(
            _: rlua::Context<'lua>,
            (awesome, index): (AnyUserData<'lua>, Value<'lua>)
        ) -> rlua::Result<Value<'lua>> {
            let table = awesome.get_user_value::<Table>()?;
            table.get::<_, Value>(index)
        };
        methods.add_meta_function(MetaMethod::Index, index);
    }
}

pub fn init<'lua>(lua: rlua::Context<'lua>) -> rlua::Result<()> {
    let awesome_table = lua.create_table()?;
    method_setup(lua, &awesome_table)?;
    property_setup(lua, &awesome_table)?;
    let globals = lua.globals();

    let global_string: Table = globals.get("string")?;
    global_string.set("wlen", lua.create_function(wlen)?)?;
    // TODO Add the rest of fixups

    let awesome = lua.create_userdata(AwesomeState::default())?;
    awesome.set_user_value(awesome_table)?;
    globals.set("awesome", awesome)
}

fn method_setup<'lua>(
    lua: rlua::Context<'lua>,
    awesome_table: &Table<'lua>
) -> rlua::Result<()> {
    // TODO Fill in rest
    awesome_table.set(
        "connect_signal",
        lua.create_function(signal::global_connect_signal)?
    )?;
    awesome_table.set(
        "disconnect_signal",
        lua.create_function(signal::global_disconnect_signal)?
    )?;
    awesome_table.set(
        "emit_signal",
        lua.create_function(signal::global_emit_signal)?
    )?;
    awesome_table
        .set("xrdb_get_value", lua.create_function(xrdb_get_value)?)?;
    awesome_table.set(
        "xkb_set_layout_group",
        lua.create_function(xkb_set_layout_group)?
    )?;
    awesome_table.set(
        "xkb_get_layout_group",
        lua.create_function(xkb_get_layout_group)?
    )?;
    awesome_table.set(
        "set_preferred_icon_size",
        lua.create_function(set_preferred_icon_size)?
    )?;
    awesome_table.set(
        "register_xproperty",
        lua.create_function(register_xproperty)?
    )?;
    awesome_table.set(
        "xkb_get_group_names",
        lua.create_function(xkb_get_group_names)?
    )?;
    awesome_table.set("set_xproperty", lua.create_function(set_xproperty)?)?;
    awesome_table.set("get_xproperty", lua.create_function(get_xproperty)?)?;
    awesome_table.set("systray", lua.create_function(systray)?)?;
    awesome_table.set("restart", lua.create_function(restart)?)?;
    awesome_table.set("load_image", lua.create_function(load_image)?)?;
    awesome_table
        .set("pixbuf_to_surface", lua.create_function(pixbuf_to_surface)?)?;
    awesome_table.set("sync", lua.create_function(sync)?)?;
    awesome_table.set("exec", lua.create_function(exec)?)?;
    awesome_table.set("spawn", lua.create_function(crate::objects::dummy)?)?;
    awesome_table.set("kill", lua.create_function(kill)?)?;
    awesome_table.set("quit", lua.create_function(quit)?)
}

fn property_setup<'lua>(
    _: rlua::Context<'lua>,
    awesome_table: &Table<'lua>
) -> rlua::Result<()> {
    // TODO Do properly
    awesome_table.set("version", "0")?;
    awesome_table.set("themes_path", "/usr/share/awesome/themes")?;
    awesome_table.set("conffile", "")
}

/// Registers a new X property
fn register_xproperty(
    _: rlua::Context,
    (name_rust, v_type): (String, String)
) -> rlua::Result<()> {
    let name =
        CString::new(name_rust.clone()).expect("XProperty was not CString");
    let arg_type = XPropertyType::from_string(v_type.clone()).ok_or(
        rlua::Error::RuntimeError(format!("{} not a valid xproperty", v_type))
    )?;

    unsafe {
        crate::XCB_CONNECTION.with(|con| {
            let raw_con = con.get_raw_conn();
            let atom_c = xproto::xcb_intern_atom_unchecked(
                raw_con,
                false as u8,
                name.to_bytes().len() as u16,
                name.as_ptr()
            );
            let atom_r =
                xproto::xcb_intern_atom_reply(raw_con, atom_c, ptr::null_mut());
            if atom_r.is_null() {
                return Ok(());
            }
            let new_property =
                XProperty::new(name_rust.clone(), arg_type, (*atom_r).atom);
            let mut properties =
                PROPERTIES.lock().expect("Could not lock properties list");
            if let Some(found) = properties
                .iter()
                .find(|&property| property == &new_property)
            {
                if found.type_ != new_property.type_ {
                    return Err(rlua::Error::RuntimeError(format!(
                        "property '{}' already \
                         registered with \
                         different type",
                        name_rust
                    )));
                }
                return Ok(());
            }
            properties.push(new_property);
            Ok(())
        })
    }
}

/// Get layout short names
fn xkb_get_group_names<'lua>(
    lua: rlua::Context<'lua>,
    _: ()
) -> rlua::Result<Value<'lua>> {
    unsafe {
        crate::XCB_CONNECTION.with(|con| {
            let raw_con = con.get_raw_conn();
            let id = xkb::ID_USE_CORE_KBD as _;
            let names_r = {
                let names_cookie = xkb::get_names_unchecked(
                    &con,
                    id,
                    xkb::NAME_DETAIL_SYMBOLS
                );
                names_cookie.get_reply()
            };
            let names_r = match names_r {
                Ok(names_r) => {
                    if names_r.ptr.is_null() {
                        warn!("Failed to get xkb symbols name");
                        return Ok(Value::Nil);
                    }
                    names_r
                },
                Err(err) => {
                    warn!("Failed to get xkb symbols name {:?}", err);
                    return Ok(Value::Nil);
                }
            };
            let buffer = ffi::xkb::xcb_xkb_get_names_value_list(names_r.ptr);
            if buffer.is_null() {
                warn!("Returned buffer was NULL");
                return Ok(Value::Nil);
            }
            let names_r_ptr = names_r.ptr;
            if names_r_ptr.is_null() {
                warn!("Name reply pointer was NULL");
                return Ok(Value::Nil);
            }
            let mut names_list: ffi::xkb::xcb_xkb_get_names_value_list_t =
                mem::zeroed();

            xcb_xkb_get_names_value_list_unpack(
                buffer,
                (*names_r_ptr).nTypes,
                (*names_r_ptr).indicators,
                (*names_r_ptr).virtualMods,
                (*names_r_ptr).groupNames,
                (*names_r_ptr).nKeys,
                (*names_r_ptr).nKeyAliases,
                (*names_r_ptr).nRadioGroups,
                (*names_r_ptr).which,
                &mut names_list
            );
            let atom_name_c = ffi::xproto::xcb_get_atom_name_unchecked(
                raw_con,
                names_list.symbolsName
            );
            let atom_name_r = ffi::xproto::xcb_get_atom_name_reply(
                raw_con,
                atom_name_c,
                ptr::null_mut()
            );
            if atom_name_r.is_null() {
                warn!("Failed to get atom symbols name");
                return Ok(Value::Nil);
            }
            let name_c = ffi::xproto::xcb_get_atom_name_name(atom_name_r);
            CStr::from_ptr(name_c)
                .to_string_lossy()
                .into_owned()
                .to_lua(lua)
        })
    }
}

/// Query & set information about the systray
fn systray<'lua>(
    _: rlua::Context<'lua>,
    _: ()
) -> rlua::Result<(u32, Value<'lua>)> {
    Ok((0, Value::Nil))
}

fn restart<'lua>(_: rlua::Context<'lua>, _: ()) -> rlua::Result<()> {
    info!("Restarting");
    NEXT_LUA.with(|next_lua| {
        next_lua.set(true);
    });
    Ok(())
}

/// Load an image from the given path
/// Returns either a cairo surface as light user data or nil and an error message
fn load_image<'lua>(
    lua: rlua::Context<'lua>,
    file_path: String
) -> rlua::Result<Value<'lua>> {
    let pixbuf = Pixbuf::new_from_file(file_path.as_str())
        .map_err(|err| rlua::Error::RuntimeError(format!("{}", err)))?;
    let surface = load_surface_from_pixbuf(pixbuf);
    // UGH, I wanted to do to_glib_full, but that isn't defined apparently
    // So now I have to ignore the lifetime completely and just forget about the
    // surface.
    let surface_ptr = surface.to_glib_none().0;
    ::std::mem::forget(surface);
    LightUserData(surface_ptr as _).to_lua(lua)
}

/// Convert a pixbuf to a cairo image surface.
/// Returns either a cairo surface as light user data, nil and an error message
fn pixbuf_to_surface<'lua>(
    lua: rlua::Context<'lua>,
    pixbuf: LightUserData
) -> rlua::Result<Value<'lua>> {
    let pixbuf = unsafe { Pixbuf::from_glib_none(pixbuf.0 as *const _) };
    let surface = load_surface_from_pixbuf(pixbuf);
    // UGH, I wanted to do to_glib_full, but that isn't defined apparently
    // So now I have to ignore the lifetime completely and just forget about the
    // surface.
    let surface_ptr = surface.to_glib_none().0;
    ::std::mem::forget(surface);
    LightUserData(surface_ptr as _).to_lua(lua)
}

fn exec(_: rlua::Context<'_>, command: String) -> rlua::Result<()> {
    trace!("exec: \"{}\"", command);
    thread::Builder::new()
        .name(command.clone())
        .spawn(|| {
            Command::new(command)
                .stdout(Stdio::null())
                .spawn()
                .expect("Could not spawn command")
                .wait()
        })
        .expect("Unable to spawn thread");
    Ok(())
}

/// Kills a PID with the given signal
///
/// Returns false if it could not send the signal to that process
fn kill(
    _: rlua::Context<'_>,
    (pid, sig): (libc::pid_t, libc::c_int)
) -> rlua::Result<bool> {
    Ok(nix::sys::signal::kill(pid, sig).is_ok())
}

fn set_preferred_icon_size(
    lua: rlua::Context<'_>,
    val: u32
) -> rlua::Result<()> {
    let awesome_state = lua.globals().get::<_, AnyUserData>("awesome")?;
    let mut awesome_state = awesome_state.borrow_mut::<AwesomeState>()?;
    awesome_state.preferred_icon_size = val;
    Ok(())
}

fn quit(_: rlua::Context<'_>, _: ()) -> rlua::Result<()> {
    crate::lua::terminate();
    Ok(())
}

// TODO This is used in tests to synchronize with the x11 server.
// We might need to do something similar with Way Cooler.
fn sync(_: rlua::Context<'_>, _: ()) -> rlua::Result<()> {
    Ok(())
}

fn set_xproperty(_: rlua::Context<'_>, _: Value) -> rlua::Result<()> {
    warn!("set_xproperty not supported");
    Ok(())
}

fn get_xproperty(_: rlua::Context<'_>, _: Value) -> rlua::Result<()> {
    warn!("get_xproperty not supported");
    Ok(())
}

fn xkb_set_layout_group(_: rlua::Context<'_>, group: u8) -> rlua::Result<()> {
    unsafe {
        crate::XCB_CONNECTION.with(|con| {
            let raw_con = con.get_raw_conn();
            ffi::xkb::xcb_xkb_latch_lock_state(
                raw_con,
                ffi::xkb::XCB_XKB_ID_USE_CORE_KBD as _,
                0,
                0,
                true as u8,
                group,
                0,
                0,
                0
            );
        })
    }
    Ok(())
}

fn xkb_get_layout_group<'lua>(
    lua: rlua::Context<'lua>,
    _: ()
) -> rlua::Result<Value<'lua>> {
    unsafe {
        crate::XCB_CONNECTION.with(|con| {
            let raw_con = con.get_raw_conn();
            let state_c = ffi::xkb::xcb_xkb_get_state_unchecked(
                raw_con,
                ffi::xkb::XCB_XKB_ID_USE_CORE_KBD as _
            );
            let state_r = ffi::xkb::xcb_xkb_get_state_reply(
                raw_con,
                state_c,
                ptr::null_mut()
            );
            if state_r.is_null() {
                warn!("State reply was NULL");
                return Ok(Value::Nil);
            }
            (*state_r).group.to_lua(lua)
        })
    }
}

fn xrdb_get_value<'lua>(
    _lua: rlua::Context<'lua>,
    (_resource_class, _resource_name): (String, String)
) -> rlua::Result<Value<'lua>> {
    warn!("xrdb_get_value not supported");
    Ok(Value::Nil)
}

#[cfg(target_endian = "big")]
fn write_u32(
    data: &mut ImageSurfaceData,
    i: usize,
    a: u8,
    r: u8,
    g: u8,
    b: u8
) {
    data[i + 0] = a;
    data[i + 1] = r;
    data[i + 2] = g;
    data[i + 3] = b;
}

#[cfg(target_endian = "little")]
fn write_u32(
    data: &mut ImageSurfaceData,
    i: usize,
    a: u8,
    r: u8,
    g: u8,
    b: u8
) {
    data[i + 3] = a;
    data[i + 2] = r;
    data[i + 1] = g;
    data[i + 0] = b;
}

/// Using a Pixbuf buffer, loads the data into a Cairo surface.
pub fn load_surface_from_pixbuf(pixbuf: Pixbuf) -> ImageSurface {
    let width = pixbuf.get_width();
    let height = pixbuf.get_height();
    let channels = pixbuf.get_n_channels() as usize;
    let pix_stride = pixbuf.get_rowstride() as usize;
    // NOTE This is safe because we aren't modifying the bytes, but there's no
    // immutable view
    let pixels = unsafe { pixbuf.get_pixels() };
    let format = if channels == 3 {
        cairo::Format::Rgb24
    } else {
        cairo::Format::ARgb32
    };
    let mut surface = ImageSurface::create(format, width, height)
        .expect("Could not create image of that size");
    let cairo_stride = surface.get_stride() as usize;
    {
        let mut cairo_data = surface.get_data().unwrap();
        for y in 0..height as usize {
            let mut pix_pixels_index = y * pix_stride;
            let mut cairo_pixels_index = y * cairo_stride;
            for _ in 0..width {
                let mut r = pixels[pix_pixels_index];
                let mut g = pixels[pix_pixels_index + 1];
                let mut b = pixels[pix_pixels_index + 2];
                let mut a = 1;
                if channels == 4 {
                    a = pixels[pix_pixels_index + 3];
                    let alpha = a as f64 / 255.0;
                    r = (r as f64 * alpha) as u8;
                    g = (g as f64 * alpha) as u8;
                    b = (b as f64 * alpha) as u8;
                }
                write_u32(&mut cairo_data, cairo_pixels_index, a, r, g, b);
                pix_pixels_index += channels;
                cairo_pixels_index += 4;
            }
        }
    }
    surface
}

/// UTF-8 aware string length computing
pub fn wlen<'lua>(
    lua: rlua::Context<'lua>,
    cmd: String
) -> rlua::Result<Value<'lua>> {
    cmd.len().to_lua(lua)
}
