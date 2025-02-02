#![warn(clippy::perf, missing_docs, clippy::pedantic)]
#![crate_type = "cdylib"]
#![no_std]

//! A crate to let you put Madeline from Celeste (Classic) in anything you want!

extern crate alloc;
use libc_alloc::LibcAlloc;

#[global_allocator]
static ALLOC: LibcAlloc = LibcAlloc;

use core::{f32, panic::PanicInfo};
use alloc::boxed::Box;

/// Bitflag for the left key being held down.
pub const KEYFLAG_LEFT:  u8 = 0b1000_0000;
/// Bitflag for the up key being held down.
pub const KEYFLAG_UP:    u8 = 0b0100_0000;
/// Bitflag for the down key being held down.
pub const KEYFLAG_DOWN:  u8 = 0b0010_0000;
/// Bitflag for the right key being held down.
pub const KEYFLAG_RIGHT: u8 = 0b0001_0000;
/// Bitflag for the dash key being held down.
pub const KEYFLAG_DASH:  u8 = 0b0000_0010;
/// Bitflag for the jump key being held down.
pub const KEYFLAG_JUMP:  u8 = 0b0000_0001;


/// An array to RGBA image stored as raw RGBA bytes.
/// 
/// The image is a 64x16 sprite atlas of 16 8x8 sprites, stored with 8 bits per channel.
/// 
/// This should be used along with Maddy.sprite in order to draw the player and hair.
/// 
/// Note that:
/// - The hair sprites are stored separate, at Maddy.sprite + 8
/// - The circles used to draw the hair nodes are at:
///   - Index 0 for size 1 circles (index into hair > 2)
///   - Index 8 for size 2 circles (index into hair <= 2)
#[no_mangle]
pub static ATLAS: [u8; 4096] = *include_bytes!("assets/atlas.rgba");

mod consts {
    pub(crate) const HAIR_COUNT: usize = 5;
    pub(crate) const JUMP_GRACE_TIME: f32 = 6.;
    pub(crate) const JUMP_BUFFER_TIME: f32 = 4.;
    pub(crate) const PLAYER_HITBOX: crate::Hitbox = crate::Hitbox { x: 1, y: 3, w: 6, h: 5 };

    pub(crate) const MAX_SPEED: f32 = 1.;
    pub(crate) const WALL_JUMP_SPEED: f32 = 2.;
    pub(crate) const GROUND_ACCEL: f32 = 0.6;
    pub(crate) const AIR_ACCEL: f32 = 0.6;
    pub(crate) const DECEL: f32 = 0.15;
    pub(crate) const MAX_FALL: f32 = 2.;
    pub(crate) const MAX_FALL_SLIDE: f32 = 0.6;
    pub(crate) const GRAVITY: f32 = 0.21;
    pub(crate) const HALF_GRAVITY_THRESHOLD: f32 = 0.15;
    pub(crate) const JUMP_SPEED: f32 = -2.;

    pub(crate) const DASH_SPEED: f32 = 5.;
    pub(crate) const DASH_TIME: f32 = 4.;
    pub(crate) const DASH_EFFECT_TIME: f32 = 10.;
    pub(crate) const DASH_TARGET: f32 = 2.;
    pub(crate) const DASH_ACCEL: f32 = 1.5;
    pub(crate) const DASH_UPWARDS_MUL: f32 = 0.75;

    pub(crate) const WALL_JUMP_CHECK_DISTANCE: i32 = 3;

    pub(crate) const HAIR_EASING_FACTOR: f32 = 1.002;
    pub(crate) const FPS: f32 = 30.;

    pub(crate) const HAIR_ZERO_DASHES: crate::Color = crate::Color::new(0x29, 0xad, 0xff, 0xff);
    pub(crate) const HAIR_ONE_DASH:    crate::Color = crate::Color::new(0xff, 0x00, 0x4d, 0xFF);
    pub(crate) const HAIR_TWO_DASHES:  crate::Color = crate::Color::new(0x00, 0xe4, 0x36, 0xFF);
    pub(crate) const HAIR_WHITE:       crate::Color = crate::Color::new(0xff, 0xf1, 0xe8, 0xff);
}

use consts::*;

macro_rules! pressed {
    ($input_flags: ident [ $flag: ident ]) => {
        ($input_flags & $flag) != 0
    };
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Default)]
/// A 2-dimensional vector.
pub struct Vector2 {
    /// The vector's X component.
    pub x: f32,
    /// The vector's Y component.
    pub y: f32
}

