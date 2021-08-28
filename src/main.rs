// A simple interactive fluid-dynamics simulation
// Colin McNamee <colinomcnamee@gmail.com>
// Dylan McNamee <dylan.mcnamee@gmail.com>

use macroquad::prelude::*;
use shipyard::{
    AddComponent, AllStoragesViewMut, Component, EntitiesViewMut, IntoIter, IntoWithId, SparseSet,
    UniqueView, UniqueViewMut, View, ViewMut, Workload, World,
};
use std::process;
use macroquad::color;
// use turtle_graphics::{Canvas, Turtle};

const WIDTH: i32 = 640;
const HEIGHT: i32 = 360;

const CELLS_X: i32 = 20;
const CELLS_Y: i32 = 12;

#[derive(Debug, Component)]
enum GameOver {
    Lose,
    Victory,
}

pub struct Point2 {
    pub x: f32,
    pub y: f32,
}
#[derive(Component, PartialEq)]
pub struct GameModeInfo{
    pub game_mode: GameMode,
}
#[derive(PartialEq)]

pub enum GameMode {
    Debug,
    Default,
}

#[derive(Component)]
pub struct Cells {
    pub all_cells: Vec<FluidCell>,
}

#[derive(Component)]
pub struct Boat {
    pub loc: Point2,
    pub vel: Vec2,
    pub health: f32, // or some other per-boat state
    t: Turtle,
}

pub fn new_boat(x: f32, y: f32, vx: f32, vy: f32) -> Boat {
    Boat { loc: Point2 {x: x, y: y}, vel: Vec2::new(vx, vy), health: 1., t: new_turtle()}
}

impl Boat {
    pub fn render(&mut self) {
        self.t.direction = (self.vel.y/self.vel.x).atan();
        self.t.pen_down();
        self.t.move_to(self.loc.x, self.loc.y);
        self.t.forward(10.);
        self.t.turn_right(90.);
        self.t.forward(10.);
        self.t.turn_right(90.);
        self.t.forward(10.);
        self.t.turn_right(90.);
        self.t.forward(10.);
    }
}

pub struct FluidCell {
    pub flow_v: Vec2,
    pub flow_updates: Vec2, 
    pub particle_count: u32,
}
#[derive(Component)]
pub struct ParticleDragger {
    pub point_x: f32,
    pub point_y: f32,
}

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub position: Point2,
    pub size: f32,
}

impl Particle {
    fn update_pos(&mut self) -> () {
        self.position.x = self.position.x + self.velocity.x;
        while self.position.x < 0. {
            self.position.x += WIDTH as f32;
        }
        while self.position.x >= (WIDTH as f32) {
            self.position.x -= WIDTH as f32;
        }
        self.position.y = self.position.y + self.velocity.y;
        while self.position.y < 0. {
            self.position.y += HEIGHT as f32;
        }
        while self.position.y >= HEIGHT as f32 {
            self.position.y -= HEIGHT as f32;
        }
    }

    pub fn get_cell_index(&self) -> usize {
        let cell_width = (WIDTH as f32/ CELLS_X as f32).ceil();
        let cell_height = (HEIGHT as f32 / CELLS_Y as f32).ceil();
        let particle_coord_x = (self.position.x / cell_width).floor() as usize;
        let particle_coord_y = (self.position.y / cell_height).floor() as usize;
        let mut ret = particle_coord_y * CELLS_X as usize + particle_coord_x;
        if ret >= (CELLS_X * CELLS_Y) as usize {
            println!("uh oh, {},{} -> {} (y's f32: {})", particle_coord_x, particle_coord_y, ret, self.position.y);
            ret = (CELLS_X * CELLS_Y - 1) as usize;
        }
        ret     
    }

