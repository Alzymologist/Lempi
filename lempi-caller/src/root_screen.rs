use termwiz::widgets::{layout, RenderArgs, Widget};

pub struct MainScreen {}

impl MainScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl Widget for MainScreen {
    fn render(&mut self, _args: &mut RenderArgs) {}

    fn get_size_constraints(&self) -> layout::Constraints {
        // Switch from default horizontal layout to vertical layout
        let mut c = layout::Constraints::default();
        c.child_orientation = layout::ChildOrientation::Vertical;
        c
    }
}
