use std::f32::consts::{PI, FRAC_PI_2};

use ggez::{self, event, graphics::{self, Canvas, Mesh}, Context, GameError, glam::{vec2, Vec2, Mat2}};

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
const FIELD_OF_VIEW: f32 = 0.60; // works out to ~60 degrees
// Scaling factor between world units and map cells.
// This is to let us deal with reasonably sized numbers and normalized vectors instead of tiny fractions
const UNITS_PER_CELL: f32 = 64.0;

#[derive(Debug, Clone, Copy)]
enum Wall {
    Brick,
    Fallback,
}

struct GameState {
    map: Vec<Vec<Option<Wall>>>,
    player_position: Vec2,
    direction_vector: Vec2,
    camera_plane: Vec2,
}

impl GameState {
    fn new(map: Vec<Vec<Option<Wall>>>, player_position: Vec2, direction_vector: Vec2) -> GameState {
        let direction_vector = direction_vector.normalize(); // Make sure it's normalized!!
        GameState {
            map,
            player_position,
            direction_vector,
            // Perpendicular to the direction vector and scaled by the fov coefficient
            camera_plane: vec2(-direction_vector.y, direction_vector.x).clamp_length(FIELD_OF_VIEW, FIELD_OF_VIEW),
        }
    }

    fn rotate(&mut self, angle: f32) {
        let rotation_matrix = Mat2::from_cols_array(&[angle.cos(), angle.sin(), -angle.sin(), angle.cos()]);
        self.direction_vector = rotation_matrix.mul_vec2(self.direction_vector);
        self.camera_plane = rotation_matrix.mul_vec2(self.camera_plane);
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

        // ---- THIS IS WHERE THE RAYCASTING HAPPENS ----
        {
            
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
    let window_mode = ggez::conf::WindowMode::default().dimensions(X_RESOLUTION, Y_RESOLUTION);
    context.gfx.set_mode(window_mode).expect("Failed to set window mode");
    let map = parse_map(DEFAULT_MAP);
    let state = GameState::new(map, vec2(3.0, 3.0), vec2(0.0, -1.0));
    event::run(context, events, state);
}
