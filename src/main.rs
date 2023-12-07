use std::f32::consts::{PI, FRAC_PI_2};

use ggez::{self, event, graphics::{self, Canvas, Mesh}, Context, GameError, glam::vec2};

const DEFAULT_MAP: &str = "
##########
#........#
#........#
#........#
#........#
#........#
#........#
#........#
#........#
##########
";
const CAST_ITERATIONS: u8 = 10;
const THREEHALFS_PI: f32 = PI + FRAC_PI_2;
const X_RESOLUTION: f32 = 640.0;
const Y_RESOLUTION: f32 = 480.0;
const FIELD_OF_VIEW: f32 = FRAC_PI_2;


#[derive(Debug, Clone, Copy)]
enum Wall {
    Brick,
    Fallback,
}

struct GameState {
    map: Vec<Vec<Option<Wall>>>,
    player_x: f32,
    player_y: f32,
    look_angle: f32
}

impl GameState {
    fn new(map: Vec<Vec<Option<Wall>>>, player_x: f32, player_y: f32) -> GameState {
        GameState {
            map,
            player_x,
            player_y,
            look_angle: 0.0,
        }
    }

    /// Casts a ray from the player at the given angle (in radians). If it hits a wall, returns the type and distance; else returns None.
    fn cast_ray(&self, ctx: &Context, canvas: &mut Canvas, angle: f32, screen_x: f32) -> Result<(), GameError> {
        let mut ray_x: f32 = 0.0;
        let mut ray_y:f32 = 0.0;
        let mut x_offset: f32 = 0.0;
        let mut y_offset: f32 = 0.0;

        fn cast (map: &Vec<Vec<Option<Wall>>>,
            ctx: &Context,
            player_x: f32,
            player_y: f32,
            mut ray_x: f32,
            mut ray_y: f32,
            x_offset: f32,
            y_offset: f32,
        ) -> Result<Option<(f32, Mesh)>, GameError> {
            for _i in 0..CAST_ITERATIONS {
                // Break the loop if out of bounds
                if ray_x < 0.0
                    || ray_y < 0.0
                    || ray_y as usize >= map.len()
                    || ray_x as usize >= map[ray_y as usize].len()
                {
                    return Ok(None);
                }
                // If the line hits a wall, set the wall type and distance
                if let Some(wall) = map[ray_y as usize][ray_x as usize] {
                    let color = match wall {
                        _ => {graphics::Color::from_rgb(0, 255, 0)},
                    };
                    let distance = get_distance(player_x, player_y, ray_x, ray_y);
                    let length = Y_RESOLUTION / distance;
                    let line = graphics::Mesh::new_line(ctx,
                        &[
                            vec2(0.0, -(length / 2.0)),
                            vec2(0.0, length / 2.0)],
                        1.0,
                        color)?;
                    return Ok(Some((distance, line)));
                }
                // Else, increment to the next cell
                ray_x += x_offset;
                ray_y += y_offset;
            }
            return Ok(None);
        }

        // --- Check for hits on horizontal lines ---
        let looking_horizontal = angle == 0.0 || angle == PI;
        let cot = 1.0 / angle.tan();
        let mut horizontal_result: Option<(f32, Mesh)> = None;

        // If we're looking horizontally, we'll never hit a horizontal line
        if !looking_horizontal {
            // Set starting positions and offsets depending on angle
            if angle < PI {
                ray_y = self.player_y.ceil();
                y_offset = 1.0;
            } else {
                ray_y = self.player_y.floor();
                y_offset = -1.0;
            }
            ray_x = self.player_x + cot * (ray_y - self.player_y);
            x_offset = cot * y_offset;
            // Cast the ray
            horizontal_result = cast(&self.map, ctx, self.player_x, self.player_y, ray_x, ray_y, x_offset, y_offset)?;
        }

        // --- Check for hits on vertical lines ---
        let looking_vertical = angle == FRAC_PI_2 || angle == THREEHALFS_PI;
        let tan = angle.tan();
        let mut vertical_result: Option<(f32, Mesh)> = None;

        if !looking_vertical {
            if angle < FRAC_PI_2 || angle > THREEHALFS_PI {
                ray_x = self.player_x.ceil();
                x_offset = 1.0;
            } else {
                ray_x = self.player_x.floor();
                x_offset = -1.0;
            }
            ray_y = self.player_y + tan * (ray_x - self.player_x);
            y_offset = tan * x_offset;
            vertical_result = cast(&self.map, ctx, self.player_x, self.player_y, ray_x, ray_y, x_offset, y_offset)?;
        }

        // --- Compare the results ---
        let mut line: graphics::Mesh;
        if let Some(horizontal) = horizontal_result {
            if let Some(vertical) = vertical_result {
                if vertical.0 < horizontal.0 {
                    canvas.draw(&vertical.1, vec2(screen_x, Y_RESOLUTION / 2.0))
                }
            }
            canvas.draw(&horizontal.1, vec2(screen_x, Y_RESOLUTION / 2.0))
        }
        Ok(())
    }
}

impl event::EventHandler for GameState {
    fn update(&mut self, _ctx: &mut Context) -> Result<(), GameError> {
        Ok(())
    }

    fn draw(&mut self, _ctx: &mut Context) -> Result<(), GameError> {

        let mut canvas = graphics::Canvas::from_frame(
            _ctx,
            graphics::Color::from_rgb(0, 0, 0)
        );

        let mut cast_angle = self.look_angle - (FIELD_OF_VIEW / 2.0);
        let mut angle_increment = FIELD_OF_VIEW / X_RESOLUTION;
        for i in 0..(X_RESOLUTION as u32) {
            self.cast_ray(_ctx, &mut canvas, cast_angle, i as f32);
            cast_angle += angle_increment;
        }

        canvas.finish(_ctx)?;
        Ok(())
    }
}

fn get_distance(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt()
}

/// Converts an ASCII art representation of a map to a matrix of tiles
fn parse_map(map_str: &str) -> Vec<Vec<Option<Wall>>> {
    return map_str
        .trim()
        .lines()
        .map(|line| {
            line.chars()
                .map(|char| match char {
                    '.' => None,
                    '#' => Some(Wall::Brick),
                    _ => Some(Wall::Fallback),
                })
                .collect::<Vec<Option<Wall>>>()
        })
        .collect::<Vec<Vec<Option<Wall>>>>();
}

fn main() {
    let builder = ggez::ContextBuilder::new("Raycast test", "sagakar");
    let (mut context, events) = builder.build().expect("Failed to build context");
    let window_mode = ggez::conf::WindowMode::default();
    window_mode.dimensions(X_RESOLUTION, Y_RESOLUTION);
    context.gfx.set_mode(window_mode);
    let map = parse_map(DEFAULT_MAP);
    let state = GameState::new(map, 6.0, 6.0);
    event::run(context, events, state);
}
