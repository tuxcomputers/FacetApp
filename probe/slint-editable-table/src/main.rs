// The Rust half of the editable-table probe: a model that changes under the UI, and a transcript.
//
// **What is being tested, in the order it matters:**
//
//   1. **Does typing survive a model update to the same row?** The clock ticks once a second on every row,
//      including the one being edited. If the LineEdit loses its text or its focus when `set_row_data`
//      touches that row, then CLAUDE.md's rule -- a value being typed into is not re-read underneath
//      whoever is typing -- is not something this toolkit lets the app keep, and every table in the app is
//      affected.
//   2. **Does it survive an update to a different row?** The weaker version of the same question.
//   3. **Is the commit allowed to be asynchronous?** Return does not change the name on screen in the real
//      app: the name changes when the table has been written and read back. So a commit here waits 800ms
//      before the model moves, and the transcript shows what the row displayed in between.
//   4. **Do Escape and a click elsewhere abandon it**, leaving the stored name alone.
//
// Every event is printed with the seconds since launch, so the transcript is the evidence. Nothing here
// needs watching to be read afterwards.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{Model, ModelRc, SharedString, Timer, TimerMode, VecModel};

slint::include_modules!();

thread_local! {
    static STARTED: Instant = Instant::now();
}

fn log(what: &str) {
    let secs = STARTED.with(|s| s.elapsed().as_secs_f32());
    println!("{secs:>7.2}s  {what}");
}

fn seed() -> Vec<Row> {
    [
        ("Meeting", 0xF0803Cu32),
        ("Code", 0x4A90D9),
        ("Admin", 0x9B59B6),
        ("Break", 0x7F8C8D),
        ("Emails", 0x16A085),
        ("Support", 0xE74C3C),
    ]
    .iter()
    .enumerate()
    .map(|(i, (name, rgb))| Row {
        id: i as i32 + 1,
        name: SharedString::from(*name),
        tint: slint::Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, *rgb as u8),
        clock: SharedString::from("0:00"),
    })
    .collect()
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = TableWindow::new()?;
    let rows = Rc::new(VecModel::from(seed()));
    ui.set_rows(ModelRc::from(rows.clone()));

    // `FACET_PROBE_NO_BUTTON=1` leaves clicking the name as the only way in, which is the case a human
    // has to try: a synthetic click at the TouchArea's own accessibility frame did not open an edit, and
    // whether a real one does is the difference between broken hit-testing and synthetic events not
    // reaching a Slint TouchArea at all.
    let show_button = std::env::var("FACET_PROBE_NO_BUTTON").is_err();
    ui.set_show_edit_button(show_button);
    if show_button {
        log("probe started. Press Edit, or click a name, then type.");
    } else {
        log("probe started WITHOUT the Edit button. Clicking a name is the only way in.");
    }
    log("the clock updates EVERY row once a second, including the row being edited.");

    // **The model changing underneath, which is the whole point.** A real app would be reading the
    // database here; what matters to the toolkit is identical either way: set_row_data on a row whose
    // bindings are live, once a second, forever.
    let ticks = Rc::new(RefCell::new(0u32));
    let tick_timer = Timer::default();
    {
        let rows = rows.clone();
        let ticks = ticks.clone();
        let weak = ui.as_weak();
        tick_timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            let n = { let mut t = ticks.borrow_mut(); *t += 1; *t };
            let editing = weak.upgrade().map(|u| u.get_editing_id()).unwrap_or(-1);
            for i in 0..rows.row_count() {
                if let Some(mut row) = rows.row_data(i) {
                    row.clock = SharedString::from(format!("{}:{:02}", n / 60, n % 60));
                    rows.set_row_data(i, row);
                }
            }
            if editing != -1 {
                log(&format!(
                    "tick {n}: every row updated by set_row_data, INCLUDING row {editing} which is being edited"
                ));
            }
        });
    }

    // Click a name: it becomes a field.
    {
        let weak = ui.as_weak();
        ui.on_start_edit(move |id| {
            if let Some(u) = weak.upgrade() {
                u.set_editing_id(id);
                u.set_status(SharedString::from(format!("editing row {id}")));
                log(&format!("row {id}: edit opened"));
            }
        });
    }

    // Every keystroke, so a lost edit is visible as the text going backwards rather than merely stopping.
    ui.on_typed(|id, text| log(&format!("row {id}: field now holds {text:?}")));

    // Return commits, and the row does NOT change until the write has been read back.
    {
        let weak = ui.as_weak();
        let rows = rows.clone();
        let commit_timer = Rc::new(RefCell::new(Timer::default()));
        ui.on_committed(move |id, text| {
            let Some(u) = weak.upgrade() else { return };
            log(&format!("row {id}: Return pressed with {text:?}; the row still shows the stored name"));
            u.set_editing_id(-1);
            u.set_status(SharedString::from(format!("writing row {id}...")));
            let rows = rows.clone();
            let weak = weak.clone();
            // 800ms standing in for the database write and the read back that follows it.
            commit_timer.borrow().start(TimerMode::SingleShot, Duration::from_millis(800), move || {
                for i in 0..rows.row_count() {
                    if let Some(mut row) = rows.row_data(i) {
                        if row.id == id {
                            row.name = text.clone();
                            rows.set_row_data(i, row);
                            log(&format!("row {id}: the table now holds {text:?}, so the row adopts it"));
                        }
                    }
                }
                if let Some(u) = weak.upgrade() {
                    u.set_status(SharedString::from("idle"));
                }
            });
        });
    }

    // Escape, and a click anywhere else, abandon it.
    {
        let weak = ui.as_weak();
        ui.on_abandoned(move |id| {
            if let Some(u) = weak.upgrade() {
                if u.get_editing_id() == -1 { return; }
                u.set_editing_id(-1);
                u.set_status(SharedString::from("idle"));
                log(&format!("row {id}: edit abandoned, the stored name is unchanged"));
            }
        });
    }

    ui.run()
}
