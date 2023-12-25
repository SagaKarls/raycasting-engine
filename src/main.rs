use std::usize;
use ggez::{
    self,
    event,
    graphics::{self, Mesh, Color, DrawMode, Image, DrawParam, Rect, Text, Quad, InstanceArray, Canvas},
    Context,
    GameError,
    input::keyboard::KeyCode,
    glam::{vec2, Vec2, Mat2}, timer::TimeContext
};

// Gameplay parameters
const MOVE_SPEED: f32 = 2.5; // In units / second
const ROTATION_SPEED: f32 = 1.6; // In radians / second

// Rendering parameters
const X_RESOLUTION: f32 = 640.0;
const Y_RESOLUTION: f32 = 360.0;
const FIELD_OF_VIEW: f32 = 0.60; // works out to ~60 degrees
const TEXTURE_SIZE: u32 = 64;
const PIXEL_FRAC: f32 = 1.0 / TEXTURE_SIZE as f32;
const CAMERA_HEIGHT: f32 = 0.5; // As a fraction of screen height
const HORIZON_HEIGHT: f32 = 0.5; // As a fraction of screen height

// Misc parameters
const MAP_PATH: &str = "map.txt";

#[derive(PartialEq, Eq)]
enum Side {
    NorthSouth,
    EastWest
}

struct Player {
    position: Vec2,
    direction: Vec2,
    camera: Vec2,
}

impl Player {
    fn rotate(&mut self, angle: f32) {
        let rotation_matrix = Mat2::from_cols_array(&[angle.cos(), angle.sin(), -angle.sin(), angle.cos()]);
        self.direction = rotation_matrix.mul_vec2(self.direction);
        self.camera = rotation_matrix.mul_vec2(self.camera);
    }
}

struct Gfx {
    wall_textures: Vec<Image>,
    floor_batch: InstanceArray,
    ceiling_batch: InstanceArray,
}

struct Level {
    map: Vec<Vec<Option<usize>>>,
    decorations: Vec<Decoration>
}

trait Sprite {
    fn sprite(&self) -> Image;
    fn position(&self) -> Vec2;

    fn draw(&self, canvas: &mut Canvas, player: &Player) {
        let sprite = self.sprite();
        let relative_position = self.position() - player.position;
        let transform_matrix = Mat2::from_cols(
            Vec2::new(player.camera.x, player.camera.y),
            Vec2::new(player.direction.x, player.direction.y)
        ).inverse();
        let transformed_position = transform_matrix.mul_vec2(relative_position);
        let screen_x = (X_RESOLUTION / 2.0) * (1.0 + transformed_position.x / transformed_position.y);

        let scale = 2.0 / transformed_position.y;
        if scale > 0.0 {
            let param = DrawParam::new()
            .offset(Vec2::new(0.5, 0.5))
            .dest(Vec2::new(screen_x, Y_RESOLUTION / 2.0))
            .scale(Vec2::new(scale, scale))
            .z(-(transformed_position.y * 100.0) as i32);
            canvas.draw(&sprite, param);
        }
    }
}

struct Decoration {
    sprite: Image,
    position: Vec2,
    facing: bool,
}

impl Sprite for Decoration {
    fn sprite(&self) -> Image {self.sprite.clone()}
    fn position(&self) -> Vec2 {self.position}
}

impl Decoration {
    fn new<T: Into<Vec2>>(ctx: &Context, sprite_path: &str, position: T, facing: bool) -> Result<Decoration, GameError>{
        Ok(
            Decoration {
                sprite: Image::from_path(ctx, sprite_path)?,
                position: position.into(),
                facing
            }
        )
    }
}

struct GameState {
    level: Level,
    player: Player,
    gfx: Gfx,
    time_context: TimeContext
}

impl GameState {
    fn new(ctx: &Context, level: Level, player_position: Vec2, direction_vector: Vec2) -> Result<GameState, GameError> {
        let direction = direction_vector.normalize(); // Make sure it's normalized!!
        let wall_textures = vec![
            Image::from_path(ctx, "/textures/stone.png")?,
            Image::from_path(ctx, "/textures/brick.png")?,
            Image::from_path(ctx, "/textures/wood.png")?,
            Image::from_color(ctx, 64, 64, Some(Color::MAGENTA)),
        ];
        let gfx = Gfx {
            wall_textures,
            floor_batch: InstanceArray::new(ctx, Image::from_path(ctx, "/textures/floor.png")?),
            ceiling_batch: InstanceArray::new(ctx, Image::from_path(ctx, "/textures/ceiling.png")?),
        };
        

        let player = Player {
            position: player_position,
            direction: direction_vector,
            camera: vec2(direction.y, -direction.x).clamp_length(FIELD_OF_VIEW, FIELD_OF_VIEW),
        };

        Ok(GameState {
            level,
            player,
            gfx,
            time_context: TimeContext::new()
        })
    }

