mod app;
mod ui;
mod worker;

use anyhow::{bail, Context, Result};
use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ghdeck_core::{ago, plural, Filter, Kind};
use worker::Worker;
use std::io::{IsTerminal, Write};
use std::time::Duration;

const HELP: &str = "\
ghdeck, all your github work in one list

usage:
  ghdeck              open the dashboard
  ghdeck list [what]  print to stdout, what is one of attention, open, yours, to-review, all
  ghdeck sync         refresh the cache now
  ghdeck poll         refresh only if github notifications changed, costs nothing otherwise
  ghdeck show <ref>   print the conversation, ref is owner/repo#123
  ghdeck user <name>  print someone's prs and issues, newest first
                      add authored, mentioned or open to narrow it
  ghdeck where        print the cache path

  --version, -V       print the version
";

fn main() {
    if let Err(e) = run() {
        let closed = e.chain().any(|c| {
            c.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
        });
        if closed {
            return;
        }
        eprintln!("ghdeck: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => dashboard(),
        Some("list") => list(args.get(1).map(String::as_str)),
        Some("sync") => {
            let st = ghdeck_core::open()?.refresh(5)?;
            println!("synced {}, {} api pts, {} left", plural(st.fetched as u64, "item"), st.cost, st.remaining);
            if !st.notif_ok {
                eprintln!("note: could not read notifications, so poll will always do a full sync");
            }
            Ok(())
        }
        Some("poll") => {
            let st = ghdeck_core::open()?.poll(5)?;
            if st.skipped {
                println!("nothing changed");
            } else {
                println!("synced {}, {} api pts, {} left", plural(st.fetched as u64, "item"), st.cost, st.remaining);
            }
            Ok(())
        }
        Some("show") => show(args.get(1).map(String::as_str)),
        Some("user") => user(args.get(1).map(String::as_str), args.get(2).map(String::as_str)),
        Some("where") => {
            println!("{}", ghdeck_core::cache::default_path()?.display());
            Ok(())
        }
        Some("-V") | Some("--version") | Some("version") => {
            println!("ghdeck {}", env!("CARGO_PKG_VERSION"));
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
        Some("yours") | Some("mine") => Filter::Mine,
        Some("to-review") | Some("reviews") => Filter::Reviews,
        Some("all") => Filter::All,
        _ => Filter::Attention,
    }
}

fn ensure_synced(sync: &mut ghdeck_core::Sync) -> Result<Vec<ghdeck_core::Item>> {
    let items = sync.items()?;
    if items.is_empty() {
        sync.refresh(5)?;
        return sync.items();
    }
    Ok(items)
}

fn list(what: Option<&str>) -> Result<()> {
    let filter = parse_filter(what);
    let mut sync = ghdeck_core::open()?;
    let items = ensure_synced(&mut sync)?;
    let me = sync.login()?;
    let rows: Vec<_> = items.iter().filter(|it| filter.keeps(it, &me)).collect();
    let mut out = std::io::stdout().lock();
    writeln!(out, "{}, {}\n", plural(rows.len() as u64, "item"), filter.label().to_lowercase())?;
    print_rows(&mut out, &rows)?;
    Ok(())
}

fn user(who: Option<&str>, what: Option<&str>) -> Result<()> {
    let who = who.context("give a username: ghdeck user [username]")?;
    let login = ghdeck_core::clean_login(who).with_context(|| format!("{who} is not a github username"))?;
    let filter = match what {
        None | Some("all") => Filter::All,
        Some("authored") => Filter::Authored,
        Some("mentioned") => Filter::Mentioned,
        Some("open") => Filter::Open,
        Some(other) => bail!("{other} is not a filter here, use all, authored, mentioned or open"),
    };
    let tty = std::io::stderr().is_terminal();
    let mut progress = |items: &[ghdeck_core::Item], _: u64| {
        if tty {
            eprint!("\rloading {login}, {} so far", items.len());
        }
        true
    };
    let got = ghdeck_core::open()?.activity(&login, 5, &mut progress);
    if tty {
        eprint!("\r\x1b[K");
    }
    let got = got?;

    let mut out = std::io::stdout().lock();
    if got.items.is_empty() {
        writeln!(out, "nothing for {login} that you can see")?;
        return Ok(());
    }
    let rows: Vec<_> = got.items.iter().filter(|it| filter.keeps(it, &login)).collect();
    writeln!(out, "{}, {} for {login}", plural(rows.len() as u64, "item"), filter.label().to_lowercase())?;
    if let Some(since) = got.feed_since {
        writeln!(out, "github hides {login} from search, so this is their public activity since {}", since.format("%-d %b"))?;
        if filter == Filter::Mentioned {
            writeln!(out, "mentions are not in that activity, so this filter stays empty")?;
        }
    } else if got.involved_total > got.items.len() as u64 {
        writeln!(out, "from the newest {} of {} they are involved in", got.items.len(), got.involved_total)?;
    }
    writeln!(out)?;
    print_rows(&mut out, &rows)?;
    Ok(())
}

fn print_rows(out: &mut impl Write, rows: &[&ghdeck_core::Item]) -> std::io::Result<()> {
    for it in rows {
        let kind = if it.kind == Kind::Pr { "[PR]" } else { "[ISSUE]" };
        writeln!(out, "{:>4}  {kind:<7} {}  {}", ago(it.updated_at), it.slug(), it.title)?;
        let chips: Vec<String> = it.chips().into_iter().map(|(t, _)| t).collect();
        let mut line = format!("{:<14}{}", "", chips.join(" \u{b7} "));
        if it.comments > 0 {
            line.push_str(&format!("   {}", plural(it.comments, "comment")));
        }
        if let Some(a) = it.activity() {
            line.push_str(&format!("   {a}"));
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn show(target: Option<&str>) -> Result<()> {
    let target = target.context("give a ref like owner/repo#123")?;
    let (repo, number) = target.rsplit_once('#').context("expected owner/repo#123")?;
    let number: u64 = number.parse().context("that number did not parse")?;
    let events = ghdeck_core::open()?.thread(repo, number)?;
    let mut out = std::io::stdout().lock();
    if events.is_empty() {
        writeln!(out, "nothing on {repo}#{number}")?;
        return Ok(());
    }
    for e in events {
        writeln!(out, "{} {} {}", e.at.format("%Y-%m-%d %H:%M"), e.who, e.label)?;
        if e.bot {
            let gist = e.gist();
            if !gist.is_empty() {
                writeln!(out, "    {gist}")?;
            }
        } else {
            for line in e.body.lines() {
                writeln!(out, "    {line}")?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}

fn dashboard() -> Result<()> {
    let boot = Worker::boot()?;
    let cold = boot.items.is_empty();
    let mut app = App::new(boot);
    if cold {
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
        match event::read()? {
            Event::Key(key) if key.kind != event::KeyEventKind::Release => {
                let was_detail = app.detail;
                handle(app, key);
                if was_detail || app.detail {
                    term.clear()?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle(app: &mut App, key: KeyEvent) {
    if app.asking {
        match key.code {
            KeyCode::Esc => {
                app.asking = false;
                app.ask.clear();
            }
            KeyCode::Enter => {
                app.asking = false;
                let who = std::mem::take(&mut app.ask);
                if !who.trim().is_empty() {
                    app.look_up(&who);
                }
            }
            KeyCode::Backspace => {
                app.ask.pop();
            }
            KeyCode::Char(c) if !c.is_whitespace() && app.ask.len() < 40 => app.ask.push(c),
            _ => {}
        }
        return;
    }

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
    if app.detail {
        match key.code {
            KeyCode::Char('c') if ctrl => app.quit = true,
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Left => app.close_detail(),
            KeyCode::Down => app.scroll_by(1),
            KeyCode::Up => app.scroll_by(-1),
            KeyCode::PageDown | KeyCode::Char(' ') => app.scroll_by(15),
            KeyCode::PageUp => app.scroll_by(-15),
            KeyCode::Char('g') | KeyCode::Home => app.scroll = 0,
            KeyCode::Char('b') => app.show_bots = !app.show_bots,
            KeyCode::Char('r') => app.reload_thread(),
            KeyCode::Char('o') => app.open_in_browser(),
            KeyCode::Char('y') => copy_url(app),
            _ => {}
        }
        return;
    }
    match key.code {
        KeyCode::Char('c') if ctrl => app.quit = true,
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Esc if !app.search.is_empty() => {
            app.search.clear();
            app.reindex();
        }
        KeyCode::Esc if app.help => app.help = false,
        KeyCode::Esc if app.who.is_some() => app.back_to_mine(),
        KeyCode::Char('?') => app.help = !app.help,
        KeyCode::Down => app.move_by(1),
        KeyCode::Up => app.move_by(-1),
        KeyCode::PageDown => app.move_by(10),
        KeyCode::PageUp => app.move_by(-10),
        KeyCode::Char('g') | KeyCode::Home => app.cursor = 0,
        KeyCode::Char('G') | KeyCode::End => app.jump_to_end(),
        KeyCode::Right | KeyCode::Tab => app.cycle_filter(true),
        KeyCode::Left | KeyCode::BackTab => app.cycle_filter(false),
        KeyCode::Char(c @ '1'..='5') => app.pick_filter(c as usize - '1' as usize),
        KeyCode::Enter => app.open_detail(),
        KeyCode::Char('o') => app.open_in_browser(),
        KeyCode::Char('y') => copy_url(app),
        KeyCode::Char('r') => app.refresh(5),
        KeyCode::Char('R') => app.refresh(20),
        KeyCode::Char('/') => {
            app.searching = true;
            app.search.clear();
        }
        KeyCode::Char('u') => {
            app.asking = true;
            app.ask.clear();
        }
        _ => {}
    }
}

fn copy_url(app: &mut App) {
    let Some(url) = app.selected().map(|it| it.url.clone()) else { return };
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(windows) {
        &[("clip", &[])]
    } else {
        &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"]), ("xsel", &["--clipboard", "--input"])]
    };
    let copied = candidates.iter().any(|(bin, args)| pipe_to(bin, args, &url));
    app.status = if copied { "url copied".into() } else { "no clipboard tool found".into() };
}

fn pipe_to(bin: &str, args: &[&str], text: &str) -> bool {
    use std::io::Write;
    let Ok(mut child) = std::process::Command::new(bin)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    let wrote = child
        .stdin
        .take()
        .map(|mut pipe| pipe.write_all(text.as_bytes()).is_ok())
        .unwrap_or(false);
    matches!(child.wait(), Ok(st) if st.success()) && wrote
}
