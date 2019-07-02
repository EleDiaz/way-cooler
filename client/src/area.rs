//! Utility methods and structures

use rlua::{self, FromLua, ToLua};

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
// negative values could be used for slide-out animations
pub struct Margin {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32
}

impl<'lua> ToLua<'lua> for Margin {
    fn to_lua(self, lua: rlua::Context<'lua>) -> rlua::Result<rlua::Value<'lua>> {
        let res = lua.create_table()?;
        res.set("left", self.left)?;
        res.set("right", self.right)?;
        res.set("top", self.top)?;
        res.set("bottom", self.bottom)?;
        Ok(rlua::Value::Table(res))
    }
}

impl<'lua> FromLua<'lua> for Margin {
    fn from_lua(lua_value: rlua::Value<'lua>, lua: rlua::Context<'lua>) -> rlua::Result<Self> {
        let table = rlua::Table::from_lua(lua_value, lua)?;
        Ok(Margin {
            left: table.get("left")?,
            right: table.get("right")?,
            top: table.get("top")?,
            bottom: table.get("bottom")?
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
/// Generic geometry-like struct. Contains an origin (x, y) point and bounds
/// (width, height).
pub struct Area {
    pub origin: Origin,
    pub size: Size
}

impl Area {
    /// Makes a new `Area` with width and height set to the values in the given
    /// `Size`.
    pub fn with_size(self, size: Size) -> Self {
        Area { size, ..self }
    }

    /// Makes a new `Area` with x and y set to the value in the given `Origin`.
    #[allow(dead_code)]
    pub fn with_origin(self, origin: Origin) -> Self {
        Area { origin, ..self }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Origin {
    pub x: i32,
    pub y: i32
}

impl Into<Area> for Origin {
    fn into(self) -> Area {
        Area {
            origin: self,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Size {
    pub width: u32,
    pub height: u32
}

impl Into<Area> for Size {
    fn into(self) -> Area {
        Area {
            size: self,
            ..Default::default()
        }
    }
}
