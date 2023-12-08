use ggez::{
    self,
    event,
    graphics::{self, Mesh, Color},
    Context,
    GameError,
    input::keyboard::KeyCode,
    glam::{vec2, Vec2, Mat2}, timer::TimeContext};

const MAP_PATH: &str = "map.txt";
const X_RESOLUTION: f32 = 640.0;
const Y_RESOLUTION: f32 = 480.0;
const FIELD_OF_VIEW: f32 = 0.60; // works out to ~60 degrees
const MOVE_SPEED: f32 = 1.5; // In units / second
const ROTATION_SPEED: f32 = 1.2; // In radians / second 

#[derive(Debug, Clone, Copy)]
enum Wall {
    Brick,
    Fallback,
}

enum Side {
    NorthSouth,
    EastWest
}

struct GameState {
    map: Vec<Vec<Option<Wall>>>,
    player_position: Vec2,
    direction_vector: Vec2,
    camera_plane: Vec2,
    time_context: TimeContext,
}

impl GameState {
    fn new(map: Vec<Vec<Option<Wall>>>, player_position: Vec2, direction_vector: Vec2) -> GameState {
        let direction_vector = direction_vector.normalize(); // Make sure it's normalized!!
        GameState {
            map,
            player_position,
            direction_vector,
            // Perpendicular to the direction vector and scaled by the fov coefficient
            camera_plane: vec2(direction_vector.y, -direction_vector.x).clamp_length(FIELD_OF_VIEW, FIELD_OF_VIEW),
            time_context: ggez::timer::TimeContext::new(),
        }
    }

    fn rotate(&mut self, angle: f32) {
        let rotation_matrix = Mat2::from_cols_array(&[angle.cos(), angle.sin(), -angle.sin(), angle.cos()]);
        self.direction_vector = rotation_matrix.mul_vec2(self.direction_vector);
        self.camera_plane = rotation_matrix.mul_vec2(self.camera_plane);
    }

    fn handle_input(&mut self, ctx: &mut Context, delta: f32) {
        let player_x = self.player_position.x;
        let player_y = self.player_position.y;
        let direction_x = self.direction_vector.x;
        let direction_y = self.direction_vector.y;
        if ctx.keyboard.is_key_pressed(KeyCode::W) {
            if let None = self.map[player_y as usize][(player_x + direction_x) as usize] {
                self.player_position.x += direction_x * MOVE_SPEED * delta;
            }
            if let None = self.map[(player_y + direction_y) as usize][player_x as usize] {
                self.player_position.y += direction_y * MOVE_SPEED * delta;
            }
        }
        if ctx.keyboard.is_key_pressed(KeyCode::S) {
            if let None = self.map[player_y as usize][(player_x - direction_x) as usize] {
                self.player_position.x -= direction_x * MOVE_SPEED * delta;
            }
            if let None = self.map[(player_y - direction_y) as usize][player_x as usize] {
                self.player_position.y -= direction_y * MOVE_SPEED * delta;
            }
        }
        if ctx.keyboard.is_key_pressed(KeyCode::A) {
            self.rotate(ROTATION_SPEED * delta);
        }
        if ctx.keyboard.is_key_pressed(KeyCode::D) {
            self.rotate(-ROTATION_SPEED * delta);
        }
    }
}

impl event::EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let delta = self.time_context.delta().as_secs_f32();
        self.handle_input(ctx, delta);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {

        let mut canvas = graphics::Canvas::from_frame(
            ctx,
            graphics::Color::from_rgb(0, 0, 0)
        );

        // ---- THIS IS WHERE THE RAYCASTING HAPPENS ----
        // Algorithm courtesy of Lode's Computer Graphics Tutorial
        // https://lodev.org/cgtutor/raycasting.html
        // Rustified and adapted by me
        for x in 0..(X_RESOLUTION as u32) {
            let x = x as f32; // Re-floatify x to enable use in graphics drawing
            // Create a direction vector for the ray
            let camera_x = 2.0 * x / X_RESOLUTION - 1.0;
            let ray_direction = self.direction_vector + self.camera_plane * camera_x;
            // Set up DDA
            let mut map_x = self.player_position.x as i32;
            let mut map_y = self.player_position.y as i32;
            let delta_x = match ray_direction.x == 0.0 {true => 99999999.9, false => (1.0 / ray_direction.x).abs()};
            let delta_y = match ray_direction.y == 0.0 {true => 99999999.9, false => (1.0 / ray_direction.y).abs()};
            let (x_step, mut x_distance) = match ray_direction.x < 0.0 {
                true => (-1, (self.player_position.x - map_x as f32) * delta_x),
                false => (1, (map_x as f32 + 1.0 - self.player_position.x) * delta_x)
            };
            let (y_step, mut y_distance) = match ray_direction.y < 0.0 {
                true => (-1, (self.player_position.y - map_y as f32) * delta_y),
                false => (1, (map_y as f32 + 1.0 - self.player_position.y) * delta_y)
            };
            let mut hit = false;
            let mut side = Side::EastWest;
            let mut wall_type = Wall::Fallback;
            // Execute DDA
            while !hit {
                if x_distance < y_distance {
                    x_distance += delta_x;
                    map_x += x_step;
                    side = Side::EastWest;
                }
                else {
                    y_distance += delta_y;
                    map_y += y_step;
                    side = Side::NorthSouth;
                }
                if let Some(wall) = self.map[map_y as usize][map_x as usize] {
                    hit = true;
                    wall_type = wall;
                } 
            }
            let perpendicular_distance = match side {
                Side::EastWest => x_distance - delta_x,
                Side::NorthSouth => y_distance - delta_y
            };
            // Draw the line
            let mut color = match wall_type {
                Wall::Brick => Color::RED,
                Wall::Fallback => Color::MAGENTA,
            };
            match side {
                Side::EastWest => {
                    color.r -= 0.25;
                    color.g -= 0.25;
                    color.b -= 0.25;
                }
                _ => {},
            }
            let length = Y_RESOLUTION / perpendicular_distance;
            let line = Mesh::new_line(ctx,
                &[vec2(0.0, 0.0), vec2(0.0, length)],
                1.0,
                color
            )?;
            let y0 = Y_RESOLUTION / 2.0 - length / 2.0;
            canvas.draw(&line, vec2(x, y0));
        }

        canvas.finish(ctx)?;
        Ok(())
    }
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
    let map_string = std::fs::read_to_string(MAP_PATH).expect("Failed reading map file");
    let map = parse_map(&map_string);
    let state = GameState::new(map, vec2(3.0, 3.0), vec2(0.0, -1.0));
    event::run(context, events, state);
}
