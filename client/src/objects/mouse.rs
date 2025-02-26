//! Represents a mouse that the user controls.
//!
//! We can request the compositor to do anything with the muose, including
//! changing the cursor and selecting where it should be on the screen.

use std::default::Default;

use rlua::{
    self, AnyUserData, MetaMethod, Table, ToLua, UserData, UserDataMethods,
    Value
};

use crate::objects::screen::{Screen, SCREENS_HANDLE};

const INDEX_MISS_FUNCTION: &'static str = "__index_miss_function";
const NEWINDEX_MISS_FUNCTION: &'static str = "__newindex_miss_function";

#[derive(Clone, Debug)]
pub struct MouseState {
    // TODO Fill in
    dummy: i32
}

impl Default for MouseState {
    fn default() -> Self {
        MouseState { dummy: 0 }
    }
}

impl UserData for MouseState {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_function(MetaMethod::Index, index);
    }
}

pub fn init(lua: rlua::Context) -> rlua::Result<()> {
    let mouse_table = lua.create_table()?;
    let meta_table = lua.create_table()?;
    let mouse = lua.create_userdata(MouseState::default())?;
    method_setup(lua, &mouse_table)?;
    let globals = lua.globals();
    mouse_table.set_metatable(Some(meta_table));
    mouse.set_user_value(mouse_table)?;
    globals.set("mouse", mouse)
}

fn method_setup<'lua>(
    lua: rlua::Context<'lua>,
    mouse_table: &Table<'lua>
) -> rlua::Result<()> {
    mouse_table.set("coords", lua.create_function(coords)?)?;
    mouse_table.set(
        "set_index_miss_handler",
        lua.create_function(set_index_miss)?
    )?;
    mouse_table.set(
        "set_newindex_miss_handler",
        lua.create_function(set_newindex_miss)?
    )?;
    Ok(())
}

fn coords<'lua>(
    _lua: rlua::Context<'lua>,
    (_coords, _ignore_enter): (Value<'lua>, Value<'lua>)
) -> rlua::Result<Table<'lua>> {
    // TODO Get Cords
    unimplemented!()
}

fn set_index_miss<'lua>(
    lua: rlua::Context<'lua>,
    func: rlua::Function<'lua>
) -> rlua::Result<()> {
    let button = lua.globals().get::<_, AnyUserData>("button")?;
    let table = button.get_user_value::<Table>()?;
    table.set(INDEX_MISS_FUNCTION, func)
}

fn set_newindex_miss<'lua>(
    lua: rlua::Context<'lua>,
    func: rlua::Function<'lua>
) -> rlua::Result<()> {
    let button = lua.globals().get::<_, AnyUserData>("button")?;
    let table = button.get_user_value::<Table>()?;
    table.set(NEWINDEX_MISS_FUNCTION, func)
}

fn index<'lua>(
    lua: rlua::Context<'lua>,
    (mouse, index): (AnyUserData<'lua>, Value<'lua>)
) -> rlua::Result<Value<'lua>> {
    let obj_table = mouse.get_user_value::<Table>()?;
    if let Value::String(ref string) = index {
        if string.to_str()? == "screen" {
            // TODO Get output
            let output = None; // unimplemented!();

            let screens: Vec<Screen> = lua
                .named_registry_value::<str, Vec<AnyUserData>>(SCREENS_HANDLE)?
                .into_iter()
                .map(|obj| Screen::cast(obj.into()).unwrap())
                .collect();
            if let Some(output) = output {
                for screen in &screens {
                    if screen.state()?.outputs.contains(&output) {
                        return screen.clone().to_lua(lua);
                    }
                }
            }
            if screens.len() > 0 {
                return screens[0].clone().to_lua(lua);
            }

            return Ok(Value::Nil);
        }
    }
    return obj_table.get(index);
}
