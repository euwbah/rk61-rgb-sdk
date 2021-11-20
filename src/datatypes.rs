use std::collections::HashMap;
use rand;
use rand::Rng;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

const MODES: [Mode; 21] = {
    use Mode::*;

    [
        NoBacklight,
        Static,
        SingleOn,
        SingleOff,
        Glittering,
        Falling,
        Colorful,
        Breath,
        Spectrum,
        Outward,
        Scrolling,
        Rolling,
        Rotating,
        Explode,
        Launch,
        Ripples,
        Flowing,
        Pulsating,
        Tilt,
        Shuttle,
        UserDefined
    ]
};

pub struct LightingUpdateMessage {
    /// List of all mode presets for all modes EXCEPT `Mode::NoBacklight`
    mode_presets: HashMap<Mode, ModePreset>,

    /// Contains RGB color mappings for each key in UserDefined mode
    /// If empty, defaults to all off.
    key_colors: HashMap<Key, RGB>,

    /// The current active mode, can also be `Mode::NoBacklight`
    active_mode: ModePreset,
}

impl LightingUpdateMessage {
    /// Creates a `LightingUpdateMessage` struct based on desired
    /// active mode configuration setting, and sets all the other modes
    /// to the default settings as per `ModePreset::default_for(Mode)`
    pub fn set_active_mode(active_mode: ModePreset) -> LightingUpdateMessage {
        let mut mode_presets = mode_presets_default_hashmap();

        // If active_mode is Mode::NoBacklight, the mode_presets hashmap would
        // not contain such a mode.
        if mode_presets.contains_key(&active_mode.mode) {
            *mode_presets.get_mut(&active_mode.mode).unwrap() = active_mode.clone();
        }

        LightingUpdateMessage {
            mode_presets,
            key_colors: HashMap::new(),
            active_mode,
        }
    }

    pub fn set_backlight_off() -> LightingUpdateMessage {
        LightingUpdateMessage {
            mode_presets: mode_presets_default_hashmap(),
            key_colors: HashMap::new(),
            active_mode: mode_preset(
                Mode::NoBacklight,
                rgb(0,0,0),
                false,
                1,
                1,
                Direction::Right
            )
        }
    }

    pub fn set_user_defined(brightness: u8, hmap: HashMap<Key, RGB>) -> LightingUpdateMessage {
        LightingUpdateMessage {
            mode_presets: mode_presets_default_hashmap(),
            key_colors: hmap,
            active_mode: mode_preset(
                Mode::UserDefined,
                rgb(0xff, 0xff, 0xff),
                false,
                brightness,
                1,
                Direction::Right
            )
        }
    }

