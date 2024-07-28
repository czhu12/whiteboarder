use crate::data;
use tera::{Context, Tera};

pub fn draw_svg(board: &mut data::Board) -> String {
    let mut context = Context::new();
    context.insert("width", &board.width());
    context.insert("height", &board.height());
    let strokes: Vec<data::TeraStroke> = board.strokes.iter().map(|s| s.renderable(board.x_offset(), board.y_offset())).collect();
    context.insert("strokes", &strokes);
    let template = Tera::new("assets/*.svg").expect("Failed to parse templates");
    template.render("template.svg", &context).expect("Failed to render template.svg")
}
