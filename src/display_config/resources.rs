/**
 * A CRTC (CRT controller) is a logical monitor, ie a portion
 * of the compositor coordinate space. It might correspond
 * to multiple monitors, when in clone mode.
 */
#[derive(Debug)]
pub struct Crtc {
    pub id: u32,
    pub winsys_id: i64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub current_mode: i32,
    pub current_transform: u32,
    pub transforms: Vec<u32>,
    pub properties: dbus::arg::PropMap,
}

impl Crtc {
    pub fn from(
        result: (
            u32,
            i64,
            i32,
            i32,
            i32,
            i32,
            i32,
            u32,
            Vec<u32>,
            dbus::arg::PropMap,
        ),
    ) -> Crtc {
        Crtc {
            id: result.0,
            winsys_id: result.1,
            x: result.2,
            y: result.3,
            width: result.4,
            height: result.5,
            current_mode: result.6,
            current_transform: result.7,
            transforms: result.8,
            properties: result.9,
        }
    }
}

/**
 * An output represents a physical screen, connected somewhere to
 * the computer. Floating connectors are not exposed in the API.
 */
#[derive(Debug)]
pub struct Output {
    pub id: u32,
    pub winsys_id: i64,
    pub current_crtc: i32,
    pub possible_crtcs: Vec<u32>,
    pub name: String,
    pub modes: Vec<u32>,
    pub clones: Vec<u32>,
    pub properties: dbus::arg::PropMap,
}

impl Output {
    pub fn from(
        result: (
            u32,
            i64,
            i32,
            Vec<u32>,
            String,
            Vec<u32>,
            Vec<u32>,
            dbus::arg::PropMap,
        ),
    ) -> Output {
        Output {
            id: result.0,
            winsys_id: result.1,
            current_crtc: result.2,
            possible_crtcs: result.3,
            name: result.4,
            modes: result.5,
            clones: result.6,
            properties: result.7,
        }
    }
}

/**
 * A mode represents a set of parameters that are applied to each output,
 * such as resolution and refresh rate.
 */
#[derive(Debug)]
pub struct Mode {
    pub id: u32,
    pub winsys_id: i64,
    pub width: u32,
    pub height: u32,
    pub frequency: f64,
    pub flags: u32,
}

impl Mode {
    pub fn from(result: (u32, i64, u32, u32, f64, u32)) -> Mode {
        Mode {
            id: result.0,
            winsys_id: result.1,
            width: result.2,
            height: result.3,
            frequency: result.4,
            flags: result.5,
        }
    }
}

#[derive(Debug)]
pub struct Resources {
    pub serial: u32,
    pub crtcs: Vec<Crtc>,
    pub outputs: Vec<Output>,
    pub modes: Vec<Mode>,
    pub max_screen_width: i32,
    pub max_screen_height: i32,
}

impl Resources {
    pub fn from(
        result: (
            u32,
            Vec<(
                u32,
                i64,
                i32,
                i32,
                i32,
                i32,
                i32,
                u32,
                Vec<u32>,
                dbus::arg::PropMap,
            )>,
            Vec<(
                u32,
                i64,
                i32,
                Vec<u32>,
                String,
                Vec<u32>,
                Vec<u32>,
                dbus::arg::PropMap,
            )>,
            Vec<(u32, i64, u32, u32, f64, u32)>,
            i32,
            i32,
        ),
    ) -> Resources {
        Resources {
            serial: result.0,
            crtcs: result.1.into_iter().map(Crtc::from).collect(),
            outputs: result.2.into_iter().map(Output::from).collect(),
            modes: result.3.into_iter().map(Mode::from).collect(),
            max_screen_width: result.4,
            max_screen_height: result.5,
        }
    }
}
