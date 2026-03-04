use protocol::{AttentionLevel, WorkspaceSummary};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub const TILE_W: u16 = 38;
pub const TILE_H: u16 = 5;
const ORANGE: Color = Color::Rgb(255, 165, 0);

/// Renders the workspace tile grid into `area`.
///
/// Each workspace in `items` is displayed as a fixed-size rounded card.
/// `selected` highlights the focused tile; `flash_on` drives attention pulse.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    items: &[WorkspaceSummary],
    selected: usize,
    flash_on: bool,
) {
    if items.is_empty() {
        render_empty_state(frame, area);
        return;
    }

    let cols = (area.width / TILE_W).max(1) as usize;
    for (i, ws) in items.iter().enumerate() {
        let tile = tile_rect(area, i, cols);
        if tile.width < 8 || tile.height < 5 {
            continue;
        }
        render_tile(frame, tile, ws, i == selected, flash_on);
    }
}

/// Returns the tile index at pixel coordinate (`x`, `y`) within `area`,
/// or `None` if the coordinate falls outside all tiles.
pub fn index_at(area: Rect, x: u16, y: u16, item_count: usize) -> Option<usize> {
    if item_count == 0 {
        return None;
    }
    if x < area.x || y < area.y || x >= area.right() || y >= area.bottom() {
        return None;
    }
    let rel_x = x - area.x;
    let rel_y = y - area.y;
    let cols = (area.width / TILE_W).max(1) as usize;
    let col = (rel_x / TILE_W) as usize;
    let row = (rel_y / TILE_H) as usize;
    let idx = row * cols + col;
    (idx < item_count).then_some(idx)
}

/// Draws the placeholder shown when there are no workspaces.
fn render_empty_state(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title("Workspaces")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White));
    frame.render_widget(
        Paragraph::new("No workspaces yet. Press `n` to add current directory.").block(block),
        area,
    );
}

/// Computes the `Rect` for tile at grid position `index` given `cols` columns.
fn tile_rect(area: Rect, index: usize, cols: usize) -> Rect {
    let row = index / cols;
    let col = index % cols;
    Rect {
        x: area.x + (col as u16 * TILE_W),
        y: area.y + (row as u16 * TILE_H),
        width: TILE_W.min(area.width.saturating_sub(col as u16 * TILE_W)),
        height: TILE_H.min(area.height.saturating_sub(row as u16 * TILE_H)),
    }
}

/// Renders a single workspace tile into `tile`.
fn render_tile(
    frame: &mut Frame,
    tile: Rect,
    ws: &WorkspaceSummary,
    is_selected: bool,
    flash_on: bool,
) {
    let border_style = tile_border_style(ws, is_selected, flash_on);
    let title_left = Line::from(Span::styled(
        format!(" {} ", ws.name),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));
    let title_right = build_status_badge(&ws.attention, flash_on);
    let body_line = build_body_line(ws);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title_top(title_left)
        .title_top(title_right.right_aligned());

    frame.render_widget(
        Paragraph::new(vec![Line::from(""), body_line]).block(block),
        tile,
    );
}

/// Computes the border style based on attention level, selection, and flash phase.
fn tile_border_style(ws: &WorkspaceSummary, is_selected: bool, flash_on: bool) -> Style {
    let base = match ws.attention {
        AttentionLevel::Error => {
            if flash_on {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::LightRed)
            }
        }
        AttentionLevel::NeedsInput => {
            if flash_on {
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            }
        }
        _ if ws.agent_running => Style::default().fg(Color::White),
        _ => Style::default().fg(Color::White),
    };

    if !is_selected {
        return base;
    }

    let needs_attention = matches!(
        ws.attention,
        AttentionLevel::NeedsInput | AttentionLevel::Error
    );
    if needs_attention && flash_on {
        base
    } else {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    }
}

/// Builds the right-aligned status badge for attention states.
/// Returns an empty line for non-attention tiles.
fn build_status_badge(attention: &AttentionLevel, flash_on: bool) -> Line<'static> {
    match attention {
        AttentionLevel::NeedsInput => {
            let style = Style::default().fg(ORANGE);
            Line::from(Span::styled(" ⚠ input ", flash_bold(style, flash_on)))
        }
        AttentionLevel::Error => {
            let style = Style::default().fg(Color::Red);
            Line::from(Span::styled(" ✖ error ", flash_bold(style, flash_on)))
        }
        _ => Line::from(""),
    }
}

/// Builds the single-line metadata row: branch, dirty count, agent status.
fn build_body_line(ws: &WorkspaceSummary) -> Line<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    let branch = ws.branch.as_deref().unwrap_or("-");
    Line::from(vec![
        Span::styled(" ⎇ ", dim),
        Span::styled(
            truncate_end(branch, 12),
            Style::default().fg(Color::White),
        ),
        Span::styled("  ◈ ", dim),
        Span::styled(
            ws.dirty_files.to_string(),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("  ● ", dim),
        Span::styled(
            if ws.agent_running { "agent" } else { "off" },
            if ws.agent_running {
                Style::default().fg(Color::Green)
            } else {
                dim
            },
        ),
    ])
}

/// Returns `style` with `BOLD` added when `flash_on` is true.
fn flash_bold(style: Style, flash_on: bool) -> Style {
    if flash_on {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

/// Truncates `input` to at most `max` characters, appending `…` if shortened.
fn truncate_end(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let mut s: String = input.chars().take(max - 1).collect();
    s.push('…');
    s
}