impl core::fmt::Debug for Vector2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Vector2 {{ x: {}, y: {} }}", self.x, self.y)
    }
}

impl Vector2 {
    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Vector2 { x, y }
    }
}


#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
/// An RGBA color.
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8    
}

impl Color {
    pub(crate) const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
/// A rectangle hitbox used for the player.
pub struct Hitbox {
    /// The hitbox's X offset.
    pub x: i32,
    /// The hitbox's Y offset.
    pub y: i32,
    /// The hitbox's width.
    pub w: i32,
    /// The hitbox's height.
    pub h: i32
}

impl core::fmt::Debug for Hitbox {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hitbox {{ x: {}, y: {}, w: {}, h: {} }}", self.x, self.y, self.w, self.h)
    }
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[allow(missing_docs)]
/// An instance of Madeline.
/// 
/// Note that her position is stored with Y going down as opposed to up.
pub struct Maddy {
    /// Callback for collision. Takes the X and Y to check, as well as the X and Y of the direction that's being checked, and returns a boolean.
    pub solid_callback: Option<extern "C" fn(*mut Self, i32, i32, i32, i32) -> bool>,
    /// Callback for audio. Takes the ID of audio to play. You might not need this.
    pub audio_callback: Option<extern "C" fn(u8)>,
    pub hitbox: Hitbox,
    pub hair: [Vector2; 5],
    pub dash_target: Vector2,
    pub dash_accel: Vector2,
    pub speed: Vector2,
    pub x: i32,
    pub y: i32,
    pub rem: Vector2,
    pub jump_buffer: f32,
    pub jump_grace: f32,
    pub dashes: u8,
    pub max_dashes: u8, 
    pub dash_time: f32,
    pub dash_effect_time: f32,
    pub sprite: u8,
    pub sprite_offset: f32,
    pub was_on_ground: bool,
    pub flip_x: bool,
    pub jump_last_tick: bool,
    pub dash_last_tick: bool,
    pub time_elapsed: f32
}

fn approach(value: f32, target: f32, amount: f32) -> f32 {
    if value > target {
        target.max(value - amount)
    } else {
        target.min(value + amount)
    }
}

impl Maddy {
    #[no_mangle]
    /**
        Initializes a new instance of Madeline on the heap.
        
        DO *NOT* FREE THIS POINTER.
        Instead, pass the pointer to CLST_Drop.
     */
    pub extern "C" fn CLST_Init() -> *mut Self {
        let this = Self {
            solid_callback: None,
            audio_callback: None,
            x: 0,
            y: 0,
            rem: Vector2::new(0., 0.),
            speed: Vector2::new(0., 0.),
            jump_grace: 0.,
            dashes: 0,
            dash_time: 0.,
            dash_effect_time: 0.,
            dash_target: Vector2::new(0., 0.),
            dash_accel: Vector2::new(0., 0.),
            hitbox: PLAYER_HITBOX,
            sprite: 0,
            sprite_offset: 0.,
            was_on_ground: false,
            flip_x: false,
            max_dashes: 1,
            hair: [Vector2::new(0., 0.); HAIR_COUNT],
            jump_buffer: 0.,
            jump_last_tick: false,
            dash_last_tick: false,
            time_elapsed: 0.
        };
        Box::into_raw(Box::new(this))
    }

    fn play(&self, sound_index: u8) {
        if let Some(callback) = self.audio_callback {
            callback(sound_index);
        }
    }

    fn is_solid(&mut self, dir_x: i32, dir_y: i32) -> bool {
        self.solid_callback.map_or(
            false,
            |callback| {
                for i in self.x + dir_x + self.hitbox.x .. self.x + self.hitbox.w + dir_x + self.hitbox.x {
                    for j in self.y + dir_y + self.hitbox.y .. self.y + self.hitbox.h + dir_y + self.hitbox.y {
                        if callback(self, i, j, dir_x, dir_y) {
                            return true;
                        }
                    }
                }
                false
            }
        )
    }

