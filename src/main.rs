use std::collections::BTreeSet;

use oorandom::Rand32;
use ggez::{
    event, graphics::{self, Color},
    input::keyboard::{KeyCode, KeyInput, KeyMods},
    Context, GameResult,
};

// The first thing we want to do is set up some constants that will help us out later.

const GAME_GRID_WIDTH: usize = 10;
const GAME_GRID_HEIGHT: usize = 20;

const FULL_GRID_SIZE: (i8, i8) = (20, 30);
const GAME_GRID_SIZE: (i8, i8) = (GAME_GRID_WIDTH as i8, GAME_GRID_HEIGHT as i8);
const GRID_CELL_SIZE: (i8, i8) = (32, 32);

// Next we define how large we want our actual window to be by multiplying
// the components of our grid size by its corresponding pixel size.
const SCREEN_SIZE: (f32, f32) = (
    FULL_GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    FULL_GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);

// Here we're defining how often we want our game to update. This will be
// important later so that we don't have our snake fly across the screen because
// it's moving a full tile every frame.
const DESIRED_FPS: u32 = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Pos {
    x: i8,
    y: i8,
}

impl Pos {
    pub const fn new(x: i8, y: i8) -> Self {
        Pos { x, y }
    }
}

impl From<Pos> for graphics::Rect {
    fn from(pos: Pos) -> Self {
        const START_X: i32 = (FULL_GRID_SIZE.0 - GAME_GRID_SIZE.0) as i32 / 2;
        const START_Y: i32 = (FULL_GRID_SIZE.1 - GAME_GRID_SIZE.1) as i32;
        graphics::Rect::new_i32(
            (START_X + pos.x as i32) * GRID_CELL_SIZE.0 as i32,
            (START_Y + pos.y as i32) * GRID_CELL_SIZE.1 as i32,
            GRID_CELL_SIZE.0 as i32,
            GRID_CELL_SIZE.1 as i32,
        )
    }
}

/// And here we implement `From` again to allow us to easily convert between
/// `(i8, i8)` and a `Pos`.
impl From<(i8, i8)> for Pos {
    fn from(pos: (i8, i8)) -> Self {
        Pos { x: pos.0, y: pos.1 }
    }
}

const NUM_COLOURS: usize = 7;
const COLOURS: [Color; NUM_COLOURS] = [
    Color::new(0.5, 0., 0.5, 1.),
    Color::RED,
    Color::YELLOW,
    Color::GREEN,
    Color::CYAN,
    Color::BLUE,
    Color::WHITE,
];

struct Grid {
    grid: [[u8; GAME_GRID_WIDTH]; GAME_GRID_HEIGHT],
}

impl Grid {
    pub const fn new() -> Self {
        Grid {
            grid: [[255; GAME_GRID_WIDTH]; GAME_GRID_HEIGHT],
        }
    }

