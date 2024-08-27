#![no_std]
#![crate_type = "cdylib"]

use core::f32;

pub mod consts {
    pub const LEFT:  u8 = 0b1000_0000;
    pub const UP:    u8 = 0b0100_0000;
    pub const DOWN:  u8 = 0b0010_0000;
    pub const RIGHT: u8 = 0b0001_0000;


    pub const DASH:  u8 = 0b0000_0010;
    pub const JUMP:  u8 = 0b0000_0001;

    pub const HAIR_COUNT: usize = 5;
    pub const JUMP_GRACE_TIME: u8 = 6;
    pub const JUMP_BUFFER_TIME: u8 = 4;
    pub const PLAYER_HITBOX: crate::Hitbox = crate::Hitbox { x: 1, y: 3, w: 6, h: 5 };

    pub const MAX_SPEED: f32 = 1.;
    pub const WALL_JUMP_SPEED: f32 = 2.;
    pub const GROUND_ACCEL: f32 = 0.6;
    pub const AIR_ACCEL: f32 = 0.6;
    pub const DECEL: f32 = 0.15;
    pub const MAX_FALL: f32 = 2.;
    pub const MAX_FALL_SLIDE: f32 = 0.6;
    pub const GRAVITY: f32 = 0.21;
    pub const HALF_GRAVITY_THRESHOLD: f32 = 0.15;
    pub const JUMP_SPEED: f32 = -2.;

    pub const DASH_SPEED: f32 = 5.;
    pub const DASH_TIME: u8 = 4;
    pub const DASH_EFFECT_TIME: u8 = 10;
    pub const DASH_TARGET: f32 = 2.;
    pub const DASH_ACCEL: f32 = 1.5;
    pub const DASH_UPWARDS_MUL: f32 = 0.75;

    pub const WALL_JUMP_CHECK_DISTANCE: i32 = 3;
}

use consts::*;

macro_rules! pressed {
    ($input_flags: ident [ $flag: ident ]) => {
        ($input_flags & $flag) != 0
    };
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32
}

impl core::fmt::Debug for Vector2 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Vector2 {{ x: {}, y: {} }}", self.x, self.y)
    }
}

impl Vector2 {
    pub const extern "C" fn new(x: f32, y: f32) -> Self {
        Vector2 { x, y }
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct Hitbox {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32
}

impl core::fmt::Debug for Hitbox {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hitbox {{ x: {}, y: {}, w: {}, h: {} }}", self.x, self.y, self.w, self.h)
    }
}


#[repr(C)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Maddy {
    /// Callback for collision. Takes the X and Y to check, as well as the X and Y of the direction that's being checked, and returns a boolean.
    pub solid_callback: Option<extern "C" fn(*mut Self, i32, i32, i32, i32) -> bool>,
    pub audio_callback: Option<extern "C" fn(u8)>,
    pub hitbox: Hitbox,
    pub hair: [Vector2; 5],
    pub dash_target: Vector2,
    pub dash_accel: Vector2,
    pub speed: Vector2,
    pub x: i32,
    pub y: i32,
    pub rem: Vector2,
    pub jump_buffer: u8,
    pub jump_grace: u8,
    pub dashes: u8,
    pub max_dashes: u8, 
    pub dash_time: u8,
    pub dash_effect_time: u8,
    pub sprite: u8,
    pub sprite_offset: u8,
    pub was_on_ground: bool,
    pub flip_x: bool,
    pub jump_last_tick: bool,
    pub dash_last_tick: bool
}

fn approach(value: f32, target: f32, amount: f32) -> f32 {
    if value > target {
        target.max(value - amount)
    } else {
        target.min(value + amount)
    }
}