    fn update_velocity_from_cell(&mut self, cell: &FluidCell) {
        self.velocity.x = lerp (self.velocity.x, cell.flow_v.x, 0.03);
        self.velocity.y = lerp (self.velocity.y, cell.flow_v.y, 0.03);
    }
    fn update_velocity_from_mouse(&mut self, x: f32, y: f32) {
        self.velocity.x = lerp (self.velocity.x, x, 0.02);
        self.velocity.y = lerp (self.velocity.y, y, 0.02);
    }
    fn render(&self) {
        let line_length_multiplier = 8.0;
        let indicator_line_x = self.position.x + self.velocity.x * line_length_multiplier;
        let indicator_line_y = self.position.y + self.velocity.y * line_length_multiplier;
        let vel_magnitude = pythag_dist(0., 0., self.velocity.x, self.velocity.y);
        let line_color = color::hsl_to_rgb(1.8 - vel_magnitude / 6.,1.,0.5);
        draw_line(self.position.x, self.position.y, indicator_line_x, indicator_line_y, 0.5, line_color);
        //TODO: lil arrows lines!
        //draw_line(indicatorLineX, indicatorLineY, 0., 0., 0.5, BLACK);
        //draw_circle(self.position.x, self.position.y, 1.0, BLACK);
        // where color = BLUE, GREEN, YELLOW, ... https://docs.rs/macroquad/0.3.8/macroquad/color/index.html
        // println!("particle: ({}, {}) v: ({}, {})", particle.position.x, particle.position.y, particle.velocity.x, particle.velocity.y );
    }
}

impl FluidCell {
    // cache an update to this cell's flow according to a particle in it
    // call by each particle in this cell
    fn update_flow(&mut self, particle: &Particle) {
        self.flow_updates.x += particle.velocity.x;
        self.flow_updates.y += particle.velocity.y;
        self.particle_count += 1;
    }

    // apply the updates to this cell (call once per timestep)
    fn apply_flow_update(&mut self) {
        if self.particle_count > 0 {
            self.flow_v.x = lerp (self.flow_v.x, self.flow_updates.x / self.particle_count as f32, 0.1 );
            self.flow_v.y = lerp (self.flow_v.y, self.flow_updates.y / self.particle_count as f32, 0.1);
            self.flow_updates.x = 0.;
            self.flow_updates.y = 0.;
            self.particle_count = 0;
        }
    }

    fn render(&self, x_coord: i32, y_coord: i32) {
        let cell_width: f32 = WIDTH as f32 / CELLS_X as f32;
        let cell_height: f32 = HEIGHT as f32 / CELLS_Y as f32;
        let cell_middle_x = cell_width as f32 / 2. + cell_width as f32 * x_coord as f32;
        let cell_middle_y = cell_height as f32 / 2. + cell_height as f32 * y_coord as f32;
        let cell_vector_size = 20.;
        draw_circle(cell_middle_x, cell_middle_y, 0.8, WHITE);
        draw_line(cell_middle_x, cell_middle_y, cell_middle_x + self.flow_v.x * cell_vector_size, cell_middle_y + self.flow_v.y * cell_vector_size,  0.5, WHITE);
        //draw_line(cell_middle_x + 5., cell_middle_y, cell_middle_x + 5. + self.flow_updates.x * cell_vector_size, cell_middle_y + self.flow_updates.y * cell_vector_size,  0.7, DARKGREEN);

    }
}

fn lerp (start: f32, target: f32, fraction: f32) -> f32 {
    start + (target - start) * fraction
}

/// generates a new random particle.
fn new_particle() -> Particle {
    Particle { 
        position: Point2 {
            x: rand::gen_range(0., WIDTH as f32),
            y: rand::gen_range(0., HEIGHT as f32),
        },
        size: 1.,
        velocity: Vec2::new(rand::gen_range(-1., 1.), rand::gen_range(-1., 1.)),
    }
}

fn new_particle_at(x: f32, y: f32, vx: f32, vy: f32) -> Particle {
    Particle {position: Point2 {x, y},
              size: 1.,
              velocity: Vec2::new(vx, vy)}
}

fn new_cells() -> Cells {
    let len: usize = CELLS_X as usize * CELLS_Y as usize;
    let mut ret = Vec::with_capacity(len);
    for _i in 0 .. len {
        ret.push(FluidCell{ flow_v: Vec2::new(rand::gen_range(-1., 1.), rand::gen_range(-1., 1.)), 
                            flow_updates: Vec2::new (0.,0.),
                            particle_count: 0, 
                        });
    }
    Cells{all_cells: ret}

}

pub struct Turtle {
    loc: Point2,
    direction: f32,
    pen_down: bool,
    color: Color,
    line_width: f32,
}

