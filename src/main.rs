use app::{tui, App};
use std::io;
use users::{get_current_uid, get_user_by_uid};

mod app;

pub fn main() -> io::Result<()> {
    let mut terminal = tui::init()?;

    let service = "ttylock".to_string();
    let current_username = get_user_by_uid(get_current_uid())
        .unwrap()
        .name()
        .to_str()
        .unwrap()
        .to_string();

    let mut app = App::new(service, current_username);
    let app_result = app.run(&mut terminal);

    tui::restore()?;

    app_result
}
