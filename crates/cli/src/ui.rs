use crate::app::App;
use ghwork_core::{ago, plural, Filter, Item, Kind, Tone};
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
    let [head, body, foot] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(4), Constraint::Length(6)])
            .areas(f.area());

    header(f, head, app);
    if app.help {
        help(f, body);
    } else if app.detail {
        detail(f, body, app);
    } else {
        list(f, body, app);
    }
    footer(f, foot, app);
}

fn header(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(" ghwork ", Style::default().bold().fg(Color::Cyan))];
    for (i, filt) in Filter::ORDER.iter().enumerate() {
        let n = app.count(*filt);
        let active = *filt == app.filter;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {}:{} {} ", i + 1, filt.label(), n), style));
    }
    if app.busy() {
        spans.push(Span::styled("  syncing", Style::default().fg(Color::Yellow)));
    }
    f.render_widget(Line::from(spans), area);
}

fn row(it: &Item, width: usize) -> ListItem<'static> {
    let kind = match it.kind {
        Kind::Pr => Span::styled("PR", Style::default().fg(Color::Blue).bold()),
        Kind::Issue => Span::styled("IS", Style::default().fg(Color::Cyan).bold()),
    };
    let slug = it.slug();
    let budget = width.saturating_sub(slug.len() + 14).max(20);
    let mut title = it.title.clone();
    if title.chars().count() > budget {
        title = title.chars().take(budget.saturating_sub(1)).collect::<String>() + "…";
    }

    let top = Line::from(vec![
        Span::styled(format!("{:>4} ", ago(it.updated_at)), Style::default().fg(Color::DarkGray)),
        kind,
        Span::raw(" "),
        Span::styled(slug, Style::default().bold()),
        Span::raw("  "),
        Span::raw(title),
    ]);

    let mut chips: Vec<Span> = vec![Span::raw("     ")];
    for (text, t) in it.chips() {
        chips.push(Span::styled(text, tone(t)));
        chips.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
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

    ListItem::new(vec![top, Line::from(chips)])
}

fn list(f: &mut Frame, area: Rect, app: &mut App) {
    if app.view.is_empty() {
        let msg = if app.items.is_empty() {
            "nothing cached yet, press r to sync"
        } else {
            "nothing matches this filter"
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

    let hint = if app.searching {
        format!("search: {}_", app.search)
    } else if !app.status.is_empty() {
        app.status.clone()
    } else if app.detail {
        "j k scroll · b bots · r reload · o open · esc back".into()
    } else {
        "j k move · enter read · tab filter · o open · r sync · / search · ? help · q quit".into()
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

fn help(f: &mut Frame, area: Rect) {
    let keys = [
        ("j / k, arrows", "move"),
        ("g / G", "top / bottom"),
        ("tab, shift-tab", "cycle filter"),
        ("1 to 5", "jump to filter"),
        ("enter, l", "read the conversation"),
        ("b", "expand bot messages in a conversation"),
        ("o", "open in browser"),
        ("y", "copy url"),
        ("r", "sync now"),
        ("R", "deep sync, more pages"),
        ("/", "search title, repo, label"),
        ("esc", "clear search"),
        ("?", "close help"),
        ("q", "quit"),
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
