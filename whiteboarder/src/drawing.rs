use crate::data;
use tera::{Context, Tera};

pub fn draw_svg(board: &mut data::Board, background_color: Option<String>) -> String {
    let mut context = Context::new();
    context.insert("width", &board.width());
    context.insert("height", &board.height());
    context.insert("background_color", &background_color.unwrap_or_else(|| String::from("transparent")));
    let strokes: Vec<data::TeraStroke> = board.strokes.iter().map(|s| s.renderable(board.x_offset(), board.y_offset())).collect();
    context.insert("strokes", &strokes);
    let template = Tera::new("assets/*.svg").expect("Failed to parse templates");
    template.render("template.svg", &context).expect("Failed to render template.svg")
}
