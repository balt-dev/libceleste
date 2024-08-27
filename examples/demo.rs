
use libceleste::Maddy;
use macroquad::prelude::*;

extern "C" fn solid_check(_maddy: *mut Maddy, x: i32, y: i32, _dir_x: i32, _dir_y: i32) -> bool {
    !(96 .. 128).contains(&y) || !(32 .. 128).contains(&x)
}

const SCALE: f32 = 2.0;

#[macroquad::main("Demo")]
async fn main() {
    let mut maddy = libceleste::Maddy::new();
    maddy.x = 64;
    maddy.y = 104;
    maddy.solid_callback = Some(solid_check);

    loop {
        let keyflags = 
            (is_key_down(KeyCode::Left)  as u8) << 7 |
            (is_key_down(KeyCode::Up)    as u8) << 6 |
            (is_key_down(KeyCode::Down)  as u8) << 5 |
            (is_key_down(KeyCode::Right) as u8) << 4 |
            // No key on bit 3
            (is_key_down(KeyCode::Z)     as u8) << 2 |
            (is_key_down(KeyCode::X)     as u8) << 1 |
            (is_key_down(KeyCode::C)     as u8) << 0;
        
        maddy.tick(keyflags, get_frame_time());

        clear_background(WHITE);
        draw_rectangle(32. * SCALE, 96. * SCALE, 96. * SCALE, 32. * SCALE, BLACK);
        draw_rectangle((maddy.x + maddy.hitbox.x) as f32 * SCALE, (maddy.y + maddy.hitbox.y) as f32 * SCALE, (maddy.hitbox.w) as f32 * SCALE, (maddy.hitbox.h) as f32 * SCALE, RED);
        for (i, node) in maddy.hair.iter().enumerate() {
            let size = if i > 2 { 1. } else { 2. };
            draw_circle(node.x * SCALE, node.y * SCALE, size * SCALE, PINK)
        }
        
        draw_text(&format!("{:08b}", keyflags), 0., 10., 16., BLACK);
        for (i, line) in format!("{maddy:#?}").lines().enumerate() {
            draw_text(line, 0., 128. * SCALE + 30. + (i as f32) * 12., 16., BLACK);
        }

        next_frame().await;
    }
}