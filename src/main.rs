use nannou::geom::Rect;
use nannou::prelude::*;
use nannou::rand::random_range;

const SIZE: isize = 32;

fn main() {
    nannou::app(model).update(update).simple_window(view).run();
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum CellState {
    Enabled,
    Disabled,
}

impl CellState {
    fn get_color(&self) -> Srgb<u8> {
        match self {
            Self::Enabled => WHITE,
            Self::Disabled => BLACK,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    rect: Rect,
    state: CellState,
    marked: bool,
}

impl Cell {
    fn draw(&self, draw: &Draw) {
        let rect = self.rect;

        draw.rect()
            .xy(rect.xy())
            .wh(rect.wh())
            .color(self.state.get_color());

        if self.marked {
            let pad = rect.h() * 0.2;
            let rect2 = rect
                .pad_left(pad)
                .pad_right(pad)
                .pad_top(pad)
                .pad_bottom(pad);
            draw.rect().xy(rect2.xy()).wh(rect2.wh()).color(YELLOW);
        }
    }
}

struct Model {
    field: Vec<Cell>,
    window: Rect,
    initialized: bool,
}

fn model(app: &App) -> Model {
    Model {
        field: init_recs(app.window_rect(), None),
        window: app.window_rect(),
        initialized: false,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let wr = app.window_rect();
    if !wr.eq(&model.window) {
        model.window = wr;
        model.field = init_recs(wr, Some(&model.field));
    }

    mover(app, model);
    // rain(app, model);
    // life(app, model);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.background().color(STEELBLUE);

    model.field.iter().for_each(|cell| cell.draw(&draw.clone()));

    draw.to_frame(app, &frame).unwrap();
}

// simulations
fn mover(app: &App, model: &mut Model) {
    if !model.initialized {
        set_cell_params(&mut model.field, 0, 0, Some(CellState::Enabled), None);
        put_markers(&mut model.field);
        model.initialized = true;
    }

    let (px, py) = get_prev_pos(app);
    let (x, y) = get_next_pos(app);
    if px == x && py == y {
        return;
    }
    set_cell_params(&mut model.field, px, py, Some(CellState::Disabled), None);
    set_cell_params(&mut model.field, x, y, Some(CellState::Enabled), None);
}

fn rain(_app: &App, model: &mut Model) {
    if !model.initialized {
        put_markers(&mut model.field);
        model.initialized = true;
    }

    let enabled_indexes = get_enabled_cells_indexes(&model.field);

    clear_field(&mut model.field);

    // add new drop
    if enabled_indexes.len() < (SIZE * 2) as usize {
        let x = random_range(0, SIZE);
        set_cell_params(&mut model.field, x, 0, Some(CellState::Enabled), None);
    }

    // fall old drops
    for index in enabled_indexes {
        let (x, y) = index_to_pos(index);
        if y + 1 < SIZE {
            set_cell_params(&mut model.field, x, y + 1, Some(CellState::Enabled), None)
        }
    }
}

fn _is_alive(cell: &Option<Cell>) -> bool {
    match cell {
        Some(c) => match c.state {
            CellState::Enabled => true,
            CellState::Disabled => false,
        },
        None => false,
    }
}

fn life(app: &App, model: &mut Model) {
    // init
    if !model.initialized {
        for _ in 0..SIZE * SIZE / 2 {
            let x = random_range(0, SIZE);
            let y = random_range(0, SIZE);
            set_cell_params(&mut model.field, x, y, Some(CellState::Enabled), None);
        }
        put_markers(&mut model.field);
        model.initialized = true;
    }

    let mut next_field = init_recs(model.window, Some(&model.field));

    for x in 0..SIZE {
        for y in 0..SIZE {
            let cell = get_cell(&model.field, x, y);

            let is_alive = _is_alive(&cell);
            let neigbours_cells = get_neighbours_cells(&model.field, x, y);
            let alive_neighbours = neigbours_cells.iter().filter(|&c| _is_alive(c)).count();

            if is_alive {
                match alive_neighbours {
                    1 => set_cell_params(&mut next_field, x, y, Some(CellState::Disabled), None),
                    2 | 3 => set_cell_params(&mut next_field, x, y, Some(CellState::Enabled), None),
                    _ => set_cell_params(&mut next_field, x, y, Some(CellState::Disabled), None),
                }
            } else if alive_neighbours == 3 {
                set_cell_params(&mut next_field, x, y, Some(CellState::Enabled), None)
            }
        }
    }

    model.field = next_field;
}

// utils
fn init_recs(window_rect: Rect, old_field: Option<&Vec<Cell>>) -> Vec<Cell> {
    let mut field: Vec<Cell> = vec![];
    let (side, zone) = get_rect_side_and_zone(window_rect);

    for i in 0..SIZE * SIZE {
        let (x, y) = index_to_pos(i as isize);

        let rect = Rect::from_x_y_w_h(0.0, 0.0, side, side)
            .top_left_of(window_rect)
            .shift_x(x as f32 * zone)
            .shift_y(y as f32 * -zone);

        let state = if let Some(f) = old_field {
            f[i as usize].state
        } else {
            CellState::Disabled
        };

        let marked = if let Some(f) = old_field {
            f[i as usize].marked
        } else {
            false
        };

        field.push(Cell {
            rect,
            state,
            marked,
        });
    }

    field
}

fn put_markers(rects: &mut Vec<Cell>) {
    let marked_count = SIZE * SIZE / 8;

    for _ in 0..marked_count {
        let x = random_range(0, SIZE);
        let y = random_range(0, SIZE);
        rects[pos_to_index((x, y)) as usize].marked = true;
    }
}

fn get_enabled_cells_indexes(rects: &[Cell]) -> Vec<isize> {
    rects
        .iter()
        .enumerate()
        .filter(|&(_, c)| _is_alive(&Some(*c)))
        .map(|(i, _)| i as isize)
        .collect()
}

fn get_cells_by_state(rects: &[Cell], state: CellState) -> Vec<(isize, isize)> {
    rects
        .iter()
        .enumerate()
        .filter(|&(_, v)| v.state == state)
        .map(|(i, _)| index_to_pos(i as isize))
        .collect()
}

fn set_cells_params(
    rects: &mut Vec<Cell>,
    positions: Vec<(isize, isize)>,
    state: Option<CellState>,
    marked: Option<bool>,
) {
    for (x, y) in positions.iter() {
        set_cell_params(rects, *x, *y, state, marked)
    }
}

fn get_neighbours_cells(rects: &[Cell], x: isize, y: isize) -> Vec<Option<Cell>> {
    let mut result: Vec<Option<Cell>> = vec![];

    result.push(get_cell(rects, x - 1, y - 1)); // top left
    result.push(get_cell(rects, x, y - 1)); // top
    result.push(get_cell(rects, x + 1, y - 1)); // top right

    result.push(get_cell(rects, x - 1, y)); // left
    result.push(get_cell(rects, x + 1, y)); // right

    result.push(get_cell(rects, x - 1, y + 1)); // bottom left
    result.push(get_cell(rects, x, y + 1)); // bottom
    result.push(get_cell(rects, x + 1, y + 1)); // bottom right

    result
}

fn get_cell(rects: &[Cell], x: isize, y: isize) -> Option<Cell> {
    if x < 0 || y < 0 {
        return None;
    }

    if x >= SIZE || y >= SIZE {
        return None;
    }

    let index = pos_to_index((x, y)) as isize;
    Some(rects[index as usize])
}

fn clear_field(rects: &mut Vec<Cell>) {
    let indexes = get_enabled_cells_indexes(rects);
    for index in indexes {
        rects[index as usize].state = CellState::Disabled;
    }
}

fn set_cell_params(
    rects: &mut Vec<Cell>,
    x: isize,
    y: isize,
    state: Option<CellState>,
    marked: Option<bool>,
) {
    let index = pos_to_index((x, y)) as usize;
    let cell = rects[index];
    let rect = rects[index].rect;
    let new_state = match state {
        Some(s) => s,
        None => cell.state,
    };
    let new_marked = match marked {
        Some(m) => m,
        None => cell.marked,
    };

    rects[index] = Cell {
        rect,
        state: new_state,
        marked: new_marked,
    };
}

// INFO: 4 fps
fn get_frame(app: &App) -> isize {
    (app.duration.since_start.as_secs_f64() * 4.0) as isize
}

fn get_prev_pos(app: &App) -> (isize, isize) {
    let frame = get_frame(app);
    if frame < 1 {
        return (0, 0);
    }

    index_to_pos((frame - 1) % (SIZE * SIZE))
}

fn get_next_pos(app: &App) -> (isize, isize) {
    let frame = get_frame(app);
    index_to_pos(frame % (SIZE * SIZE))
}

fn index_to_pos(i: isize) -> (isize, isize) {
    let x = i % SIZE;
    let y = i / SIZE;
    (x, y)
}

fn pos_to_index((x, y): (isize, isize)) -> isize {
    y * SIZE + x
}

fn get_rect_side_and_zone(window_rect: Rect) -> (f32, f32) {
    let zone = window_rect.w().min(window_rect.h()) / SIZE as f32;
    let padding = zone * 0.01;
    let side = zone - padding * 2.0;
    (side, zone)
}
