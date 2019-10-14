pub struct Location {
    pub(crate) x: u64,
    pub(crate) y: u64,
    pub(crate) z: u64,
}

impl Location {
    pub const fn from_long(val: u64) -> Location {
        Location {
            x: val >> 38,
            y: val << 26 >> 52,
            z: val << 38 >> 38,
        }
    }
}

pub struct Entity {
    pub id: u32,
    pub position: Location,
}