    fn handle_input(&mut self, ctx: &mut Context, delta: f32) {
        let player_x = self.player.position.x;
        let player_y = self.player.position.y;
        let direction_x = self.player.direction.x;
        let direction_y = self.player.direction.y;
        if ctx.keyboard.is_key_pressed(KeyCode::W) {
            if let None = self.level.map[player_y as usize][(player_x + direction_x) as usize] {
                self.player.position.x += direction_x * MOVE_SPEED * delta;
            }
            if let None = self.level.map[(player_y + direction_y) as usize][player_x as usize] {
                self.player.position.y += direction_y * MOVE_SPEED * delta;
            }
        }
        if ctx.keyboard.is_key_pressed(KeyCode::S) {
            if let None = self.level.map[player_y as usize][(player_x - direction_x) as usize] {
                self.player.position.x -= direction_x * MOVE_SPEED * delta;
            }
            if let None = self.level.map[(player_y - direction_y) as usize][player_x as usize] {
                self.player.position.y -= direction_y * MOVE_SPEED * delta;
            }
        }
        if ctx.keyboard.is_key_pressed(KeyCode::A) {
            self.player.rotate(ROTATION_SPEED * delta);
        }
        if ctx.keyboard.is_key_pressed(KeyCode::D) {
            self.player.rotate(-ROTATION_SPEED * delta);
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
        self.time_context.tick();

        let mut canvas = graphics::Canvas::from_frame(
            ctx,
            graphics::Color::WHITE,
        );
        canvas.set_screen_coordinates(Rect::new(0.0, 0.0, X_RESOLUTION, Y_RESOLUTION));

        // ---- THIS IS WHERE THE RAYCASTING HAPPENS ----
        // Algorithm courtesy of Lode's Computer Graphics Tutorial
        // https://lodev.org/cgtutor/raycasting.html
        // Rustified and adapted by me
        let mut wall_mask: Vec<f32> = vec![]; // Keep track of where the wall starts for every screenspace x
        // --- Create wall batches ---
        for x in 0..(X_RESOLUTION as u32) {
            let x = x as f32; // Re-floatify x to enable use in graphics drawing
            // Create a direction vector for the ray
            let camera_x = 2.0 * x / X_RESOLUTION - 1.0;
            let ray_direction = self.player.direction + self.player.camera * camera_x;
            // Set up DDA
            let mut map_x = self.player.position.x as i32;
            let mut map_y = self.player.position.y as i32;
            let delta_x = match ray_direction.x == 0.0 {true => 99999999.9, false => (1.0 / ray_direction.x).abs()};
            let delta_y = match ray_direction.y == 0.0 {true => 99999999.9, false => (1.0 / ray_direction.y).abs()};
            let (x_step, mut x_distance) = match ray_direction.x < 0.0 {
                true => (-1, (self.player.position.x - map_x as f32) * delta_x),
                false => (1, (map_x as f32 + 1.0 - self.player.position.x) * delta_x)
            };
            let (y_step, mut y_distance) = match ray_direction.y < 0.0 {
                true => (-1, (self.player.position.y - map_y as f32) * delta_y),
                false => (1, (map_y as f32 + 1.0 - self.player.position.y) * delta_y)
            };
            let mut hit = false;
            let mut side = Side::EastWest;
            let mut texture_index = usize::MAX;
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
                if let Some(index) = self.level.map[map_y as usize][map_x as usize] {
                    hit = true;
                    texture_index = index;
                } 
            }
            let perpendicular_distance = match side {
                Side::EastWest => x_distance - delta_x,
                Side::NorthSouth => y_distance - delta_y
            };
            // Create draw params
            let wall_x = match side {
                Side::EastWest => self.player.position.y + perpendicular_distance * ray_direction.y,
                Side::NorthSouth => self.player.position.x + perpendicular_distance * ray_direction.x,
            };
            let wall_x = wall_x - wall_x.floor();
            let mut texture_x = wall_x * TEXTURE_SIZE as f32;
            if (side == Side::EastWest && ray_direction.x > 0.0)
            || (side == Side::NorthSouth && ray_direction.y < 0.0) {
                texture_x = TEXTURE_SIZE as f32 - texture_x - 1.0;
            }
            let height = Y_RESOLUTION / perpendicular_distance;
            let y0 = Y_RESOLUTION / 2.0 - height / 2.0;
            let params = DrawParam::new()
            .src(Rect::new(texture_x * PIXEL_FRAC, 0.0, PIXEL_FRAC, 1.0))
            .dest(vec2(x, y0))
            .scale(vec2(1.0, height * PIXEL_FRAC))
            .z(-(perpendicular_distance * 100.0) as i32);
            texture_index = texture_index.clamp(0, self.gfx.wall_textures.len() - 1);
            canvas.draw(&self.gfx.wall_textures[texture_index], params);
            wall_mask.push(y0);
        }

        // --- Create floor/ceiling batches ---
        self.gfx.floor_batch.clear();
        self.gfx.ceiling_batch.clear();
        for y in 0..(Y_RESOLUTION as u32 / 2) {
            let y = y as f32;
            let ray_left = self.player.direction - self.player.camera;
            let ray_right = self.player.direction + self.player.camera;
            let horizon_distance = y - Y_RESOLUTION * HORIZON_HEIGHT;
            let camera_height = Y_RESOLUTION * CAMERA_HEIGHT;
            let row_distance = camera_height / horizon_distance;
            let x_step = row_distance * (ray_right.x - ray_left.x) / X_RESOLUTION;
            let y_step = row_distance * (ray_right.y - ray_left.y) / X_RESOLUTION;
            let mut floor_x = row_distance * ray_left.x - self.player.position.x;
            let mut floor_y = row_distance * ray_left.y - self.player.position.y;
            for x in 0..(X_RESOLUTION as u32) {
                let cell_x = floor_x.floor();
                let cell_y = floor_y.floor();
                let texture_x =  floor_x - cell_x;
                let texture_y = floor_y - cell_y;
                floor_x += x_step;
                floor_y += y_step;
                if wall_mask[x as usize] < y {
                    continue;
                }
                let x = x as f32;
                let src_rect = Rect::new(texture_x, texture_y, PIXEL_FRAC, PIXEL_FRAC);
                // Add floor to batch
                let floor_params = DrawParam::new().src(src_rect).dest(vec2(x, Y_RESOLUTION - y - 1.0));
                self.gfx.floor_batch.push(floor_params);
                // Add ceiling to batch
                let ceiling_params = DrawParam::new().src(src_rect).dest(vec2(x, y));
                self.gfx.ceiling_batch.push(ceiling_params);
                
            }
        }

        // -- Draw decorations --
        for item in &self.level.decorations {
            item.draw(&mut canvas, &mut self.player)
        }

        // -- Draw batched textures --
        // floor and ceiling
        canvas.draw(&self.gfx.floor_batch, DrawParam::new().z(i32::MIN));
        canvas.draw(&self.gfx.ceiling_batch, DrawParam::new().z(i32::MIN));
        // Draw FPS counter
        let fps = self.time_context.fps();
        let fps_counter = Text::new(format!("{:.2}", fps));
        canvas.draw(&fps_counter, vec2(0.0, 0.0));

        canvas.finish(ctx)?;
        Ok(())
    }
}

