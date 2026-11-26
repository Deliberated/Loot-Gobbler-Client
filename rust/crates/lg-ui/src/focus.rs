//! The focus: one game mode run, or one character you added with its eye under Characters, the whole app is about
//! while it is on. The other tabs then show what is relative to its character: its side of the forge resources, its
//! own finds, and for a run the missions it can still do.

use lg_app::App;
use lg_app::characters::Char;
use lg_app::properties::Properties;
use lg_app::upe::Run;

/// The setting holding the focus: "run:<char id>" or "char:<char id>", empty for none.
pub const KEY: &str = "focus";
/// The characters added to the focus row with their eye (Characters tab): char ids, comma-separated.
pub const CHARS_KEY: &str = "focusChars";

/// The characters added with their eye, in the order added.
pub fn chars(props: &Properties) -> Vec<i32> {
    props.get(CHARS_KEY).unwrap_or("").split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

/// The setting for that list.
pub fn chars_value(ids: &[i32]) -> String {
    ids.iter().map(i32::to_string).collect::<Vec<_>>().join(",")
}

/// The characters added with their eye that can be the focus: alive, and not one of the runs already in the row.
pub fn char_candidates<'a>(app: &'a App, props: &Properties) -> Vec<&'a Char> {
    let runs: Vec<i32> = candidates(app).iter().map(|(r, _)| r.char_id).collect();
    chars(props).into_iter().filter(|id| !runs.contains(id)).filter_map(|id| app.characters.get(id).filter(|c| c.alive)).collect()
}

/// The character the focus is on, a run's or one added with its eye, while it can still be the focus.
pub fn focused_char<'a>(app: &'a App, props: &Properties) -> Option<&'a Char> {
    let v = props.get(KEY)?;
    if let Some(id) = v.strip_prefix("char:").and_then(|i| i.parse::<i32>().ok()) {
        // any living character (one with a run too: its eye sets it as a character)
        return app.characters.get(id).filter(|c| c.alive);
    }
    current(app, props).map(|(_, c)| c)
}

/// The value saved when a box in the focus row is picked: its run's, else the character's.
pub fn value_for(app: &App, char_id: Option<i32>) -> String {
    match char_id {
        None => String::new(),
        Some(id) if candidates(app).iter().any(|(r, _)| r.char_id == id) => format!("run:{id}"),
        Some(id) => format!("char:{id}"),
    }
}

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
