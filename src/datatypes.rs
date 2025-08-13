//! Simplified representation of Jupyter Notebook.

pub struct Notebook {
    pub cells: Vec<Cell>,
}

pub struct Cell {
    pub code: String,
    pub result: Option<String>,
}