impl Maddy {
    pub const extern "C" fn new() -> Self {
        Self {
            solid_callback: None,
            audio_callback: None,
            x: 0,
            y: 0,
            rem: Vector2::new(0., 0.),
            speed: Vector2::new(0., 0.),
            jump_grace: 0,
            dashes: 0,
            dash_time: 0,
            dash_effect_time: 0,
            dash_target: Vector2::new(0., 0.),
            dash_accel: Vector2::new(0., 0.),
            hitbox: PLAYER_HITBOX,
            sprite: 0,
            sprite_offset: 0,
            was_on_ground: false,
            flip_x: false,
            max_dashes: 1,
            hair: [Vector2::new(0., 0.); HAIR_COUNT],
            jump_buffer: 0,
            jump_last_tick: false,
            dash_last_tick: false
        }
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

    pub extern "C" fn tick(&mut self, keys: u8) {
        let input_x = 
            if pressed!(keys[RIGHT]) { 1 }
            else if pressed!(keys[LEFT]) { -1 }
            else { 0 };
        let input_y =
            if pressed!(keys[UP]) { -1 }
            else if pressed!(keys[DOWN]) { 1 }
            else { 0 };

        let on_ground = self.is_solid(0, 1);
    
        let jump = pressed!(keys[JUMP]) && !self.jump_last_tick;
        self.jump_last_tick = pressed!(keys[JUMP]);

        if jump {
            self.jump_buffer = JUMP_BUFFER_TIME;
        } else if self.jump_buffer > 0 {
            self.jump_buffer -= 1;
        }

        let dash = pressed!(keys[DASH]) && !self.dash_last_tick;
        self.dash_last_tick = pressed!(keys[DASH]);

        if on_ground {
            self.jump_grace = JUMP_GRACE_TIME;
            if self.dashes < self.max_dashes {
                self.play(54);
                self.dashes = self.max_dashes;
            }
        } else if self.jump_grace > 0 {
            self.jump_grace -= 1;
        }

        if self.dash_effect_time > 0 {
            self.dash_effect_time -= 1;
        }

        if self.dash_time > 0 {
            // dash state

            self.dash_time -= 1;
            self.speed.x = approach(self.speed.x, self.dash_target.x, self.dash_accel.x);
            self.speed.y = approach(self.speed.y, self.dash_target.y, self.dash_accel.y);
        } else {
            // normal state

            let accel = if on_ground { GROUND_ACCEL } else { AIR_ACCEL };
            self.speed.x = if libm::fabsf(self.speed.x) > MAX_SPEED {
                approach(self.speed.x, libm::copysignf(MAX_SPEED, self.speed.x), DECEL)
            } else {
                approach(self.speed.x, input_x as f32 * MAX_SPEED, accel)
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
                self.speed.y = approach(self.speed.y, max_fall, gravity);
            }

            // jumping
            if self.jump_buffer > 0 {
                if self.jump_grace > 0 {
                    // jump normally
                    self.play(1);
                    self.jump_buffer = 0;
                    self.jump_grace = 0;
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
                        self.jump_buffer = 0;
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
                    self.dash_effect_time = 10;


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
            self.sprite_offset = (self.sprite_offset + 1) % 16;
            self.sprite = if !on_ground {
                // wall-pushing check
                if self.is_solid(input_x, 0)
                    { 5 }
                    else { 3 }
            } else if pressed!(keys[DOWN]) {
                6
            } else if pressed!(keys[UP]) {
                7
            } else if self.speed.x == 0. || input_x == 0 {
                1
            } else {
                1 + self.sprite_offset / 4
            }
        }

        self.was_on_ground = on_ground;

        // update position

        self.rem.x += self.speed.x;
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

        self.rem.y += self.speed.y;
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
            self.y as f32 + if pressed!(keys[DOWN]) { 4. } else { 3. }
        );

        for node in self.hair.iter_mut() {
            // I think this is supposed to be an easing thing? 
            node.x += (last.x - node.x) / 1.5;
            node.y += (last.y + 0.5 - node.y) / 1.5;
            last = *node;
        }
    }
}
