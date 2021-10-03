use nannou::geom::Rect;
use nannou::prelude::*;
use nannou::rand::random_range;
use nannou::ui::prelude::*;
use nannou_osc as osc;
use nannou_osc::Type;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;

const SIZE: isize = 32;
const HEIGHT: u32 = SIZE as u32 * 2 * 10;
const WIDTH: u32 = SIZE as u32 * 2 * 10;
const CHANNEL: i32 = 0;

static SIMULATIONS: &[Simulation] = &[Simulation::Mover, Simulation::Rain, Simulation::Life];
static NOTE_POLICIES: &[NotePolicy] = &[
    NotePolicy::Min,
    NotePolicy::Max,
    NotePolicy::Avg,
    NotePolicy::Random,
];

fn main() {
    nannou::app(model).update(update).run();
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum NotePolicy {
    Min,
    Max,
    Avg,
    Random,
}

impl fmt::Display for NotePolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Simulation {
    Mover,
    Rain,
    Life,
}

impl fmt::Display for Simulation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
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
    active: bool,
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

        if self.active {
            let pad = rect.h() * 0.2;
            let rect2 = rect
                .pad_left(pad)
                .pad_right(pad)
                .pad_top(pad)
                .pad_bottom(pad);

            draw.rect().xy(rect2.xy()).wh(rect2.wh()).color(RED);
        }
    }
}

widget_ids! {
    struct Ids {
        title_label,
        restart_btn,
        reseed_btn,
        simulation_label,
        simulation_combo,
        note_label,
        note_combo
    }
}

struct Model {
    field: Vec<Cell>,
    initialized: bool,
    sender: osc::Sender<osc::Connected>,
    main_window: WindowId,
    main_window_rect: Rect,
    text: String,
    ids: Ids,
    ui: Ui,
    simulation: Simulation,
    note_policy: NotePolicy,
}

fn model(app: &App) -> Model {
    let main_window = app
        .new_window()
        .title(app.exe_name().unwrap())
        .size(WIDTH, HEIGHT)
        .view(view)
        .build()
        .unwrap();

    let ui_window = app
        .new_window()
        .title(app.exe_name().unwrap() + " controls")
        .size(250, 260)
        .view(ui_view)
        .event(ui_event)
        .build()
        .unwrap();

    let mut ui = app.new_ui().window(ui_window).build().unwrap();
    let ids = Ids::new(ui.widget_id_generator());

    ui.clear_with(color::DARK_CHARCOAL);
    let mut theme = ui.theme_mut();
    theme.label_color = color::WHITE;
    theme.shape_color = color::CHARCOAL;

    let main_window_rect = app.window(main_window).unwrap().rect();

    let mut model = Model {
        field: init_recs(app.window_rect(), None),
        initialized: false,
        sender: osc::sender()
            .unwrap()
            .connect("192.168.0.107:9001")
            .unwrap(),
        main_window: main_window,
        main_window_rect: main_window_rect,
        ids: ids,
        ui: ui,
        text: "".to_string(),
        simulation: Simulation::Life,
        note_policy: NotePolicy::Min,
    };

    ui_event(&app, &mut model, WindowEvent::Focused);

    seed(&mut model.field);

    model
}

fn ui_view(app: &App, model: &Model, frame: Frame) {
    model.ui.draw_to_frame(app, &frame).unwrap();
}

