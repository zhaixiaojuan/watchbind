mod lines;

use crate::config::Styles;
use anyhow::Result;
use itertools::izip;
use lines::Lines;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};
use std::cmp::max;

pub struct State {
    lines: Lines,
    selected: Vec<bool>,
    styles: Styles,
    cursor: Option<usize>,
    first_index: usize,
    // TODO: deprecate in future
    table_state: TableState,
    // help_menu_body: Vec<String>,
    show_help_menu: bool,
}

impl State {
    pub fn new(header_lines: usize, field_separator: Option<String>, styles: Styles) -> Self {
        Self {
            lines: Lines::new(field_separator, styles.clone(), header_lines),
            selected: vec![],
            styles,
            cursor: None,
            first_index: header_lines,
            table_state: TableState::default(),
            show_help_menu: false,
        }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>) {
        self.render_lines(frame);

        if self.show_help_menu {
            self.render_help_menu(frame);
        }
    }

    fn render_lines<B: Backend>(&mut self, frame: &mut Frame<B>) {
        // TODO: do as much as possible in update_lines to improve performance
        let rows: Vec<Row> = izip!(&self.lines, &self.selected)
            .map(|(line, &selected)| {
                // TODO: consider replacing Vec<bool> with Vec<Style> directly
                let selected_style = if selected {
                    self.styles.selected
                } else {
                    self.styles.line
                };

                Row::new(vec![Cell::from(" ").style(selected_style), line.draw()])
            })
            .collect();

        let table = Table::new(rows)
            .widths(&[Constraint::Length(1), Constraint::Percentage(100)])
            .column_spacing(0);

        frame.render_stateful_widget(table, frame.size(), &mut self.table_state);
    }

    fn render_help_menu<B: Backend>(&mut self, frame: &mut Frame<B>) {
        let area = centered_rect(50, 50, frame.size());

        let text = vec![
            Spans::from(Span::raw("TODO:")),
            Spans::from(Span::raw("Show all defined keybindings here.")),
        ];
        // TODO: don't trim
        let paragraph = Paragraph::new(text)
            .block(Block::default().title("help").borders(Borders::ALL))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    pub fn update_lines(&mut self, new_lines: String) -> Result<()> {
        self.lines.update(new_lines)?;
        self.selected.resize(self.lines.len(), false);
        self.cursor_calibrate();
        Ok(())
    }

    fn cursor_position(&mut self) -> Option<usize> {
        self.cursor
    }

    fn cursor_move(&mut self, index: isize) {
        let old = self.cursor_position();
        let new = if self.lines.is_empty() {
            None
        } else {
            let first = self.first_index as isize;
            let last = self.last_index() as isize;
            Some(index.clamp(first, last) as usize)
        };

        self.cursor = new;
        self.table_state.select(self.cursor);
        self.cursor_adjust_style(old, new);
    }

    fn cursor_calibrate(&mut self) {
        match self.cursor_position() {
            None => self.first(),
            Some(i) => self.cursor_move(i as isize),
        };
    }

    fn cursor_adjust_style(&mut self, old: Option<usize>, new: Option<usize>) {
        if let Some(old_index) = old {
            self.lines.update_style(old_index, self.styles.line);
        }
        if let Some(new_index) = new {
            self.lines.update_style(new_index, self.styles.cursor);
        }
    }

    fn get_cursor_line(&mut self) -> Option<String> {
        if let Some(i) = self.cursor_position() {
            self.lines.get_unformatted(i)
        } else {
            None
        }
    }

    pub fn get_selected_lines(&mut self) -> Option<String> {
        if self.selected.contains(&true) {
            let lines: String = izip!(self.lines.unformatted(), self.selected.iter())
                .filter_map(|(line, &selected)| selected.then(|| line.to_owned()))
                .collect::<Vec<String>>()
                .join("\n");
            Some(lines)
        } else {
            self.get_cursor_line()
        }
    }

    pub fn down(&mut self, steps: usize) {
        if let Some(i) = self.cursor_position() {
            self.cursor_move(i as isize + steps as isize);
        }
    }

    pub fn up(&mut self, steps: usize) {
        if let Some(i) = self.cursor_position() {
            self.cursor_move(i as isize - steps as isize);
        }
    }

    pub fn first(&mut self) {
        self.cursor_move(self.first_index as isize);
    }

    pub fn last(&mut self) {
        self.cursor_move(self.last_index() as isize);
    }

    pub fn select(&mut self) {
        if let Some(i) = self.cursor_position() {
            if let Some(selected) = self.selected.get_mut(i) {
                *selected = true;
            }
        }
    }

    pub fn unselect(&mut self) {
        if let Some(i) = self.cursor_position() {
            if let Some(selected) = self.selected.get_mut(i) {
                *selected = false;
            }
        }
    }

    pub fn select_toggle(&mut self) {
        if let Some(i) = self.cursor_position() {
            if let Some(selected) = self.selected.get_mut(i) {
                *selected = !(*selected);
            }
        }
    }

    pub fn select_all(&mut self) {
        self.selected.fill(true);
    }

    pub fn unselect_all(&mut self) {
        self.selected.fill(false);
    }

    fn last_index(&self) -> usize {
        if self.lines.is_empty() {
            self.first_index
        } else {
            max(self.first_index, self.lines.len() - 1)
        }
    }

    pub fn toggle_help_menu(&mut self) {
        self.show_help_menu = !self.show_help_menu;
    }
}

/// Helper function to create a centered rect using up certain percentage
/// of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}