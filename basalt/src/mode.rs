#[derive(Debug, Clone, Default, PartialEq)]
pub enum Mode {
    #[default]
    Select,
    Normal,
    Insert,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Select => "Select",
            Mode::Normal => "Normal",
            Mode::Insert => "Insert",
        }
    }
}
