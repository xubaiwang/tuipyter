use ratatui::{
    crossterm::event::{Event, KeyCode, read},
    text::Text,
    widgets::{Block, Widget},
};
use tuipyter::{
    datatypes::{Cell, Notebook},
    widgets,
};
use unicode_width::UnicodeWidthStr;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let my_notebook = Notebook {
        cells: vec![
            Cell {
                code: "console.log(123)".into(),
                result: None,
            },
            Cell {
                code: "console.log(true)".into(),
                result: None,
            },
        ],
    };

    loop {
        terminal.draw(|frame| {
            let state = &mut None;

            let notebook = widgets::Notebook::new(&my_notebook);
            frame.render_stateful_widget(notebook, frame.area(), state);

            if let Some(position) = *state {
                frame.set_cursor_position(position);
            }
        })?;

        if let Event::Key(key) = read()? {
            match key.code {
                KeyCode::Esc => break,
                // KeyCode::Char(ch) => {
                //     buf.push(ch);
                // }
                // KeyCode::Enter => buf.push('\n'),
                // KeyCode::Backspace => {
                //     buf.pop();
                // }
                _ => {}
            }
        }
    }

    ratatui::restore();

    Ok(())
}

struct Textarea<'a> {
    text: &'a str,
    focused: bool,
}

impl<'a> Textarea<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn line_count(&self) -> usize {
        self.text.chars().filter(|&c| c == '\n').count() + 1
    }

    pub fn last_line_offset(&self) -> usize {
        match self.text.rfind('\n') {
            Some(index) => self.text[(index + 1)..].width(),
            None => self.text.width(),
        }
    }

    pub fn height(&self) -> u16 {
        self.line_count() as u16 + 4
    }
}

impl<'a> Widget for Textarea<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // def
        let outer_block = Block::bordered().title("[1]:");
        let inner_block = Block::bordered();
        let text = Text::from(self.text);

        // layout
        let inner_area = outer_block.inner(area);
        let inner_inner_area = inner_block.inner(inner_area);

        // render
        outer_block.render(area, buf);
        inner_block.render(inner_area, buf);
        text.render(inner_inner_area, buf);
    }
}