    pub(crate) fn construct_feature_report_data_blocks(&self) -> [[u8; 65]; 26] {
        // data consists of 26 blocks of 64 bytes.
        let mut data: Vec<u8> = vec![0; 26 * 64];

        // block 1: the poll/wake message
        data[0] = 0x04;
        data[1] = 0x18;

        // block 2: Start of Lighting Update Message
        data[0x40] = 0x04;
        data[0x41] = 0xab;

        // block 3: Absolute nonsense (TODO: figure out what this is for)
        let mut rng = rand::thread_rng();
        for i in 0x00..0x40 {
            data[0x80 + i] = rng.gen();
        }

        // block 4: 04 02
        data[0xc0] = 0x04;
        data[0xc1] = 0x02;

        // block 5: signifies start of preset programming(?)
        data[0x100..0x109].copy_from_slice(&[
            0x04, 0x13, 0, 0,
            0, 0, 0, 0,
            0x12
        ]);

        // blocks 6 - 10: preset mode states
        {
            use Mode::*;
            for m in 0x01..0x13 {
                let idx = m - 1;
                let mode: Mode = FromPrimitive::from_usize(m).unwrap();
                let mp_bytes: [u8; 16] = self.mode_presets[&mode].into();
                data[(0x140 + idx * 0x10)..(0x150 + idx * 0x10)].copy_from_slice(
                    &mp_bytes
                );
            }

            // last 16 bytes of block 10 is for the UserDefined (0x80) mode preset
            let mp_bytes: [u8; 16] = self.mode_presets[&UserDefined].into();
            data[0x270..0x280].copy_from_slice(&mp_bytes);
        }

        // blocks 11 to 13 are all blank (index 0x280 to 0x33F)

        // blocks 14 - 22: per-key coloring
        // sets key colors for user-defined mode

        {
            use Key::*;
            for idx in ((13 * 0x40)..(22 * 0x40)).step_by(4) {
                let key_num = idx - 13 * 0x40;
                let key: Option<Key> = FromPrimitive::from_usize(key_num);

                // Each 'key' whether present or NIL is delimited with an 0x80
                // prepending the following 3 RGB bytes.
                data[idx] = 0x80;

                if let Some(key) = key {
                    let key_color = self.key_colors.get(&key)
                        .unwrap_or(&rgb(0, 0, 0)).clone();

                    data[idx + 1] = key_color.red;
                    data[idx + 2] = key_color.green;
                    data[idx + 3] = key_color.blue;
                }
            }
        }

        // Block 23: current active mode selection
        {
            let active_mode_bytes: [u8; 16] = self.active_mode.into();
            data[0x580..0x590].copy_from_slice(&active_mode_bytes);
        }

        // Block 24: 04 02 Section marker
        data[0x5c0] = 0x04;
        data[0x5c1] = 0x02;

        // Block 25: 04 F0 End transmission
        data[0x600] = 0x04;
        data[0x601] = 0xF0;

        // Block 26: random polling block after transmission for idk what reason
        data[0x640] = 0x04;
        data[0x641] = 0x18;

        let mut message_blocks = [[0u8; 65]; 26];

        for (block_number, idx) in (0..(26 * 64)).step_by(64).enumerate() {
            // prepend the 64 byte block with 1 byte HIDAPI report ID (default is assumed to be 0).
            let mut block = [0u8; 65];
            block[1..].copy_from_slice(&data[idx..(idx + 64)]);
            message_blocks[block_number] = block;
        }

        message_blocks
    }
}

#[derive(Copy, Clone)]
pub struct ModePreset {
    mode: Mode,
    color: RGB,
    full_color: bool,
    brightness: u8,
    speed: u8,
    direction: Direction,
}

pub fn mode_presets_default_hashmap() -> HashMap<Mode, ModePreset> {
    let mut h = HashMap::new();

    for m in &MODES[1..] {
        h.insert(m.clone(), ModePreset::default_for(m.clone()));
    }

    h
}

impl ModePreset {
    pub fn default_for(mode: Mode) -> ModePreset {
        mode_preset(
            mode,
            rgb(0xff, 0xff, 0xff),
            true,
            0x10,
            0xc,
            match mode {
                Mode::Scrolling => Direction::Down,
                _ => Direction::Right
            },
        )
    }
}

impl Into<[u8; 16]> for ModePreset {
    fn into(self) -> [u8; 16] {
        let mode = self.mode as u8;
        let (rr, gg, bb) = (self.color.red, self.color.green, self.color.blue);
        [
            mode, rr, gg, bb,
            0, 0, 0, 0,
            self.full_color as u8, self.brightness, self.speed, self.direction as u8,
            0, 0, 0xAA, 0x55
        ]
    }
}

pub fn mode_preset(mode: Mode, color: RGB, full_color: bool,
                   brightness: u8, speed: u8, direction: Direction) -> ModePreset {
    assert!(brightness <= 0x10 && brightness >= 0x01, "Brightness must be between 0x1 and 0x10");
    assert!(speed <= 0x10 && speed >= 0x01, "Speed must be between 0x1 and 0x10");
    ModePreset {
        mode,
        color,
        full_color,
        brightness,
        speed,
        direction,
    }
}

#[derive(Copy, Clone)]
pub struct RGB {
    red: u8,
    green: u8,
    blue: u8,
}

pub fn rgb(red: u8, green: u8, blue: u8) -> RGB {
    RGB {
        red,
        green,
        blue,
    }
}

#[repr(u8)]
#[derive(FromPrimitive, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    NoBacklight = 0,
    Static = 1,
    SingleOn = 2,
    SingleOff = 3,
    Glittering = 4,
    Falling = 5,
    Colorful = 6,
    Breath = 7,
    Spectrum = 8,
    Outward = 9,
    Scrolling = 0x0a,
    Rolling = 0x0b,
    Rotating = 0x0c,
    Explode = 0x0d,
    Launch = 0x0e,
    Ripples = 0x0f,
    Flowing = 0x10,
    Pulsating = 0x11,
    Tilt = 0x12,
    Shuttle = 0x13,
    UserDefined = 0x80,
}

