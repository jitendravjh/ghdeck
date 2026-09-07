mod app;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ghwork_core::{ago, plural, Filter, Kind};
use std::time::Duration;

const HELP: &str = "\
ghwork, all your github work in one list

usage:
  ghwork              open the dashboard
  ghwork list [what]  print to stdout, what is one of needs-you, open, mine, to-review, all
  ghwork sync         refresh the cache now
  ghwork poll         refresh only if github notifications changed, costs nothing otherwise
  ghwork where        print the cache path
";

fn main() {
    if let Err(e) = run() {
        eprintln!("ghwork: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => dashboard(),
        Some("list") => list(args.get(1).map(String::as_str)),
        Some("sync") => {
            let st = ghwork_core::open()?.refresh(5)?;
            println!("synced {}, {} api pts, {} left", plural(st.fetched as u64, "item"), st.cost, st.remaining);
            if !st.notif_ok {
                eprintln!("note: could not read notifications, so poll will always do a full sync");
            }
            Ok(())
        }
        Some("poll") => {
            let st = ghwork_core::open()?.poll(5)?;
            if st.skipped {
                println!("nothing changed");
            } else {
                println!("synced {}, {} api pts, {} left", plural(st.fetched as u64, "item"), st.cost, st.remaining);
            }
            Ok(())
        }
        Some("where") => {
            println!("{}", ghwork_core::cache::default_path()?.display());
            Ok(())
        }
        Some("-h") | Some("--help") | Some("help") => {
            print!("{HELP}");
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown command {other}\n");
            print!("{HELP}");
            std::process::exit(2);
        }
    }
}

fn parse_filter(name: Option<&str>) -> Filter {
    match name {
        Some("open") => Filter::Open,
        Some("mine") => Filter::Mine,
        Some("to-review") | Some("reviews") => Filter::Reviews,
        Some("all") => Filter::All,
        _ => Filter::Attention,
    }
}

fn ensure_synced(sync: &mut ghwork_core::Sync) -> Result<Vec<ghwork_core::Item>> {
    let items = sync.items()?;
    if items.is_empty() {
        sync.refresh(5)?;
        return sync.items();
    }
    Ok(items)
}

fn list(what: Option<&str>) -> Result<()> {
    let filter = parse_filter(what);
    let mut sync = ghwork_core::open()?;
    let items = ensure_synced(&mut sync)?;
    let me = ghwork_core::viewer()?;
    let rows: Vec<_> = items.iter().filter(|it| filter.keeps(it, &me)).collect();
    println!("{}, {}\n", plural(rows.len() as u64, "item"), filter.label());
    for it in rows {
        let kind = if it.kind == Kind::Pr { "PR" } else { "IS" };
        println!("{:>4}  {kind}  {}  {}", ago(it.updated_at), it.slug(), it.title);
        let chips: Vec<String> = it.chips().into_iter().map(|(t, _)| t).collect();
        let activity = it.activity().unwrap_or_default();
        println!("      {}   {}  {}", chips.join(" · "), plural(it.comments, "comment"), activity);
    }
    Ok(())
}

fn dashboard() -> Result<()> {
    let sync = ghwork_core::open()?;
    let items = sync.items()?;
    let me = ghwork_core::viewer()?;
    let stale = items.is_empty();
    let mut app = App::new(items, me);
    if stale {
        app.refresh(5);
    }

    let mut term = ratatui::init();
    let res = loop_events(&mut term, &mut app);
    ratatui::restore();
    res
}

fn loop_events(
    term: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<()> {
    while !app.quit {
        app.drain()?;
        term.draw(|f| ui::draw(f, app))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }
            handle(app, key);
        }
    }
    Ok(())
}

fn handle(app: &mut App, key: KeyEvent) {
    if app.searching {
        match key.code {
            KeyCode::Esc => {
                app.searching = false;
                app.search.clear();
                app.reindex();
            }
            KeyCode::Enter => app.searching = false,
            KeyCode::Backspace => {
                app.search.pop();
                app.reindex();
            }
            KeyCode::Char(c) => {
                app.search.push(c);
                app.reindex();
            }
            _ => {}
        }
        return;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if ctrl => app.quit = true,
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Esc if !app.search.is_empty() => {
            app.search.clear();
            app.reindex();
        }
        KeyCode::Esc if app.help => app.help = false,
        KeyCode::Char('?') => app.help = !app.help,
        KeyCode::Char('j') | KeyCode::Down => app.move_by(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_by(-1),
        KeyCode::PageDown => app.move_by(10),
        KeyCode::PageUp => app.move_by(-10),
        KeyCode::Char('g') | KeyCode::Home => app.cursor = 0,
        KeyCode::Char('G') | KeyCode::End => app.move_by(isize::MAX / 2),
        KeyCode::Tab => app.cycle_filter(true),
        KeyCode::BackTab => app.cycle_filter(false),
        KeyCode::Char(c @ '1'..='5') => {
            let i = c as usize - '1' as usize;
            app.set_filter(Filter::ORDER[i]);
        }
        KeyCode::Char('o') | KeyCode::Enter => app.open_in_browser(),
        KeyCode::Char('y') => copy_url(app),
        KeyCode::Char('r') => app.refresh(5),
        KeyCode::Char('R') => app.refresh(20),
        KeyCode::Char('/') => {
            app.searching = true;
            app.search.clear();
        }
        _ => {}
    }
}

fn copy_url(app: &mut App) {
    let Some(url) = app.selected().map(|it| it.url.clone()) else { return };
    let ok = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut().unwrap().write_all(url.as_bytes())?;
            c.wait()
        })
        .is_ok();
    app.status = if ok { "url copied".into() } else { "copy failed".into() };
}
