use crate::app::App;
use ghdeck_core::{ago, clip, plural, Item, Kind, Tone};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

fn tone(t: Tone) -> Style {
    let c = match t {
        Tone::Good => Color::Green,
        Tone::Bad => Color::Red,
        Tone::Warn => Color::Yellow,
        Tone::Info => Color::Magenta,
        Tone::Muted => Color::DarkGray,
    };
    Style::default().fg(c)
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(6),
    ])
    .areas(f.area());

    header(f, head, app);
    if app.help {
        help(f, body);
    } else if app.stats_open {
        stats(f, body, app);
    } else if app.detail {
        detail(f, body, app);
    } else {
        list(f, body, app);
    }
    footer(f, foot, app);
}

fn header(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(" ghdeck ", Style::default().bold().fg(Color::Cyan))];
    if let Some(who) = &app.who {
        spans.push(Span::styled(format!(" @{who} "), Style::default().bold().fg(Color::Yellow)));
    }
    for filt in app.filters() {
        let n = app.count(*filt);
        let active = *filt == app.filter;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {} [{}] ", filt.label(), n), style));
    }
    if app.who_loading {
        spans.push(Span::styled("  loading", Style::default().fg(Color::Yellow)));
    } else if app.busy() {
        spans.push(Span::styled("  syncing", Style::default().fg(Color::Yellow)));
    } else if let Some(since) = app.who_since.filter(|_| app.who.is_some()) {
        spans.push(Span::styled(
            format!("  hidden from search, showing their activity since {}", since.format("%-d %b")),
            Style::default().fg(Color::Yellow),
        ));
    } else if app.who.is_some() && app.who_total > app.items.len() as u64 {
        spans.push(Span::styled(
            format!("  newest {} of {}", app.items.len(), app.who_total),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let lens = &app.lens;
    if lens.shape != ghdeck_core::Shape::Any {
        spans.push(Span::styled(
            format!("  {}", lens.shape.label().to_lowercase()),
            Style::default().fg(Color::Rgb(122, 162, 247)),
        ));
    }
    if let Some(repo) = &lens.repo {
        spans.push(Span::styled(
            format!("  in {repo}"),
            Style::default().fg(Color::Rgb(122, 162, 247)),
        ));
    }
    f.render_widget(Line::from(spans), area);
}

const GUTTER: usize = 14;
const ROW_HEIGHT: u16 = 3;
const RULE: Color = Color::Rgb(52, 56, 64);

fn row(it: &Item, width: usize) -> ListItem<'static> {
    let (label, colour) = match it.kind {
        Kind::Pr => ("[PR]", Color::Blue),
        Kind::Issue => ("[ISSUE]", Color::Cyan),
    };
    let slug = it.slug();
    let budget = width.saturating_sub(GUTTER + slug.chars().count() + 2).max(16);

    let top = Line::from(vec![
        Span::styled(
            format!("{:>4}  ", ago(it.updated_at)),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(format!("{label:<7}"), Style::default().fg(colour)),
        Span::raw(" "),
        Span::styled(slug, Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(clip(&it.title, budget), Style::default().bold()),
    ]);

    let mut chips: Vec<Span> = vec![Span::raw(" ".repeat(GUTTER))];
    for (text, t) in it.chips() {
        chips.push(Span::styled(text, tone(t)));
        chips.push(Span::styled(" \u{b7} ", Style::default().fg(Color::DarkGray)));
    }
    chips.pop();

    let mut tail = String::new();
    if it.comments > 0 {
        tail.push_str(&format!("   {}", plural(it.comments, "comment")));
    }
    if let Some(a) = it.activity() {
        tail.push_str(&format!("   {a}"));
    }
    if !tail.is_empty() {
        chips.push(Span::styled(tail, Style::default().fg(Color::DarkGray)));
    }

    ListItem::new(vec![top, Line::from(chips), Line::raw("")])
}

fn list(f: &mut Frame, area: Rect, app: &mut App) {
    if app.view.is_empty() {
        let msg = match &app.who {
            Some(who) if app.who_loading => format!("loading {who}"),
            Some(who) if app.items.is_empty() => {
                format!("nothing for {who} that you can see")
            }
            None if app.items.is_empty() => "nothing cached yet, press r to sync".into(),
            _ => "nothing matches this filter".into(),
        };
        f.render_widget(
            Paragraph::new(msg).style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }
    let width = area.width as usize;
    let rows: Vec<ListItem> = app.view.iter().map(|&i| row(&app.items[i], width)).collect();
    let mut state = ListState::default().with_selected(Some(app.cursor));
    let widget = List::new(rows)
        .highlight_style(Style::default().bg(Color::Rgb(30, 40, 55)))
        .highlight_symbol("");
    f.render_stateful_widget(widget, area, &mut state);
    separators(f, area, app.view.len().saturating_sub(state.offset()));
}

fn detail(f: &mut Frame, area: Rect, app: &App) {
    let Some(it) = app.selected() else { return };
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(it.slug(), Style::default().fg(Color::Cyan).bold()),
            Span::raw("  "),
            Span::styled(it.title.clone(), Style::default().bold()),
        ]),
        Line::from({
            let mut chips: Vec<Span> = Vec::new();
            for (text, t) in it.chips() {
                chips.push(Span::styled(text, tone(t)));
                chips.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            }
            chips.pop();
            chips
        }),
        Line::raw(""),
    ];

    if app.thread_loading() {
        lines.push(Line::from(Span::styled(
            "loading the conversation",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let events = app.thread();
    let mut hidden = 0usize;
    for e in events.into_iter().flatten() {
        if e.bot && !app.show_bots {
            hidden += 1;
            let gist = e.gist();
            lines.push(Line::from(vec![
                Span::styled(format!("{:>4} ", ago(e.at)), Style::default().fg(Color::DarkGray)),
                Span::styled(e.who.clone(), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if gist.is_empty() { String::new() } else { format!("  {gist}") },
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            continue;
        }
        let accent = match e.label.as_str() {
            "approved" => Color::Green,
            "requested changes" => Color::Red,
            "opened" => Color::Cyan,
            l if l.starts_with("on ") => Color::Yellow,
            _ => Color::Blue,
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:>4} ", ago(e.at)), Style::default().fg(Color::DarkGray)),
            Span::styled(e.who.clone(), Style::default().fg(accent).bold()),
            Span::raw(" "),
            Span::styled(e.label.clone(), Style::default().fg(accent)),
        ]));
        for raw in e.body.lines() {
            lines.push(Line::from(Span::raw(format!("     {raw}"))));
        }
        lines.push(Line::raw(""));
    }

    if events.is_some_and(|e| e.is_empty()) {
        lines.push(Line::from(Span::styled(
            "no conversation on this one",
            Style::default().fg(Color::DarkGray),
        )));
    }
    if hidden > 0 {
        lines.push(Line::from(Span::styled(
            format!("{} bot {} collapsed, press b to expand", hidden, if hidden == 1 { "message" } else { "messages" }),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let max = lines.len().saturating_sub(area.height as usize / 2) as u16;
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).scroll((app.scroll.min(max), 0)),
        area,
    );
}

fn separators(f: &mut Frame, area: Rect, visible: usize) {
    let style = Style::default().fg(RULE).bg(Color::Reset);
    let buf = f.buffer_mut();
    for k in 0..visible {
        let y = area.y + k as u16 * ROW_HEIGHT + ROW_HEIGHT - 1;
        if y >= area.y + area.height {
            break;
        }
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char('\u{254c}');
                cell.set_style(style);
            }
        }
    }
}

fn footer(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(it) = app.selected() {
        lines.push(Line::from(vec![
            Span::styled(it.url.clone(), Style::default().fg(Color::Blue).underlined()),
        ]));
        let mut meta = vec![format!("by {}", it.author)];
        if it.kind == Kind::Pr {
            meta.push(format!(
                "+{} -{} in {}",
                it.additions,
                it.deletions,
                plural(it.changed_files, "file")
            ));
        }
        if !it.assignees.is_empty() {
            meta.push(format!("assigned {}", it.assignees.join(", ")));
        }
        if it.mergeable_unknown {
            meta.push("merge state still computing".into());
        }
        lines.push(Line::from(Span::styled(
            meta.join("   "),
            Style::default().fg(Color::Gray),
        )));
        if !it.labels.is_empty() {
            lines.push(Line::from(Span::styled(
                it.labels.join(", "),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let hint = if app.asking {
        format!("username: {}_", app.ask)
    } else if app.searching {
        format!("search: {}_", app.search)
    } else if !app.status.is_empty() {
        app.status.clone()
    } else if app.detail {
        "↑↓ scroll · b bots · r reload · o open · ← back".into()
    } else if app.who.is_some() {
        "↑↓ move · ←→ filter · enter read · o open · r reload · u someone else · esc back to yours".into()
    } else {
        "↑↓ move · ←→ filter · enter read · o open · r sync · u user · / search · ? help · q quit".into()
    };
    let mut bar = vec![Span::styled(hint, Style::default().fg(Color::DarkGray))];
    bar.push(Span::styled(
        match app.last_sync {
            Some(t) => format!("   synced {} ago", ago(t)),
            None => "   never synced".into(),
        },
        Style::default().fg(Color::DarkGray),
    ));
    bar.push(Span::styled(
        format!(", watching every {}s", app.interval),
        Style::default().fg(Color::DarkGray),
    ));
    if !app.search.is_empty() && !app.searching {
        bar.push(Span::styled(
            format!("   filtering on \"{}\"", app.search),
            Style::default().fg(Color::Yellow),
        ));
    }
    if let Some(st) = app.stats {
        bar.push(Span::styled(
            format!("   {} api pts left", st.remaining),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(bar));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

const HEAT: [Color; 5] = [
    Color::Rgb(38, 42, 50),
    Color::Rgb(26, 92, 45),
    Color::Rgb(22, 129, 59),
    Color::Rgb(0, 165, 70),
    Color::Rgb(23, 200, 91),
];

fn tile(label: &str, value: String, note: String, lit: Color) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(format!("  {label}"), Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(format!("  {value}"), Style::default().fg(lit).bold())),
        Line::from(Span::styled(format!("  {note}"), Style::default().fg(Color::DarkGray))),
    ]
}

fn tiles(f: &mut Frame, area: Rect, cells: Vec<Vec<Line<'static>>>) {
    if cells.is_empty() || area.width < 20 {
        return;
    }
    let across = cells.len().min(((area.width / 18).max(1)) as usize).min(6);
    let rows = cells.len().div_ceil(across);
    let bands = Layout::vertical(vec![Constraint::Length(3); rows]).split(area);
    for (r, band) in bands.iter().enumerate() {
        let slice: Vec<_> = cells.iter().skip(r * across).take(across).collect();
        if slice.is_empty() {
            break;
        }
        let cols = Layout::horizontal(vec![
            Constraint::Ratio(1, slice.len() as u32);
            slice.len()
        ])
        .split(*band);
        for (c, cell) in slice.iter().enumerate() {
            f.render_widget(Paragraph::new((*cell).clone()), cols[c]);
        }
    }
}

/// The contribution graph, drawn the way github draws it: weeks across, days down.
fn calendar(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RULE))
        .title(" contributions, last 12 months ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 8 {
        return;
    }

    let Some(cal) = &app.calendar else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  reading the graph from github...",
                Style::default().fg(Color::DarkGray),
            ))),
            inner,
        );
        return;
    };
    if cal.days.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  github has no public contributions for them",
                Style::default().fg(Color::DarkGray),
            ))),
            inner,
        );
        return;
    }

    // one cell is two columns wide, so the grid reads as squares not slivers
    let weeks_that_fit = ((inner.width.saturating_sub(5)) / 2) as usize;
    let weeks: Vec<&[ghdeck_core::github::Day]> = cal.days.chunks(7).collect();
    let show = weeks.len().min(weeks_that_fit);
    let from = weeks.len() - show;

    let top = cal.days.iter().map(|d| d.count).max().unwrap_or(1).max(1);
    let shade = |n: u32| {
        if n == 0 {
            HEAT[0]
        } else {
            HEAT[(1 + (n * 3 / top).min(3)) as usize]
        }
    };

    let names = ["", "Mon", "", "Wed", "", "Fri", ""];
    let mut lines = Vec::new();
    for (row, name) in names.iter().enumerate() {
        let mut spans =
            vec![Span::styled(format!("{name:>4} "), Style::default().fg(Color::DarkGray))];
        for w in &weeks[from..] {
            match w.get(row) {
                Some(day) => spans.push(Span::styled("  ", Style::default().bg(shade(day.count)))),
                None => spans.push(Span::raw("  ")),
            }
        }
        lines.push(Line::from(spans));
    }

    let mut key = vec![Span::styled(
        format!("     {} contributions    less ", cal.total),
        Style::default().fg(Color::DarkGray),
    )];
    for c in HEAT {
        key.push(Span::styled("  ", Style::default().bg(c)));
    }
    key.push(Span::styled(" more", Style::default().fg(Color::DarkGray)));
    lines.push(Line::from(""));
    lines.push(Line::from(key));
    f.render_widget(Paragraph::new(lines), inner);
}