// creates a new turtle at x,y, pen is up
pub fn new_turtle() -> Turtle {
    Turtle { loc: Point2 {x: 0., y: 0.}, direction: 0., pen_down: false, line_width: 10., color: WHITE}
}

impl Turtle {
    pub fn forward(&mut self, amount: f32) {
        let (old_x, old_y) = (self.loc.x, self.loc.y);
        self.loc.x = old_x + amount * self.direction.cos();
        self.loc.y = old_y + amount * self.direction.sin();
        if self.pen_down { 
            draw_line(old_x, old_y, self.loc.x, self.loc.y, self.line_width, self.color);
        }
    }
    pub fn turn_right(&mut self, degrees: f32) {
        self.direction += degrees;
    }
    pub fn turn_left(&mut self, degrees: f32) {
        self.direction -= degrees;
    }
    pub fn pen_down(&mut self) {
        self.pen_down = true;
    }
    pub fn pen_up(&mut self) {
        self.pen_down = false;
    }
    pub fn set_color(&mut self, new_color: Color){
        self.color = new_color;
    }
    pub fn set_line_width(&mut self, new_width: f32) {
        self.line_width = new_width;
    }
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.loc.x = x;
        self.loc.y = y;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Particle Man".to_owned(),
        window_width: WIDTH,
        window_height: HEIGHT,
        ..Default::default()
    }
}

fn init_world(world: &mut World) {
    let _ = world.remove_unique::<Particle>();

    // create the grid
    // world.add_unique( ... ).unwrap();

    world.bulk_add_entity((0..8).map(|_| (new_particle(), )));
    world.add_unique(new_cells()).unwrap();
    world.add_unique(ParticleDragger{point_x:0.,point_y:0.}).unwrap();
    world.add_unique(GameModeInfo{game_mode: GameMode::Default}).unwrap();
    world.add_unique(new_boat(WIDTH as f32 / 2., HEIGHT as f32 / 2., 0.5, 0.)).unwrap();
}

// Entry point of the program
#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();

    init_world(&mut world);

    // seed the random number generator with a random value
    rand::srand(macroquad::miniquad::date::now() as u64);

    Workload::builder("Game loop")
        .with_system(move_particle)
        .with_system(drag_particles)
        .with_system(update_grid_flow)
        .with_system(render)
        .with_system(apply_grid_updates)
        .with_system(update_particles_vectors)
        .with_system(handle_key_presses)
        .with_try_system(clean_up)
        .with_system(draw_world_grid)
        .add_to_world(&world)
        .unwrap();

    let mut is_started = false;
    loop {
        if is_started {

            clear_background(BLACK);

            if let Err(Some(err)) = world
                .run_default()
                .map_err(shipyard::error::RunWorkload::custom_error)
            {
                match err.downcast_ref::<GameOver>().unwrap() {
                    GameOver::Lose => debug!("GameOver"),
                    GameOver::Victory => debug!("Victory"),
                }

                is_started = false;
                world.clear();
                init_world(&mut world);
            }
        } else {
            if is_mouse_button_pressed(MouseButton::Left) {
                is_started = true;

                unsafe {
                    get_internal_gl().quad_context.show_mouse(false);
                }
            }

            clear_background(BLACK);

            let text_dimensions = measure_text("Click to start", None, 40, 1.);
            draw_text(
                "Click to start",
                WIDTH as f32 / 2. - text_dimensions.width / 2.,
                HEIGHT as f32 / 2. - text_dimensions.height / 2.,
                40.,
                WHITE,
            );
        }

        next_frame().await
    }
}