#[repr(u8)]
#[derive(FromPrimitive, Copy, Clone)]
pub enum Direction {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

#[derive(FromPrimitive, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    // block 14 nil

    // block 15
    Esc = 0x4c,
    Numrow1 = 0x50,
    Numrow2 = 0x54,
    Numrow3 = 0x58,
    Numrow4 = 0x5c,
    Numrow5 = 0x60,
    Numrow6 = 0x64,
    Numrow7 = 0x68,
    Numrow8 = 0x6c,
    Numrow9 = 0x70,
    Numrow0 = 0x74,
    Minus = 0x78,
    Equals = 0x7c,

    // block 16
    Tab = 0x94,
    Q = 0x98,
    W = 0x9c,
    E = 0xa0,
    R = 0xa4,
    T = 0xa8,
    Y = 0xac,
    U = 0xb0,
    I = 0xb4,
    O = 0xb8,
    P = 0xbc,

    // block 17
    LBracket = 0xc0,
    RBracket = 0xc4,
    CapsLock = 0xdc,
    A = 0xe0,
    S = 0xe4,
    D = 0xe8,
    F = 0xec,
    G = 0xf0,
    H = 0xf4,
    J = 0xf8,
    K = 0xfc,

    // block 18
    L = 0x100,
    Semicolon = 0x104,
    Quote = 0x108,
    Backslash = 0x10c,
    LShift = 0x124,
    Z = 0x128,
    X = 0x12c,
    C = 0x130,
    V = 0x134,
    B = 0x138,
    N = 0x13c,

    // block 19
    M = 0x140,
    Comma = 0x144,
    Fullstop = 0x148,
    Slash = 0x14c,
    RShift = 0x150,
    Enter = 0x154,
    LCtrl = 0x16c,
    LWin = 0x170,
    LAlt = 0x174,
    Space = 0x178,
    RAlt = 0x17c,

    // block 20
    Menu = 0x180,
    RCtrl = 0x184,
    Fn = 0x188,
    Backspace = 0x19c,

    // block 21 NIL

    // block 22 NIL
}

/// Get key by coordinate (based on RK61 layout)
pub fn key(x: usize, y: usize) -> Option<Key> {
    use Key::*;
    match y {
        0 => match x {
            0 => Some(Esc),
            1 => Some(Numrow1),
            2 => Some(Numrow2),
            3 => Some(Numrow3),
            4 => Some(Numrow4),
            5 => Some(Numrow5),
            6 => Some(Numrow6),
            7 => Some(Numrow7),
            8 => Some(Numrow8),
            9 => Some(Numrow9),
            10 => Some(Numrow0),
            11 => Some(Minus),
            12 => Some(Equals),
            13 => Some(Backspace),
            _ => None
        }
        1 => match x {
            0 => Some(Tab),
            1 => Some(Q),
            2 => Some(W),
            3 => Some(E),
            4 => Some(R),
            5 => Some(T),
            6 => Some(Y),
            7 => Some(U),
            8 => Some(I),
            9 => Some(O),
            10 => Some(P),
            11 => Some(LBracket),
            12 => Some(RBracket),
            13 => Some(Backslash),
            _ => None
        }
        2 => match x {
            0 => Some(CapsLock),
            1 => Some(A),
            2 => Some(S),
            3 => Some(D),
            4 => Some(F),
            5 => Some(G),
            6 => Some(H),
            7 => Some(J),
            8 => Some(K),
            9 => Some(L),
            10 => Some(Semicolon),
            11 => Some(Quote),
            12 => Some(Enter),
            _ => None
        }
        3 => match x {
            0 => Some(LShift),
            2 => Some(Z),
            3 => Some(X),
            4 => Some(C),
            5 => Some(V),
            6 => Some(B),
            7 => Some(N),
            8 => Some(M),
            9 => Some(Comma),
            10 => Some(Fullstop),
            11 => Some(Slash),
            13 => Some(RShift),
            _ => None
        }
        4 => match x {
            0 => Some(LCtrl),
            1 => Some(LWin),
            2 => Some(LAlt),
            6 => Some(Space),
            10 => Some(RAlt),
            11 => Some(Menu),
            12 => Some(RCtrl),
            13 => Some(Fn),
            _ => None
        }
        _ => None
    }
}