/// Converts an ASCII art representation of a map to a matrix of tiles
fn parse_map(map_str: &str) -> Vec<Vec<Option<usize>>> {
    return map_str
        .trim()
        .lines()
        .map(|line| {
            line.chars()
                .map(|char| match char {
                    '.' => None,
                    'S' => Some(0),
                    'B' => Some(1),
                    'W' => Some(2),
                    _ => Some(usize::MAX),
                })
                .collect::<Vec<Option<usize>>>()
        })
        .collect::<Vec<Vec<Option<usize>>>>();
}

fn main() {
    // ----GGEZ setup----
    let setup = ggez::conf::WindowSetup::default().title("Raycast test");
    let builder = ggez::ContextBuilder::new("Raycast test", "sagakar").window_setup(setup);
    let (mut context, events) = builder.build().expect("Failed to build context");
    let window_mode = ggez::conf::WindowMode::default()
    .borderless(true)
    .fullscreen_type(ggez::conf::FullscreenType::Desktop);
    context.gfx.set_mode(window_mode).expect("Failed to set window mode");

    // ----Game state setup----
    let map_string = std::fs::read_to_string(MAP_PATH).expect("Failed reading map file");
    let map = parse_map(&map_string);
    let level = Level {
        map,
        decorations: vec![
            //Decoration::new(&context, "/cat.png", Vec2::new(6.0, 4.0), false).unwrap(),
        ]
    };
    // Create the texture hashmap
    let state = GameState::new(
        &context,
        level,
        vec2(3.0, 3.0),
        vec2(0.0, -1.0)
    ).expect("Failed to construct game instance");

    // ----Put it all together----
    event::run(context, events, state);
}