fn move_particle(mut particles: ViewMut<Particle>) -> Result<(), GameOver> {
    for particle in (&mut particles).iter() {
        particle.update_pos();
    }
    Ok(())
}
fn drag_particles(mut dragger: UniqueViewMut<ParticleDragger>,
                  mut particles: ViewMut<Particle>,
                  mut entities: EntitiesViewMut,
                  mut player:UniqueViewMut<Boat>,){
    let (mouse_x, mouse_y) = mouse_position();
    if is_mouse_button_down(MouseButton::Left) {
        dragger.point_x = lerp(dragger.point_x, mouse_x, 0.3);
        dragger.point_y = lerp(dragger.point_y, mouse_y, 0.3);
        player.loc.x = dragger.point_x;
        player.loc.y = dragger.point_y;
        player.vel.x = 0.2 * (mouse_x - dragger.point_x); // set the velocity vector
        player.vel.y = 0.2 * (mouse_y - dragger.point_y);
        entities.add_entity((particles,), (new_particle_at(mouse_x, mouse_y, 
                                                            0.2 * (dragger.point_x - mouse_x), 
                                                            0.2 * (dragger.point_y - mouse_y),),));
    }else{
        dragger.point_x = mouse_x;
        dragger.point_y = mouse_y;
    }
    player.render();
    if is_mouse_button_pressed(MouseButton::Right) {
    }
}

pub fn pythag_dist(x1: f32, y1: f32, x2: f32, y2: f32,) -> f32 {
    let xd = x2 - x1;
    let yd = y2 - y1;
    (xd * xd + yd * yd).sqrt()
}

// handle key presses for game mode changes
fn handle_key_presses(mut game_mode: UniqueViewMut<GameModeInfo>) {
    if is_key_pressed(KeyCode::Space){
        if game_mode.game_mode == GameMode::Debug{
            game_mode.game_mode = GameMode::Default
        }else{
            game_mode.game_mode = GameMode::Debug
        }
    }
    if is_key_pressed(KeyCode::Escape){
        process::exit(1);
    }
}

// debugging utility: draw the grid lines
fn draw_world_grid(game_mode: UniqueView<GameModeInfo>) {
    if game_mode.game_mode == GameMode::Debug{
        let cell_width: f32 = WIDTH as f32 / CELLS_X as f32;
        let cell_height: f32 = HEIGHT as f32 / CELLS_Y as f32;
        for x  in 1..CELLS_X {
            draw_line( x as f32 * cell_width, 0., 
                       x as f32 * cell_width, HEIGHT as f32, 0.5, WHITE);
        }
        for y  in 1..CELLS_Y {
            draw_line(0., y as f32 * cell_height, 
                    WIDTH as f32, y as f32 * cell_height, 0.5, WHITE);
        }
    }

}

// have the particles update the cells they're in
fn update_grid_flow(particles: View<Particle>, mut map:UniqueViewMut<Cells>) -> Result<(), GameOver> {
    for particle in particles.iter() {
        let cell_index = particle.get_cell_index();
        map.all_cells[cell_index].update_flow(particle);
    }
    Ok(())
}

// apply the updates to the cells
fn apply_grid_updates(mut map:UniqueViewMut<Cells>) -> Result<(), GameOver> {
    for cell_ix in 0..map.all_cells.len() {
        map.all_cells[cell_ix].apply_flow_update();
    }
    Ok(())
}

// render a frame of the world
// documentation here: https://docs.rs/macroquad/0.3.8/macroquad/
fn render(particles: View<Particle>, 
          map: UniqueView<Cells>, 
          game_mode: UniqueView<GameModeInfo> ) -> Result<(), GameOver>
{
    for particle in particles.iter() {
        particle.render();
    }
    let mut cell_x = 0;
    let mut cell_y = 0;
    if game_mode.game_mode == GameMode:: Debug{
        for cell in map.all_cells.iter() {
            cell.render(cell_x, cell_y);
            cell_x += 1;
            if cell_x >= CELLS_X {
                cell_x = 0;
                cell_y += 1;
            }
        }
    }
    Ok(())
}

// update each particle's vector according to the flow of the cell it's in
fn update_particles_vectors(mut particles: ViewMut<Particle>, map:UniqueView<Cells> ) -> Result<(), GameOver> {
    for particle in (&mut particles).iter() {
        let cell_index = particle.get_cell_index();
        let cell = &map.all_cells[cell_index];
        // update particle's vector according to its cell;
        particle.update_velocity_from_cell(cell);
    }
    Ok(())
}

// TODO: define Renderable trait with render function
// impl render(&self) for Particle {
// }

fn clean_up(mut all_storages: AllStoragesViewMut) -> Result<(), GameOver> {
    Ok(())
}

impl std::error::Error for GameOver {}

impl std::fmt::Display for GameOver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
