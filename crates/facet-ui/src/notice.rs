//! The notice drawn over the Settings window: a title, a message and a row of buttons.
//!
//! One notice at a time. [`Notice::ask`] shows it and keeps the answer callback; the button pressed hides it
//! and hands that callback the button's index.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use facet_core::debug_log::{Record, Tag, Trace, plain};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{NoticeData, SettingsWindow};

type Answer = Box<dyn FnOnce(usize)>;

/// The Settings window's notice.
pub struct Notice {
    ui: slint::Weak<SettingsWindow>,
    answer: RefCell<Option<Answer>>,
    /// The choice Return and Escape take: the answer that changes nothing. `None` where every answer does something.
    way_out: Cell<Option<usize>>,
    /// Shows the Settings window, for a notice raised while it is hidden. Set by the composition root, which knows how.
    open_window: RefCell<Option<Box<dyn Fn()>>>,
    /// Whether the window was opened for the notice showing, and so is closed again once it is answered.
    opened_for_notice: Cell<bool>,
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
            way_out: Cell::new(None),
            open_window: RefCell::new(None),
            opened_for_notice: Cell::new(false),
            choices: RefCell::new(Vec::new()),
            log,
        });
        let weak = Rc::downgrade(&notice);
        ui.global::<NoticeData>().on_chosen(move |index| {
            if let Some(notice) = weak.upgrade() {
                notice.choose(index);
            }
        });
        let weak = Rc::downgrade(&notice);
        ui.global::<NoticeData>().on_way_out_pressed(move || {
            if let Some(notice) = weak.upgrade() {
                notice.press_way_out();
            }
        });
        notice
    }

    /// Tells the notice how to show the Settings window, which it does for a notice raised while the window is hidden,
    /// and closes through the window's own close control once the notice is answered.
    pub fn set_window_opener(&self, open_window: impl Fn() + 'static) {
        *self.open_window.borrow_mut() = Some(Box::new(open_window));
    }

    /// Takes the way out of the notice showing, as Return or Escape does. Nothing when it has none.
    pub fn press_way_out(&self) {
        if let Some(index) = self.way_out.get() {
            self.choose(i32::try_from(index).unwrap_or(i32::MAX));
        }
    }

    /// Shows a notice with one button per entry of `choices`, and calls `answer` with the index of the one
    /// pressed. A notice already up is replaced, and its answer is dropped uncalled. Return and Escape do nothing: use
    /// [`Notice::ask_with_way_out`] where one of the answers changes nothing.
    pub fn ask(&self, title: &str, message: &str, choices: &[&str], answer: impl FnOnce(usize) + 'static) {
        self.ask_with_way_out(title, message, choices, None, answer);
    }

    /// As [`Notice::ask`], with Return and Escape taking `choices[way_out]`: the answer that changes nothing. A way
    /// out is named, never inferred from a button's label, so that a stray Return cannot agree to something.
    pub fn ask_with_way_out(
        &self,
        title: &str,
        message: &str,
        choices: &[&str],
        way_out: Option<usize>,
        answer: impl FnOnce(usize) + 'static,
    ) {
        debug_assert!(
            way_out.is_none_or(|index| index < choices.len()),
            "the way out is not one of the choices"
        );
        *self.answer.borrow_mut() = Some(Box::new(answer));
        self.way_out.set(way_out);
        self.log.record(Tag::Settings, || {
            format!("Notice shown: {}, offering {}", plain(title), plain(&choices.join(", ")))
        });
        *self.choices.borrow_mut() = choices.iter().map(|&choice| choice.to_string()).collect();
        self.show(title, message, choices);
        self.open_the_window_if_hidden();
    }

    /// Shows a statement with a single OK button, which Return and Escape press, and nothing to answer.
    pub fn tell(&self, title: &str, message: &str) {
        self.ask_with_way_out(title, message, &["OK"], Some(0), |_| {});
    }

    /// A notice needs the window, so one raised while it is hidden shows it. Said in the trace.
    fn open_the_window_if_hidden(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        if ui.window().is_visible() {
            return;
        }
        let open_window = self.open_window.borrow();
        let Some(open_window) = open_window.as_ref() else { return };
        self.log.record(Tag::Settings, || {
            "The Settings window is hidden, so it is shown for the notice".to_string()
        });
        self.opened_for_notice.set(true);
        open_window();
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
        self.way_out.set(None);
        self.show("", "", &[]);
        if let Some(ui) = self.ui.upgrade() {
            ui.invoke_focus_keys();
            if self.opened_for_notice.replace(false) {
                ui.invoke_close_pressed();
            }
        }
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use slint::platform::{Key, Platform, WindowAdapter, WindowEvent};

    use super::*;

    struct Headless(Rc<MinimalSoftwareWindow>);
    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    fn window() -> (SettingsWindow, Rc<Notice>) {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let ui = SettingsWindow::new().expect("the window should build");
        let notice = Notice::attach(&ui, Rc::new(Trace::none()));
        (ui, notice)
    }

    fn press(ui: &SettingsWindow, key: Key) {
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: key.into() });
    }

    #[test]
    fn the_way_out_answers_with_the_choice_that_changes_nothing() {
        let (_ui, notice) = window();
        let answered = Rc::new(Cell::new(None));
        let heard = Rc::clone(&answered);
        notice.ask_with_way_out("Reset?", "", &["Cancel", "Reset"], Some(0), move |index| {
            heard.set(Some(index))
        });
        notice.press_way_out();
        assert_eq!(answered.get(), Some(0));
        assert_eq!(notice.title(), "", "answering takes the notice down");
    }

    #[test]
    fn a_question_with_no_way_out_is_left_up() {
        let (_ui, notice) = window();
        let answered = Rc::new(Cell::new(None));
        let heard = Rc::clone(&answered);
        notice.ask("Which?", "", &["One", "Two"], move |index| heard.set(Some(index)));
        notice.press_way_out();
        assert_eq!(answered.get(), None);
        assert_eq!(notice.title(), "Which?");
    }

    #[test]
    fn a_statement_has_its_ok_as_the_way_out() {
        let (_ui, notice) = window();
        notice.tell("Saved", "");
        notice.press_way_out();
        assert_eq!(notice.title(), "");
    }

    #[test]
    fn escape_and_return_on_the_window_take_the_way_out_and_never_close_it() {
        let (ui, notice) = window();
        let closed = Rc::new(Cell::new(0));
        let counted = Rc::clone(&closed);
        ui.on_close_pressed(move || counted.set(counted.get() + 1));
        ui.show().expect("the window should show");

        for key in [Key::Escape, Key::Return] {
            let answered = Rc::new(Cell::new(None));
            let heard = Rc::clone(&answered);
            notice.ask_with_way_out("Reset?", "", &["Cancel", "Reset"], Some(0), move |index| {
                heard.set(Some(index))
            });
            press(&ui, key);
            assert_eq!(answered.get(), Some(0), "{key:?} should take the way out");
            assert_eq!(notice.title(), "");
        }
        assert_eq!(closed.get(), 0, "a key meant for the notice must not close the window");

        notice.ask("Which?", "", &["One", "Two"], |_| {});
        press(&ui, Key::Escape);
        assert_eq!(notice.title(), "Which?", "with no way out the notice stays");
        assert_eq!(closed.get(), 0, "and Escape still does not close the window from under it");
        notice.choose(0);

        press(&ui, Key::Escape);
        assert_eq!(closed.get(), 1, "with no notice up, Escape closes the window as before");
    }

    #[test]
    fn a_notice_raised_while_the_window_is_hidden_shows_it_and_closes_it_again() {
        let (ui, notice) = window();
        let weak = ui.as_weak();
        notice.set_window_opener(move || {
            if let Some(ui) = weak.upgrade() {
                ui.show().expect("the window should show");
            }
        });
        let weak = ui.as_weak();
        ui.on_close_pressed(move || {
            if let Some(ui) = weak.upgrade() {
                ui.hide().expect("the window should hide");
            }
        });
        assert!(!ui.window().is_visible());

        notice.tell("The setting was not saved", "");
        assert!(ui.window().is_visible(), "the notice needs a window to be seen in");
        notice.choose(0);
        assert!(!ui.window().is_visible(), "the window opened for the notice goes again");

        ui.show().expect("the window should show");
        notice.tell("Another", "");
        notice.choose(0);
        assert!(ui.window().is_visible(), "a window the person had open stays open");
    }
}
