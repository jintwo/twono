use nannou::geom::Rect;
use nannou::prelude::*;
use nannou::rand::random_range;

const SIZE: isize = 32;

fn main() {
    nannou::app(model).update(update).simple_window(view).run();
}

enum CellState {
    Enabled,
    Disabled,
    Static,
}

struct Cell {
    rect: Rect,
    enabled: bool,
    // state: CellState,
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

    // mover(app, model);
    // rain(app, model);
    life(app, model);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.background().color(STEELBLUE);

    model.field.iter().for_each(|cell| {
        draw.rect()
            .xy(cell.rect.xy())
            .wh(cell.rect.wh())
            .color(if cell.enabled { WHITE } else { BLACK });
    });

    draw.to_frame(app, &frame).unwrap();
}

// simulations
fn mover(app: &App, model: &mut Model) {
    if !model.initialized {
        set_cell_state(&mut model.field, 0, 0, true);
        model.initialized = true;
    }

    let (px, py) = get_prev_pos(app);
    let (x, y) = get_next_pos(app);
    if px == x && py == y {
        return;
    }
    set_cell_state(&mut model.field, px, py, false);
    set_cell_state(&mut model.field, x, y, true)
}

fn rain(_app: &App, model: &mut Model) {
    let selected = get_enabled_cells_indexes(&model.field);

    clear_field(&mut model.field);

    // add new drop
    if selected.len() < (SIZE * 2) as usize {
        let x = random_range(0, SIZE);
        set_cell_state(&mut model.field, x, 0, true);
    }

    // fall old drops
    for index in selected {
        let (x, y) = index_to_pos(index);
        if y + 1 < SIZE {
            set_cell_state(&mut model.field, x, y + 1, true)
        }
    }
}

fn life(app: &App, model: &mut Model) {
    // init
    if !model.initialized {
        for _ in 0..SIZE * SIZE / 2 {
            let x = random_range(0, SIZE);
            let y = random_range(0, SIZE);
            set_cell_state(&mut model.field, x, y, true)
        }
        model.initialized = true;
    }

    let mut next_field = init_recs(model.window, None);
    for x in 0..SIZE {
        for y in 0..SIZE {
            let is_alive = get_cell_state(&model.field, x, y).unwrap();
            let neigbours = get_cell_neighbours_states(&model.field, x, y);
            let alive_neighbours = neigbours
                .iter()
                .filter(|&v| v.is_some() && v.unwrap())
                .count();

            if is_alive {
                match alive_neighbours {
                    1 => set_cell_state(&mut next_field, x, y, false),
                    2 | 3 => set_cell_state(&mut next_field, x, y, true),
                    _ => set_cell_state(&mut next_field, x, y, false),
                }
            } else if alive_neighbours == 3 {
                set_cell_state(&mut next_field, x, y, true)
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

        let enabled = if let Some(f) = old_field {
            f[i as usize].enabled
        } else {
            false
        };

        let state = CellState::Enabled;

        field.push(Cell {
            rect,
            enabled,
            // state,
        });
    }

    field
}

fn get_enabled_cells_indexes(rects: &[Cell]) -> Vec<isize> {
    rects
        .iter()
        .enumerate()
        .filter(|&(_, v)| v.enabled)
        .map(|(i, _)| i as isize)
        .collect()
}

fn get_cells_by_state(rects: &[Cell], state: bool) -> Vec<(isize, isize)> {
    rects
        .iter()
        .enumerate()
        .filter(|&(_, v)| v.enabled == state)
        .map(|(i, _)| index_to_pos(i as isize))
        .collect()
}

fn set_cells_state(rects: &mut Vec<Cell>, positions: Vec<(isize, isize)>, state: bool) {
    for (x, y) in positions.iter() {
        set_cell_state(rects, *x, *y, state)
    }
}

fn get_cell_neighbours_states(rects: &[Cell], x: isize, y: isize) -> Vec<Option<bool>> {
    let mut result: Vec<Option<bool>> = vec![];

    result.push(get_cell_state(rects, x - 1, y - 1)); // top left
    result.push(get_cell_state(rects, x, y - 1)); // top
    result.push(get_cell_state(rects, x + 1, y - 1)); // top right

    result.push(get_cell_state(rects, x - 1, y)); // left
    result.push(get_cell_state(rects, x + 1, y)); // right

    result.push(get_cell_state(rects, x - 1, y + 1)); // bottom left
    result.push(get_cell_state(rects, x, y + 1)); // bottom
    result.push(get_cell_state(rects, x + 1, y + 1)); // bottom right

    result
}

fn get_cell_state(rects: &[Cell], x: isize, y: isize) -> Option<bool> {
    if x < 0 || y < 0 {
        return None;
    }

    if x >= SIZE || y >= SIZE {
        return None;
    }

    let index = pos_to_index((x, y)) as isize;
    Some(rects[index as usize].enabled)
}

fn clear_field(rects: &mut Vec<Cell>) {
    let indexes = get_enabled_cells_indexes(rects);
    for index in indexes {
        rects[index as usize].enabled = false
    }
}

fn set_cell_state(rects: &mut Vec<Cell>, x: isize, y: isize, state: bool) {
    let index = pos_to_index((x, y)) as usize;
    rects[index] = Cell {
        rect: rects[index].rect,
        enabled: state,
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