fn stats(f: &mut Frame, area: Rect, app: &App) {
    let shown = app.for_stats();
    let me = app.subject_name().to_string();
    let c = ghdeck_core::stats::counts(&shown, &me);
    let w = ghdeck_core::stats::work(&shown, &me, chrono::Utc::now());
    let pc = ghdeck_core::stats::percent;

    let [caption, top, mid, _rest] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Length(12),
        Constraint::Min(0),
    ])
    .areas(area);

    f.render_widget(
        Line::from(Span::styled(
            format!("  {} prs and issues, all time", shown.len()),
            Style::default().fg(Color::DarkGray),
        )),
        caption,
    );

    let good = Color::Rgb(0, 179, 74);
    let info = Color::Rgb(154, 125, 235);
    let blue = Color::Rgb(122, 162, 247);

    tiles(
        f,
        top,
        vec![
            tile("they opened", (w.prs_opened + w.issues_opened).to_string(), format!("in {} of {} repos", w.repos, w.repos_touched), blue),
            tile("pull requests", c.prs.to_string(), format!("{} merged", c.merged), info),
            tile("issues", c.issues.to_string(), format!("{} still open", c.open_issues), good),
            tile(
                "merge rate",
                pc(w.prs_merged, w.prs_opened).map_or_else(|| "-".into(), |n| format!("{n}%")),
                match w.prs_opened {
                    0 => "they opened none here".into(),
                    n => format!("of {n} they opened"),
                },
                info,
            ),
            tile(
                "time to merge",
                w.median_merge_hours.map(ghdeck_core::stats::spell_hours).unwrap_or_else(|| "-".into()),
                match w.median_merge_hours {
                    Some(_) => "middle of their merged prs".into(),
                    None => "press R for a deep sync".to_string(),
                },
                Color::Yellow,
            ),
            tile("others' work", (c.reviews + c.discussions).to_string(), "they took part in".into(), blue),
        ],
    );

    calendar(f, mid, app);
}

fn help(f: &mut Frame, area: Rect) {
    let keys = [
        ("up / down", "move"),
        ("left / right", "change filter"),
        ("1 to 5", "jump to a filter"),
        ("p", "prs, issues, or both"),
        ("e", "only the repo under the cursor"),
        ("a", "stats"),
        ("enter", "read the conversation"),
        ("g / G", "top / bottom"),
        ("o", "open in browser"),
        ("y", "copy url"),
        ("r", "sync now"),
        ("R", "deep sync, more pages"),
        ("/", "search title, repo, label"),
        ("u", "someone else's prs and issues"),
        ("esc", "clear search, or back to your list"),
        ("?", "close help"),
        ("q", "quit"),
        ("", ""),
        ("in a conversation", ""),
        ("up / down", "scroll"),
        ("b", "expand bot messages"),
        ("r", "reload it"),
        ("left, esc", "back to the list"),
    ];
    let lines: Vec<Line> = keys
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("  {k:<16}"), Style::default().fg(Color::Cyan)),
                Span::raw(*v),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}
