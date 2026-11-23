//! The focus: one game mode run the whole app is about while it is on. The other tabs then show what is relative
//! to its character: its side of the forge resources, its own finds, the missions it can still do.

use lg_app::App;
use lg_app::characters::Char;
use lg_app::properties::Properties;
use lg_app::upe::Run;

/// The setting holding the focus: "run:<char id>", empty for none.
pub const KEY: &str = "focus";

/// A run that can be the focus: started, not lost, its character alive.
pub fn candidates(app: &App) -> Vec<(&Run, &Char)> {
    let mut out: Vec<(&Run, &Char)> = app
        .upe
        .runs
        .values()
        .filter(|r| r.marked && r.broken.is_none())
        .filter_map(|r| app.characters.get(r.char_id).filter(|c| c.alive).map(|c| (r, c)))
        .collect();
    out.sort_by_key(|(r, _)| r.marked_at); // the oldest run first: the boxes keep their places
    out
}

/// The run picked as the focus, while it can still be one.
pub fn current<'a>(app: &'a App, props: &Properties) -> Option<(&'a Run, &'a Char)> {
    let id: i32 = props.get(KEY)?.strip_prefix("run:")?.parse().ok()?;
    candidates(app).into_iter().find(|(r, _)| r.char_id == id)
}

/// The value saved for a run as the focus (`None`: no focus).
pub fn value(char_id: Option<i32>) -> String {
    char_id.map_or(String::new(), |id| format!("run:{id}"))
}

/// Whether this mission (by "season:mission") is one the focused run can still do without being disqualified: not
/// begun by anyone (still at 0%), or begun by this run alone.
pub fn mission_open(app: &App, run: &Run, key: &str, untouched: &[String]) -> bool {
    untouched.iter().any(|k| k == key) || app.upe.missions.get(key).is_some_and(|m| m.char_id == run.char_id && m.clean && m.from_zero)
}