    fn draw(&self, canvas: &mut graphics::Canvas) {
        for (y, row) in self.grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                let i = c as usize;
                if i < NUM_COLOURS {
                    canvas.draw(
                        &graphics::Quad,
                        graphics::DrawParam::new()
                            .dest_rect(Pos::new(x as i8, y as i8).into())
                            .color(COLOURS[c as usize]),
                    );
                } else {
                    canvas.draw(
                        &graphics::Quad,
                        graphics::DrawParam::new()
                            .dest_rect(Pos::new(x as i8, y as i8).into())
                            .color(Color::MAGENTA),
                    );
                }
            }
        }
    }

    fn check_for_line(&mut self, y: i8) -> bool {
        let done = self.grid[y as usize].iter().all(|&c| (c as usize) < NUM_COLOURS);
        if done {
            for y in (1..=y as usize).rev() {
                self.grid[y] = self.grid[y - 1]; 
            }
            self.grid[0] = [255; 10];
        }
        done
    }
    fn is_free_or_above(&self, pos: Pos) -> bool {
        self.grid
            .get(pos.y as usize)
            .and_then(|row| row.get(pos.x as usize))
            .map(|&c| (c as usize) >= NUM_COLOURS)
            .unwrap_or_else(|| pos.y < 0 && 0 <= pos.x && pos.x < GAME_GRID_SIZE.0)
    }
    fn set(&mut self, pos: Pos, c: u8) -> bool {
        if let Some(g) = self.grid
            .get_mut(pos.y as usize)
            .and_then(|row| row.get_mut(pos.x as usize)) {
                *g = c;
                true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Piece {
    colour: u8,
    offsets: [Pos; 4],
}

impl Piece {
    fn get_random(rng: &mut Rand32) -> Self {
        let colour = rng.rand_range(0..7) as u8;
        let offsets = match colour {
            0 => [Pos::new(-1, -1), Pos::new(0, -1), Pos::new(1, -1), Pos::new(-1, 0)],
            1 => [Pos::new(-1, 0), Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)],
            2 => [Pos::new(-1, -1), Pos::new(0, -1), Pos::new(1, -1), Pos::new(0, 0)],
            3 => [Pos::new(0, -1), Pos::new(1, -1), Pos::new(-1, 0), Pos::new(0, 0)],
            4 => [Pos::new(-1, -1), Pos::new(0, -1), Pos::new(0, 0), Pos::new(1, 0)],
            5 => [Pos::new(-1, -1), Pos::new(0, -1), Pos::new(-1, 0), Pos::new(0, 0)],
            6 => [Pos::new(-1, -1), Pos::new(0, -1), Pos::new(1, -1), Pos::new(1, 0)],
            _ => unreachable!(),
        };
        Piece {
            colour,
            offsets,
        }
    }
    // TODO: handle rotation properly
    fn rotate_left(&mut self) {
        for offset in &mut self.offsets {
            let old_x = offset.x;
            offset.x = offset.y;
            offset.y = -old_x;
        }
    }
    fn rotate_right(&mut self) {
        for offset in &mut self.offsets {
            let old_x = offset.x;
            offset.x = -offset.y;
            offset.y = old_x;
        }
    }
    fn points<'a>(&'a self, offset: Pos) -> impl Iterator<Item=Pos> + use<'a> {
        self.offsets.iter().map(move |p| Pos::new(offset.x + p.x, offset.y + p.y))
    }
    fn draw(&self, canvas: &mut graphics::Canvas, at: Pos) {
        let colour = COLOURS[self.colour as usize];
        for pos in self.points(at) {
            canvas.draw(
                &graphics::Quad,
                graphics::DrawParam::new()
                    .dest_rect(pos.into())
                    .color(colour),
            );
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MovingPiece {
    pos: Pos,
    piece: Piece,
}

impl MovingPiece {
    fn new(piece: Piece) -> Self {
        MovingPiece {
            pos: Pos::new(GAME_GRID_SIZE.0 / 2, -2),
            piece,
        }
    }
    fn draw(&self, canvas: &mut graphics::Canvas) {
        self.piece.draw(canvas, self.pos);
    }
}

const FRAMES_PER_MOVE: u8 = 18;

struct GameState {
    grid: Grid,
    gameover: bool,
    move_frames: u8,
    score: u32,
    rng: Rand32,
    next_piece: Piece,
    cur_piece: Option<MovingPiece>,
}

enum Move {
    Left, Right, RotLeft, RotRight,
}

impl GameState {
    /// Our new function will set up the initial state of our game.
    pub fn new() -> Self {
        let mut seed: [u8; 8] = [0; 8];
        getrandom::getrandom(&mut seed[..]).expect("Could not create RNG seed");
        let mut rng = Rand32::new(u64::from_ne_bytes(seed));

        GameState {
            grid: Grid::new(),
            gameover: false,
            next_piece: Piece::get_random(&mut rng),
            cur_piece: None,
            move_frames: 0,
            score: 0,
            rng,
        }
    }
    fn mv(&mut self, mv: Move) {
        if let Some(mp) = &mut self.cur_piece {
            let mut new_mp = mp.clone();
            match mv {
                Move::Left => new_mp.pos.x -= 1,
                Move::Right => new_mp.pos.x += 1,
                Move::RotLeft => new_mp.piece.rotate_left(),
                Move::RotRight => new_mp.piece.rotate_right(),
            }
            for pos in new_mp.piece.points(new_mp.pos) {
                if !self.grid.is_free_or_above(pos) {
                    return;
                }
            }
            *mp = new_mp;
        }
    }
    
    fn move_down(&mut self) {
        self.move_frames += FRAMES_PER_MOVE / 2;
    }
}

impl event::EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        while ctx.time.check_update_time(DESIRED_FPS) {
            let move_frame = {
                self.move_frames += 1;
                if self.move_frames > FRAMES_PER_MOVE {
                    self.move_frames -= FRAMES_PER_MOVE;
                    true
                } else {
                    false
                }
            };

            if !self.gameover {
                if let Some(cur_piece) = &mut self.cur_piece {
                    if move_frame {
                        let new_pos = Pos {x: cur_piece.pos.x, y: cur_piece.pos.y + 1};
                        let mut can_move_down = true;
                        for pos in cur_piece.piece.points(new_pos) {
                            if !self.grid.is_free_or_above(pos) {
                                can_move_down = false;
                                break;
                            }
                        }
                        if can_move_down {
                            cur_piece.pos = new_pos;
                        } else {
                            let mut line_set = BTreeSet::new();
                            let mut out_of_bounds = false;
                            for pos in cur_piece.piece.points(cur_piece.pos) {
                                line_set.insert(pos.y);
                                if !self.grid.set(pos, cur_piece.piece.colour) {
                                    out_of_bounds = true;
                                    break;
                                }
                            }
                            if out_of_bounds {
                                self.gameover = true;
                            } else {
                                self.cur_piece = None;
                                let mut num_cleared = 0;
                                for y in line_set {
                                    if self.grid.check_for_line(y) {
                                        num_cleared += 1;
                                    }
                                }
                                let score = match num_cleared {
                                    0 => 0,
                                    1 => 40,
                                    2 => 100,
                                    3 => 300,
                                    4 => 1200,
                                    _ => unimplemented!(),
                                };
                                self.score += score;
                            }
                        }
                    }
                } else {
                    let piece = std::mem::replace(&mut self.next_piece, Piece::get_random(&mut self.rng));
                    self.cur_piece = Some(MovingPiece::new(piece));
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        ctx.gfx.set_window_title(&format!("Tetris - Score: {}", self.score));

        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);

        self.next_piece.draw(&mut canvas, Pos::new(-3, -3));

        self.grid.draw(&mut canvas);

        if let Some(p) = &self.cur_piece {
            p.draw(&mut canvas);
        }

        canvas.finish(ctx)?;

        ggez::timer::yield_now();
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, _repeated: bool) -> Result<(), ggez::GameError> {
        let Some(keycode) = input.keycode else {
            return Ok(());
        };
        if input.mods.contains(KeyMods::SHIFT) && keycode == KeyCode::Escape {
            ctx.request_quit();
        }
        if self.gameover {
            return Ok(());
        }

        match keycode {
            KeyCode::A | KeyCode::Left => self.mv(Move::Left),
            KeyCode::D | KeyCode::Right => self.mv(Move::Right),
            KeyCode::Q => self.mv(Move::RotLeft),
            KeyCode::E => self.mv(Move::RotRight),
            KeyCode::S | KeyCode::Down => self.move_down(),
            _ => (),
        }

        Ok(())
    }
}

fn main() -> GameResult {
    let (ctx, events_loop) = ggez::ContextBuilder::new("tetris", "Falch")
        .window_setup(ggez::conf::WindowSetup::default().title("Tetris"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        .build()?;

    let state = GameState::new();
    event::run(ctx, events_loop, state)
}