fn ui_event(_app: &App, model: &mut Model, _event: WindowEvent) {
    let ui = &mut model.ui.set_widgets();

    // Control panel title
    widget::Text::new("Controls")
        .top_left_with_margin(15.0)
        .w_h(100.0, 24.0)
        .font_size(16)
        .set(model.ids.title_label, ui);

    // for event in widget::TextEdit::new(&model.text)
    //     .top_left_with_margin(10.0)
    //     .w_h(300.0, 40.0)
    //     .font_size(24)
    //     .set(model.ids.title, ui)
    // {
    //     model.text = event.clone();
    // }

    // Restart
    for _click in widget::Button::new()
        .down_from(model.ids.title_label, 12.0)
        .w_h(100.0, 28.0)
        .label("Restart")
        .label_font_size(16)
        .set(model.ids.restart_btn, ui)
    {
        model.initialized = false;
    }

    for _click in widget::Button::new()
        .right_from(model.ids.restart_btn, 12.0)
        .w_h(100.0, 28.0)
        .label("Reseed")
        .label_font_size(16)
        .set(model.ids.reseed_btn, ui)
    {
        seed(&mut model.field);
    }

    widget::Text::new("Simulation")
        .down_from(model.ids.restart_btn, 12.0)
        .w_h(100.0, 24.0)
        .font_size(16)
        .set(model.ids.simulation_label, ui);

    let current_sim = &model.simulation;

    for event in widget::DropDownList::new(
        SIMULATIONS
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .as_slice(),
        SIMULATIONS
            .iter()
            .enumerate()
            .find(|&(_, e)| *e == *current_sim)
            .map(|(i, _)| i),
    )
    .right_from(model.ids.simulation_label, 12.0)
    .w_h(100.0, 28.0)
    .label_font_size(16)
    .set(model.ids.simulation_combo, ui)
    {
        model.simulation = SIMULATIONS[event];
    }

    widget::Text::new("Note policy")
        .down_from(model.ids.simulation_label, 12.0)
        .w_h(100.0, 24.0)
        .font_size(16)
        .set(model.ids.note_label, ui);

    let current_note_policy = &model.note_policy;

    for event in widget::DropDownList::new(
        NOTE_POLICIES
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .as_slice(),
        NOTE_POLICIES
            .iter()
            .enumerate()
            .find(|&(_, e)| *e == *current_note_policy)
            .map(|(i, _)| i),
    )
    .right_from(model.ids.note_label, 12.0)
    .w_h(100.0, 28.0)
    .label_font_size(16)
    .set(model.ids.note_combo, ui)
    {
        model.note_policy = NOTE_POLICIES[event];
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let window_rect = app.window(model.main_window).unwrap().rect();
    if !window_rect.eq(&model.main_window_rect) {
        model.main_window_rect = window_rect;
        model.field = init_recs(window_rect, Some(&model.field));
    }

    match model.simulation {
        Simulation::Rain => rain(app, model),
        Simulation::Mover => mover(app, model),
        Simulation::Life => life(app, model),
    }

    // emit osc events
    let frac = app.elapsed_frames() % 10;
    match frac {
        0 => emit(model),
        9 => stop(model),
        _ => {}
    }
    //
}

// INFO: note generating policy
fn _note_by_cell_index(index: usize) -> i32 {
    let (x, y) = index_to_pos(index as isize);

    // simple emitter
    // let note = x.checked_div(y).or(Some(0)).unwrap()
    //     + x.checked_rem(y).or(Some(0)).unwrap()
    //     + 64; // compensate? ;)

    let note = (x + y).checked_rem(128).or(Some(0)).unwrap();
    println!("note({}, {}) = {}", x, y, note);
    note as i32
}

fn _note_with_max_index(indices: &[usize], model: &mut Model) {
    if let Some(index) = indices.iter().max() {
        let mut cell = model.field.get_mut(*index as usize).unwrap();

        if cell.active {
            return;
        }

        (*cell).active = true;

        let note = _note_by_cell_index(*index) as i32;
        let args = vec![Type::Int(CHANNEL), Type::Int(note as i32), Type::Float(1.0)];
        model.sender.send(("/midi/noteOn", args)).ok();
    }
}

fn _note_with_min_index(indices: &[usize], model: &mut Model) {
    if let Some(index) = indices.iter().min() {
        let mut cell = model.field.get_mut(*index as usize).unwrap();

        if cell.active {
            return;
        }

        (*cell).active = true;

        let note = _note_by_cell_index(*index) as i32;
        let args = vec![Type::Int(CHANNEL), Type::Int(note as i32), Type::Float(1.0)];
        model.sender.send(("/midi/noteOn", args)).ok();
    }
}

fn _note_with_avg_index(indices: &[usize], model: &mut Model) {
    if indices.is_empty() {
        return;
    }

    let index: usize = indices.iter().sum::<usize>() / indices.len();
    println!("index = {}", index);

    let mut cell = model.field.get_mut(index).unwrap();

    if cell.active {
        return;
    }

    (*cell).active = true;

    let note = _note_by_cell_index(index) as i32;
    let args = vec![Type::Int(CHANNEL), Type::Int(note as i32), Type::Float(1.0)];
    model.sender.send(("/midi/noteOn", args)).ok();
}

fn _note_with_random_index(indices: &[usize], model: &mut Model) {
    let mut rng = thread_rng();
    if let Some(index) = indices.choose(&mut rng) {
        let mut cell = model.field.get_mut(*index as usize).unwrap();

        if cell.active {
            return;
        }

        (*cell).active = true;

        let note = _note_by_cell_index(*index) as i32;
        let args = vec![Type::Int(CHANNEL), Type::Int(note as i32), Type::Float(1.0)];
        model.sender.send(("/midi/noteOn", args)).ok();
    }
}

fn emit(model: &mut Model) {
    let collisions = get_collisions(&model.field);
    let mut indices = vec![];
    for e in collisions.iter() {
        if let Some((i, _)) = *e {
            indices.push(i)
        }
    }

    match model.note_policy {
        NotePolicy::Min => _note_with_min_index(&indices, model),
        NotePolicy::Max => _note_with_max_index(&indices, model),
        NotePolicy::Avg => _note_with_avg_index(&indices, model),
        NotePolicy::Random => _note_with_random_index(&indices, model),
    };
}

fn stop(model: &mut Model) {
    for (i, c) in model.field.iter_mut().enumerate() {
        if c.active {
            c.active = false;
            let note = _note_by_cell_index(i);
            let args = vec![Type::Int(CHANNEL), Type::Int(note), Type::Float(1.0)];
            model.sender.send(("/midi/noteOff", args)).ok();
        }
    }
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
        set_cell_params(&mut model.field, 0, 0, Some(CellState::Enabled), None, None);
        model.initialized = true;
    }

    let (px, py) = get_prev_pos(app);
    let (x, y) = get_next_pos(app);
    if px == x && py == y {
        return;
    }
    set_cell_params(
        &mut model.field,
        px,
        py,
        Some(CellState::Disabled),
        None,
        None,
    );
    set_cell_params(&mut model.field, x, y, Some(CellState::Enabled), None, None);
}

