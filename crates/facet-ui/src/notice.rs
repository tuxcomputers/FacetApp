//! The notice drawn over the Settings window: a title, a message and a row of buttons.
//!
//! One notice at a time. [`Notice::ask`] shows it and keeps the answer callback; the button pressed hides it
//! and hands that callback the button's index.

use std::cell::RefCell;
use std::rc::Rc;

use facet_core::debug_log::{Record, Tag, Trace, plain};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{NoticeData, SettingsWindow};

type Answer = Box<dyn FnOnce(usize)>;

/// The Settings window's notice.
pub struct Notice {
    ui: slint::Weak<SettingsWindow>,
    answer: RefCell<Option<Answer>>,
    /// The choices of the notice showing, so the trace can name the one pressed.
    choices: RefCell<Vec<String>>,
    log: Rc<Trace>,
}

impl Notice {
    /// Wires the notice's buttons on `ui`. Call once, at launch. Each notice shown and each button pressed is written
    /// to `log`.
    pub fn attach(ui: &SettingsWindow, log: Rc<Trace>) -> Rc<Notice> {
        let notice = Rc::new(Notice {
            ui: ui.as_weak(),
            answer: RefCell::new(None),
            choices: RefCell::new(Vec::new()),
            log,
        });
        let weak = Rc::downgrade(&notice);
        ui.global::<NoticeData>().on_chosen(move |index| {
            if let Some(notice) = weak.upgrade() {
                notice.choose(index);
            }
        });
        notice
    }

    /// Shows a notice with one button per entry of `choices`, and calls `answer` with the index of the one
    /// pressed. A notice already up is replaced, and its answer is dropped uncalled.
    pub fn ask(&self, title: &str, message: &str, choices: &[&str], answer: impl FnOnce(usize) + 'static) {
        *self.answer.borrow_mut() = Some(Box::new(answer));
        self.log.record(Tag::Settings, || {
            format!("Notice shown: {}, offering {}", plain(title), plain(&choices.join(", ")))
        });
        *self.choices.borrow_mut() = choices.iter().map(|&choice| choice.to_string()).collect();
        self.show(title, message, choices);
    }

    /// Shows a statement with a single OK button, and nothing to answer.
    pub fn tell(&self, title: &str, message: &str) {
        self.ask(title, message, &["OK"], |_| {});
    }

    /// The title showing, or empty when no notice is up.
    pub fn title(&self) -> String {
        self.ui.upgrade().map(|ui| ui.global::<NoticeData>().get_title().to_string()).unwrap_or_default()
    }

    /// Presses button `index` of the notice showing, as a click on it does.
    pub fn choose(&self, index: i32) {
        let answer = self.answer.borrow_mut().take();
        let pressed = usize::try_from(index).ok().and_then(|index| self.choices.borrow().get(index).cloned());
        self.log.record(Tag::Settings, || match &pressed {
            Some(choice) => format!("Notice answered: {}", plain(choice)),
            None => format!("Notice answered with button {index}, which it does not have"),
        });
        self.choices.borrow_mut().clear();
        self.show("", "", &[]);
        if let (Some(answer), Ok(index)) = (answer, usize::try_from(index)) {
            answer(index);
        }
    }

    fn show(&self, title: &str, message: &str, choices: &[&str]) {
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<NoticeData>();
            let choices: Vec<SharedString> = choices.iter().map(|&choice| choice.into()).collect();
            data.set_choices(ModelRc::new(VecModel::from(choices)));
            data.set_message(message.into());
            data.set_title(title.into());
        }
    }
}
