use ratatui::{
    layout::{Constraint, Flex, Layout},
    text::Text,
    widgets::{Block, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::datatypes;

/// Handle scroll.
pub struct Notebook<'a> {
    pub data: &'a datatypes::Notebook,
    /// The currently selected cell index
    pub selected: usize,
}

impl<'a> Notebook<'a> {
    pub fn new(data: &'a datatypes::Notebook) -> Self {
        Self { data, selected: 0 }
    }
}

impl<'a> StatefulWidget for Notebook<'a> {
    type State = Option<(u16, u16)>;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let h_layout = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(1),
        ])
        .split(area);

        let mut acc_height = 0;
        let mut constraints = vec![];
        let mut widgets = vec![];

        for cell in &self.data.cells {
            let bordered_height = Cell::get_height(cell) as u16;
            let next_acc_height = acc_height + bordered_height;
            if next_acc_height > area.height {
                break;
            }
            constraints.push(Constraint::Length(bordered_height));
            widgets.push(Cell::new(cell));
            acc_height = next_acc_height;
        }
        let v_layout = Layout::vertical(constraints)
            .flex(Flex::Start)
            .split(h_layout[1]);
        for (index, widget) in widgets.into_iter().enumerate() {
            widget.render(v_layout[index], buf, state);
        }
    }
}

pub struct Cell<'a> {
    pub data: &'a datatypes::Cell,
}

impl<'a> Cell<'a> {
    pub fn new(data: &'a datatypes::Cell) -> Self {
        Self { data }
    }

    pub fn get_height(data: &datatypes::Cell) -> usize {
        data.code.chars().filter(|ch| *ch == '\n').count() + 3
    }
}

impl<'a> StatefulWidget for Cell<'a> {
    type State = Option<(u16, u16)>;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let block = Block::bordered();
        let inner_area = block.inner(area);
        let textarea = Textarea::new(&self.data.code);
        block.render(area, buf);
        StatefulWidget::render(textarea, inner_area, buf, state);
    }
}

pub struct Textarea<'a> {
    pub data: &'a str,
    pub focused: bool,
}

impl<'a> Textarea<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn caret_x(&self) -> usize {
        match self.data.rfind('\n') {
            Some(index) => self.data[(index + 1)..].width(),
            None => self.data.width(),
        }
    }

    pub fn caret_y(&self) -> usize {
        self.data.chars().filter(|&c| c == '\n').count()
    }

    pub fn caret(&self) -> (usize, usize) {
        (self.caret_x(), self.caret_y())
    }
}

pub type TextareaState = Option<(u16, u16)>;

impl<'a> StatefulWidget for Textarea<'a> {
    type State = TextareaState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if self.focused {
            *state = Some((
                // the two `2` below is the border size
                area.x + self.caret_x() as u16,
                area.y + self.caret_y() as u16,
            ));
        }
        Text::from(self.data).render(area, buf);
    }
}