fn rain(_app: &App, model: &mut Model) {
    if !model.initialized {
        clear_field(&mut model.field);
        model.initialized = true;
    }

    let enabled_indexes = get_enabled_cells_indexes(&model.field);

    clear_field(&mut model.field);

    // add new drop
    if enabled_indexes.len() < (SIZE * 2) as usize {
        let x = random_range(0, SIZE);
        set_cell_params(&mut model.field, x, 0, Some(CellState::Enabled), None, None);
    }

    // fall old drops
    for index in enabled_indexes {
        let (x, y) = index_to_pos(index);
        if y + 1 < SIZE {
            set_cell_params(
                &mut model.field,
                x,
                y + 1,
                Some(CellState::Enabled),
                None,
                None,
            )
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
            set_cell_params(&mut model.field, x, y, Some(CellState::Enabled), None, None);
        }
        model.initialized = true;
    }

    let mut next_field = init_recs(model.main_window_rect, Some(&model.field));

    for x in 0..SIZE {
        for y in 0..SIZE {
            let cell = get_cell(&model.field, x, y);

            let is_alive = _is_alive(&cell);
            let neigbours_cells = get_neighbours_cells(&model.field, x, y);
            let alive_neighbours = neigbours_cells.iter().filter(|&c| _is_alive(c)).count();

            if is_alive {
                match alive_neighbours {
                    1 => set_cell_params(
                        &mut next_field,
                        x,
                        y,
                        Some(CellState::Disabled),
                        None,
                        None,
                    ),
                    2 | 3 => {
                        set_cell_params(&mut next_field, x, y, Some(CellState::Enabled), None, None)
                    }
                    _ => set_cell_params(
                        &mut next_field,
                        x,
                        y,
                        Some(CellState::Disabled),
                        None,
                        None,
                    ),
                }
            } else if alive_neighbours == 3 {
                set_cell_params(&mut next_field, x, y, Some(CellState::Enabled), None, None)
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

        if let Some(cell) = old_field
            .and_then(|o| get_cell(o, x, y))
            .map(|c| Cell { rect, ..c })
        {
            field.push(cell);
        } else {
            field.push(Cell {
                rect,
                state: CellState::Disabled,
                marked: false,
                active: false,
            });
        };
    }

    field
}

fn seed(rects: &mut Vec<Cell>) {
    let marked_count = SIZE * SIZE / 8;

    rects.iter_mut().for_each(|c| c.marked = false);

    for _ in 0..marked_count {
        let x = random_range(0, SIZE);
        let y = random_range(0, SIZE);
        rects[pos_to_index((x, y)) as usize].marked = true;
    }
}

fn get_collisions(rects: &[Cell]) -> Vec<Option<(usize, Cell)>> {
    rects
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if matches!(c.state, CellState::Enabled) && c.marked {
                Some((i, c.clone()))
            } else {
                None
            }
        })
        .collect()
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
    active: Option<bool>,
) {
    for (x, y) in positions.iter() {
        set_cell_params(rects, *x, *y, state, marked, active)
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
    active: Option<bool>,
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

    let new_active = match active {
        Some(a) => a,
        None => cell.active,
    };

    rects[index] = Cell {
        rect,
        state: new_state,
        marked: new_marked,
        active: new_active,
    };
}

// TODO: use app.elapsed_frames
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
