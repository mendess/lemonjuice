pub mod color;

use cairo::{Context, Surface};
use color::Color;
use pango::{EllipsizeMode, FontDescription};
use pangocairo;
use paste;
use std::fmt;

macro_rules! padding_builder {
    ($name:ident) => {
        #[allow(dead_code)]
        pub fn $name($name: f64) -> Self {
            Self {
                $name,
                ..Default::default()
            }
        }

        paste::item! {
        #[allow(dead_code)]
        pub fn [<with_ $name>](self, $name: f64) -> Self {
            Self { $name, ..self }
        }
        }
    };
}

#[derive(Copy, Default, Clone, Debug, PartialEq)]
pub struct Padding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Padding {
    #[allow(dead_code)]
    pub fn new(left: f64, right: f64, top: f64, bottom: f64) -> Padding {
        Padding {
            left,
            right,
            top,
            bottom,
        }
    }
    padding_builder!(left);
    padding_builder!(right);
    padding_builder!(top);
    padding_builder!(bottom);
}

#[derive(Default, Clone, PartialEq)]
pub struct Font(FontDescription);

impl From<&str> for Font {
    fn from(name: &str) -> Self {
        Self(FontDescription::from_string(name))
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attributes {
    pub font: Font,
    pub fg_color: Color,
    pub bg_color: Option<Color>,
    pub padding: Padding,
}

impl Attributes {
    #[allow(dead_code)]
    pub fn with_padding(self, padding: Padding) -> Self {
        Self { padding, ..self }
    }

    #[allow(dead_code)]
    pub fn with_fg_color(self, fg_color: Color) -> Self {
        Self { fg_color, ..self }
    }

    #[allow(dead_code)]
    pub fn with_bg_color<C: Into<Option<Color>>>(self, bg_color: C) -> Self {
        Self {
            bg_color: bg_color.into(),
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn with_font(self, font: Font) -> Self {
        Self { font, ..self }
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            font: Font(FontDescription::default()),
            fg_color: Color::white(),
            bg_color: None,
            padding: Padding::default(),
        }
    }
}

fn create_pango_layout(cairo_context: &cairo::Context) -> Result<pango::Layout, String> {
    let layout = pangocairo::functions::create_layout(cairo_context)
        .ok_or_else(|| String::from("Failed to create Pango layout"))?;
    Ok(layout)
}

fn show_pango_layout(cairo_context: &cairo::Context, layout: &pango::Layout) {
    pangocairo::functions::show_layout(cairo_context, layout);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub attr: Attributes,
    pub text: String,
}

impl Text {
    pub(crate) fn compute(self, surface: &Surface) -> Result<ComputedText, String> {
        let (width, height) = {
            let context = Context::new(&surface);
            let layout = create_pango_layout(&context)?;
            layout.set_text(&self.text);
            layout.set_font_description(Some(&self.attr.font.0));

            let padding = &self.attr.padding;
            let (text_width, text_height) = layout.get_pixel_size();
            let width = f64::from(text_width) + padding.left + padding.right;
            let height = f64::from(text_height) + padding.top + padding.bottom;
            (width, height)
        };

        Ok(ComputedText {
            attr: self.attr,
            text: self.text,
            x: 0.0,
            y: 0.0,
            width,
            height,
        })
    }
}

// This impl allows us to see whether a widget's text has changed without
// having to call the (relatively) expensive .compute().
impl PartialEq<ComputedText> for Text {
    fn eq(&self, other: &ComputedText) -> bool {
        self.attr == other.attr && self.text == other.text
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedText {
    pub attr: Attributes,
    pub text: String,

    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ComputedText {
    pub fn render(&self, surface: &Surface) -> Result<(), String> {
        let context = Context::new(&surface);
        let layout = create_pango_layout(&context)?;
        layout.set_text(&self.text);
        layout.set_font_description(Some(&self.attr.font.0));

        context.translate(self.x, self.y);

        // Set the width/height on the Pango layout so that it word-wraps/ellipises.
        let padding = &self.attr.padding;
        let text_width = self.width - padding.left - padding.right;
        let text_height = self.height - padding.top - padding.bottom;
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_width(text_width as i32 * pango::SCALE);
        layout.set_height(text_height as i32 * pango::SCALE);

        let bg_color = &self.attr.bg_color.clone().unwrap_or_else(Color::black);
        bg_color.apply_to_context(&context);
        // FIXME: The use of `height` isnt' right here: we want to do the
        // full height of the bar, not the full height of the text. It
        // would be useful if we could do Surface.get_height(), but that
        // doesn't seem to be available in cairo-rs for some reason?
        context.rectangle(0.0, 0.0, self.width, self.height);
        context.fill();

        self.attr.fg_color.apply_to_context(&context);
        context.translate(padding.left, padding.top);
        show_pango_layout(&context, &layout);

        Ok(())
    }
}