    #[no_mangle]
        /// Ticks this instance of Madeline according to the currently pressed keys and the delta-time.
    pub extern "C" fn CLST_Tick(&mut self, keys: u8, delta_time: f32) {
        self.time_elapsed += delta_time;

        let delta_ticks = delta_time * FPS;

        let input_x = 
            if pressed!(keys[KEYFLAG_RIGHT]) { 1 }
            else if pressed!(keys[KEYFLAG_LEFT]) { -1 }
            else { 0 };
        let input_y =
            if pressed!(keys[KEYFLAG_UP]) { -1 }
            else if pressed!(keys[KEYFLAG_DOWN]) { 1 }
            else { 0 };

        let on_ground = self.is_solid(0, 1);
    
        let jump = pressed!(keys[KEYFLAG_JUMP]) && !self.jump_last_tick;
        self.jump_last_tick = pressed!(keys[KEYFLAG_JUMP]);

        if jump {
            self.jump_buffer = JUMP_BUFFER_TIME;
        } else if self.jump_buffer > 0. {
            self.jump_buffer -= delta_ticks;
        }

        let dash = pressed!(keys[KEYFLAG_DASH]) && !self.dash_last_tick;
        self.dash_last_tick = pressed!(keys[KEYFLAG_DASH]);

        if on_ground {
            self.jump_grace = JUMP_GRACE_TIME;
            if self.dashes < self.max_dashes {
                self.play(54);
                self.dashes = self.max_dashes;
            }
        } else if self.jump_grace > 0. {
            self.jump_grace -= delta_ticks;
        }

        if self.dash_effect_time > 0. {
            self.dash_effect_time -= delta_ticks;
        }

        if self.dash_time > 0. {
            // dash state

            self.dash_time -= delta_ticks;
            self.speed.x = approach(self.speed.x, self.dash_target.x, self.dash_accel.x * delta_ticks);
            self.speed.y = approach(self.speed.y, self.dash_target.y, self.dash_accel.y * delta_ticks);
        } else {
            // normal state

            let accel = if on_ground { GROUND_ACCEL } else { AIR_ACCEL };
            self.speed.x = if libm::fabsf(self.speed.x) > MAX_SPEED {
                approach(self.speed.x, libm::copysignf(MAX_SPEED, self.speed.x), DECEL * delta_ticks)
            } else {
                approach(self.speed.x, input_x as f32 * MAX_SPEED, accel * delta_ticks)
            };

            if self.speed.x != 0.0 {
                self.flip_x = self.speed.x < 0.;
            }

            let gravity = GRAVITY * 
                if libm::fabsf(self.speed.y) <= HALF_GRAVITY_THRESHOLD { 0.5 }
                else { 1.0 };
            
            let max_fall = if input_x != 0 && self.is_solid(input_x, 0)
                {MAX_FALL_SLIDE}
                else {MAX_FALL};

            if !on_ground {
                // gravity
                self.speed.y = approach(self.speed.y, max_fall, gravity * delta_ticks);
            }

            // jumping
            if self.jump_buffer > 0. {
                if self.jump_grace > 0. {
                    // jump normally
                    self.play(1);
                    self.jump_buffer = 0.;
                    self.jump_grace = 0.;
                    self.speed.y = JUMP_SPEED;
                } else {
                    // wall jump
                    let wall_direction = 
                        if self.is_solid(-WALL_JUMP_CHECK_DISTANCE, 0) {
                            -1. // Left
                        } else if self.is_solid(WALL_JUMP_CHECK_DISTANCE, 0) {
                            1. // Right
                        } else { 0. };
                    if wall_direction != 0. {
                        self.play(2);
                        self.jump_buffer = 0.;
                        self.speed.y = JUMP_SPEED;
                        self.speed.x = -wall_direction * WALL_JUMP_SPEED;
                    }
                }
            }

            // dashing
            if dash {
                if self.dashes > 0 {
                    self.dashes -= 1;
                    self.dash_time = DASH_TIME;
                    self.dash_effect_time = DASH_EFFECT_TIME;


                    // Manual vector normalization
                    self.speed = match (input_x == 0, input_y == 0) {
                        (false, false) =>
                            Vector2::new(
                                // Multiply each direction by sqrt(2) / 2 to normalize
                                input_x as f32 * DASH_SPEED * f32::consts::SQRT_2 * 0.5,
                                input_y as f32 * DASH_SPEED * f32::consts::SQRT_2 * 0.5
                            ),
                        (true, false) =>
                            Vector2::new(0., input_y as f32 * DASH_SPEED),
                        (false, true) => 
                            Vector2::new(input_x as f32 * DASH_SPEED, 0.),
                        (true, true) =>
                            // Default to facing direction
                            Vector2::new(if self.flip_x {-1.} else {1.} * DASH_SPEED, 0.)
                    };

                    self.play(3);
                    self.dash_target.x = if self.speed.x != 0. { libm::copysignf(DASH_TARGET, self.speed.x) } else { 0. };
                    self.dash_target.y = if self.speed.y != 0. { libm::copysignf(DASH_TARGET, self.speed.y) } else { 0. };
                    self.dash_accel.x = DASH_ACCEL;
                    self.dash_accel.y = DASH_ACCEL;

                    if self.speed.y < 0. {
                        self.dash_target.y *= DASH_UPWARDS_MUL;
                    }

                    // More manual normalization, but maybe broken a bit?
                    // Blame the original code.
                    if self.speed.y != 0. {
                        self.dash_accel.x *= f32::consts::SQRT_2 * 0.5;
                    }
                    if self.speed.x != 0. {
                        self.dash_accel.y *= f32::consts::SQRT_2 * 0.5;
                    }
                } else {
                    self.play(9);
                }
            }

            // animation
            self.sprite_offset = (self.sprite_offset + delta_ticks) % 16.;
            self.sprite = if !on_ground {
                // wall-pushing check
                if self.is_solid(input_x, 0)
                    { 5 }
                    else { 3 }
            } else if pressed!(keys[KEYFLAG_DOWN]) {
                6
            } else if pressed!(keys[KEYFLAG_UP]) {
                7
            } else if self.speed.x == 0. || input_x == 0 {
                1
            } else {
                1 + (self.sprite_offset / 4.) as u8
            }
        }

        self.was_on_ground = on_ground;

        // update position

        let speed_x = self.speed.x * delta_ticks;
        let speed_y = self.speed.y * delta_ticks;

        self.rem.x += speed_x;
        let move_amount_x = libm::floorf(self.rem.x + 0.5) as i32;
        self.rem.x -= move_amount_x as f32;
        let step_x = move_amount_x.signum();
        for _ in 0..(move_amount_x as i32).abs() {
            if !self.is_solid(step_x, 0) {
                self.x += step_x;
            } else {
                self.speed.x = 0.;
                self.rem.x = 0.;
                break;
            }
        }

        self.rem.y += speed_y;
        let move_amount_y = libm::floorf(self.rem.y + 0.5) as i32;
        self.rem.y -= move_amount_y as f32;
        let step_y = move_amount_y.signum();
        for _ in 0..(move_amount_y as i32).abs() {
            if !self.is_solid(0, step_y) {
                self.y += step_y;
            } else {
                self.speed.y = 0.;
                self.rem.y = 0.;
                break;
            }
        }

        // update hair
        let facing = if self.flip_x { -1. } else { 1. };
        
        // There are so many magic numbers here that I can't decipher, sorry!
        let mut last = Vector2::new(
            self.x as f32 + 4. - libm::copysignf(2., facing),
            self.y as f32 + if pressed!(keys[KEYFLAG_DOWN]) { 4. } else { 3. }
        );

        for node in self.hair.iter_mut() {
            // Okay, so this is a problem.
            //
            // The generalization of the easing function for
            // "value += (target - value) / factor"
            // is
            // "value += (target - value) / (1 - (1 - factor) ^ delta_time)",
            // which is all fine and good, until your factor is greater than 1, which ours is.
            // 
            // I ended up replacing this with a different, more continuous easing function,
            // as it's only visual anyways.

            node.x += (last.x - node.x) / HAIR_EASING_FACTOR * delta_ticks;
            node.y += (last.y + 0.5 - node.y) / HAIR_EASING_FACTOR * delta_ticks;
            last = *node;
        }
    }

    #[no_mangle]
        /// Safely frees the memory allocated by CLST_Init.
    pub unsafe extern "C" fn CLST_Drop(this: *mut Self) {
        core::mem::drop(
            Box::from_raw(this)
        )
    }

    #[no_mangle]
        /// Returns the expected hair color for the amount of dashes Madeline has.
    pub unsafe extern "C" fn CLST_HairColor(&self, disable_flashing: bool) -> Color {
        if self.dash_time > 0. && !disable_flashing {
            return HAIR_WHITE;
        }
        match self.dashes {
            0 => HAIR_ZERO_DASHES,
            1 => HAIR_ONE_DASH,
            _ if self.time_elapsed % 0.2 >= 0.1 && !disable_flashing => HAIR_WHITE,
            _ => HAIR_TWO_DASHES
        }
    }
}


#[panic_handler]
fn panic_handler(_: &PanicInfo<'_>) -> ! {
    unsafe {
        libc::exit(127)
    }
}