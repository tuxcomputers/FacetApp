//! The notice drawn over the Settings window: a title, a message and a row of buttons.
//!
//! One notice at a time. [`Notice::ask`] shows it and keeps the answer callback; the button pressed hides it
//! and hands that callback the button's index.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{NoticeData, SettingsWindow};

type Answer = Box<dyn FnOnce(usize)>;

/// The Settings window's notice.
pub struct Notice {
    ui: slint::Weak<SettingsWindow>,
    answer: RefCell<Option<Answer>>,
}

impl Notice {
    /// Wires the notice's buttons on `ui`. Call once, at launch.
    pub fn attach(ui: &SettingsWindow) -> Rc<Notice> {
        let notice = Rc::new(Notice { ui: ui.as_weak(), answer: RefCell::new(None) });
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
