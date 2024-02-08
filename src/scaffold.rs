use termwiz::terminal::ScreenSize;
use termwiz::surface::Surface;

/// This thing tells all surfaces where they belong
///
/// Might get more fields later to allow resizing; but who needs it anyway?
pub struct Scaffold {
    size: ScreenSize,
}

/// This holds where the surface is
pub struct Location {
    /// top left vertically
    line: usize,
    /// top left horizontally
    column: usize,
    /// size vertically
    height: usize,
    /// size horizontally
    width: usize,
}

impl Location {
    pub fn line(&self) -> usize { self.line }

    pub fn column(&self) -> usize { self.column }

    pub fn width(&self) -> usize { self.width }

    pub fn height(&self) -> usize { self.height }

    pub fn surface(&self) -> Surface {
        Surface::new(self.width, self.height)
    }
}

impl Scaffold {
    pub fn new(size: ScreenSize) -> Self {
        Self{size}
    }

    pub fn resize(&mut self, size: ScreenSize) {
        self.size = size;
    }

    fn vsplit(&self) -> usize {
        self.size.cols * 3 / 4
    }

    pub fn header(&self) -> Location {
        Location { line: 0, column: 0, height: 1, width: self.size.cols }
    }

    pub fn block(&self) -> Location {
        Location { line: self.size.rows, column: 0, height: 1, width: self.size.cols }
    }

    pub fn author(&self) -> Location {
        let line = 2;
        let column = 0;
        let width = self.vsplit() - 1;
        let height = self.size.rows / 10;
        Location{line, column, height, width}
    }

    pub fn call(&self) -> Location {
        let line = self.size.rows / 10;
        let column = 0;
        let width = self.vsplit() - 1;
        let height = self.size.rows * 6 / 10;
        Location{line, column, height, width}
    }

    pub fn extensions(&self) -> Location {
        let line = self.size.rows * 6 / 10;
        let column = 0;
        let width = self.vsplit() - 1;
        let height = self.size.rows - 1;
        Location{line, column, height, width}
    }
}

