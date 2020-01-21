mod error;
mod one_or_more;
use crate::text::{color::Color, Attributes, Font, Padding, Text};
use error::ParseError;
use one_or_more::OneOrMore;
use std::{
    collections::HashMap,
    convert::TryFrom,
    io, mem,
    process::{Command, Stdio},
    str::{self, FromStr},
    sync::{Arc, RwLock},
    time::Duration,
};

pub type Config = HashMap<Alignment, Vec<Block>>;

pub fn parse(config: &str, monitor: usize) -> Result<(GlobalConfig, Config), ParseError> {
    let mut blocks = HashMap::<Alignment, Vec<Block>>::with_capacity(3);
    let mut blocks_iter = config.split("\n>");
    let global_config = blocks_iter
        .next()
        .map(GlobalConfig::try_from)
        .unwrap_or_else(|| Ok(Default::default()))?;
    for block in blocks_iter {
        let b: Block = Block::parse(block, monitor)?;
        blocks.entry(b.alignment).or_default().push(b);
    }
    Ok((global_config, blocks))
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Alignment {
    Left,
    Middle,
    Right,
}

impl FromStr for Alignment {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "left" | "Left" => Ok(Self::Left),
            "middle" | "Middle" => Ok(Self::Middle),
            "right" | "Right" => Ok(Self::Right),
            _ => Err("Invalid alignment"),
        }
    }
}

enum Content {
    Static(String),
    Cmd {
        cmd: String,
        last_run: OneOrMore<String>,
    },
    Persistent {
        cmd: String,
        last_run: OneOrMore<String>,
    },
}

impl Content {
    fn update(&mut self) {
        if let Self::Cmd { cmd, last_run } = self {
            dbg!(&cmd);
            for m in 0..last_run.len() {
                match Command::new("sh")
                    .args(&["-c", cmd])
                    .env("MONITOR", m.to_string())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .and_then(|c| c.wait_with_output())
                    .and_then(|o| {
                        if o.status.success() {
                            Ok(o.stdout)
                        } else {
                            Err(io::Error::from(io::ErrorKind::InvalidInput))
                        }
                    })
                    .map_err(|e| e.to_string())
                    .and_then(|o| String::from_utf8(o).map_err(|e| e.to_string()))
                    .map(|mut l| {
                        if let Some(i) = l.find('\n') {
                            l.truncate(i);
                            l
                        } else {
                            l
                        }
                    }) {
                    Ok(o) => last_run[m] = o,
                    Err(e) => last_run[m] = e,
                }
            }
            dbg!(last_run);
        }
    }

    fn is_empty(&self, monitor: usize) -> bool {
        match self {
            Self::Static(s) => s.is_empty(),
            Self::Cmd { last_run, .. } => last_run[monitor].is_empty(),
            Self::Persistent { last_run, .. } => last_run[monitor].is_empty(),
        }
    }

    fn replicate_to_mon(mut self, n_monitor: usize) -> Self {
        match &mut self {
            Self::Cmd { last_run, .. } => {
                while last_run.len() < n_monitor {
                    last_run.push(String::new());
                }
            }
            Self::Persistent { last_run, .. } => {
                while last_run.len() < n_monitor {
                    last_run.push(String::new());
                }
            }
            _ => (),
        }
        self
    }

    fn take(&mut self, mon: usize) -> Option<String> {
        let f = |s: &mut String| {
            if !s.is_empty() {
                Some(mem::replace(s, String::new()))
            } else {
                None
            }
        };
        match self {
            Self::Static(s) => Some(s.clone()),
            Self::Cmd { last_run, .. } => f(&mut last_run[mon]),
            Self::Persistent { last_run, .. } => f(&mut last_run[mon]),
        }
    }
}

pub struct Block {
    bg: Option<Color>,
    fg: Option<Color>,
    un: Option<Color>,
    font: Option<Font>,
    offset: Option<f64>,
    actions: [Option<String>; 5],
    content: Content,
    interval: Duration,
    timer: RwLock<Duration>,
    alignment: Alignment,
    raw: bool,
    signal: bool,
}

impl Block {
    pub fn to_text(&mut self, monitor: usize) -> Option<Text> {
        self.content.take(monitor).map(|text| {
            let mut attr = Attributes::default();
            if let Some(fg) = self.fg {
                attr = attr.with_fg_color(fg);
            }
            if let Some(bg) = self.bg {
                attr = attr.with_bg_color(bg);
            }
            if let Some(font) = &self.font {
                attr = attr.with_font(font.clone());
            }
            if let Some(offset) = self.offset {
                match self.alignment {
                    Alignment::Left => attr = attr.with_padding(Padding::left(offset)),
                    Alignment::Right => attr = attr.with_padding(Padding::left(offset)),
                    Alignment::Middle => (),
                }
            }
            Text { attr, text }
        })
    }

    pub fn update(&mut self) {
        self.content.update();
    }

    fn parse(block: &str, n_monitor: usize) -> Result<Self, ParseError> {
        use BlockBuilder as BB;
        let mut block_b = BB::default();
        for opt in block.split('\n').skip(1).filter(|s| !s.trim().is_empty()) {
            let (key, value) = opt.split_at(opt.find(':').ok_or((opt, "missing :"))?);
            let value = value[1..].trim().trim_end_matches('\'');
            let color = || Color::from_str(value).map_err(|e| (opt, e.to_string()));
            block_b = match key
                .trim()
                .trim_start_matches('*')
                .trim_start_matches('-')
                .trim()
            {
                "background" | "bg" => BB {
                    bg: Some(color()?),
                    ..block_b
                },
                "foreground" | "fg" => BB {
                    fg: Some(color()?),
                    ..block_b
                },
                "underline" | "un" => BB {
                    un: Some(color()?),
                    ..block_b
                },
                "font" => BB {
                    font: Some(value.into()),
                    ..block_b
                },
                "offset" => BB {
                    offset: Some(value.parse().map_err(|_| (opt, "Invalid offset"))?),
                    ..block_b
                },
                "left-click" => block_b.action(0, value),
                "middle-click" => block_b.action(1, value),
                "right-click" => block_b.action(2, value),
                "scroll-up" => block_b.action(3, value),
                "scroll-down" => block_b.action(4, value),
                "interval" => BB {
                    interval: Some(Duration::from_secs(
                        value
                            .parse::<u64>()
                            .map_err(|_| (opt, "Invalid duration"))?,
                    )),
                    ..block_b
                },
                "command" | "cmd" => block_b.content_command(value),
                "static" => block_b.content_static(value),
                "persistent" => block_b.content_persistent(value),
                "alignment" | "align" => BB {
                    alignment: Some(value.parse().map_err(|e| (opt, e))?),
                    ..block_b
                },
                "signal" => BB {
                    signal: value.parse().map_err(|_| (opt, "Invalid boolean"))?,
                    ..block_b
                },
                "raw" => BB {
                    raw: value.parse().map_err(|_| (opt, "Invalid boolean"))?,
                    ..block_b
                },
                "multi_monitor" => BB {
                    multi_monitor: value.parse().map_err(|_| (opt, "Invalid boolean"))?,
                    ..block_b
                },
                s => {
                    eprintln!("Warning: unrecognised option '{}', skipping", s);
                    block_b
                }
            };
        }
        block_b
            .build(n_monitor)
            .map_err(|e| ("BLOCK DEFINITION", e).into())
    }
}

#[derive(Default)]
pub struct BlockBuilder {
    bg: Option<Color>,
    fg: Option<Color>,
    un: Option<Color>,
    font: Option<Font>,
    offset: Option<f64>,
    actions: [Option<String>; 5],
    content: Option<Content>,
    interval: Option<Duration>,
    alignment: Option<Alignment>,
    raw: bool,
    signal: bool,
    multi_monitor: bool,
}

impl BlockBuilder {
    fn action(mut self, index: usize, action: &str) -> Self {
        self.actions[index] = Some(action.into());
        self
    }

    fn content_command(self, c: &str) -> Self {
        Self {
            content: Some(Content::Cmd {
                cmd: c.to_string(),
                last_run: Default::default(),
            }),
            ..self
        }
    }

    fn content_static(self, c: &str) -> Self {
        Self {
            content: Some(Content::Static(c.to_string())),
            ..self
        }
    }

    fn content_persistent(self, c: &str) -> Self {
        Self {
            content: Some(Content::Persistent {
                cmd: c.to_string(),
                last_run: Default::default(),
            }),
            ..self
        }
    }

    fn build(self, n_monitor: usize) -> Result<Block, &'static str> {
        let n_monitor = if self.multi_monitor { n_monitor } else { 1 };
        if let Some(content) = self.content {
            if let Some(alignment) = self.alignment {
                Ok(Block {
                    bg: self.bg,
                    fg: self.fg,
                    un: self.un,
                    font: self.font,
                    offset: self.offset,
                    content: content.replicate_to_mon(n_monitor),
                    interval: self.interval.unwrap_or_else(|| Duration::from_secs(10)),
                    actions: self.actions,
                    alignment,
                    timer: Default::default(),
                    raw: self.raw,
                    signal: self.signal,
                })
            } else {
                Err("No alignment defined")
            }
        } else {
            Err("No content defined")
        }
    }
}

#[derive(Default)]
pub struct GlobalConfig {
    base_geometry: Option<String>,
    bars_geometries: Vec<String>,
    bottom: bool,
    font: Option<Font>,
    n_clickbles: Option<u32>,
    name: Option<String>,
    underline_width: Option<u32>,
    background: Option<Color>,
    foreground: Option<Color>,
    underline: Option<Color>,
    separator: Option<String>,
    tray: bool,
}

impl<'a> TryFrom<&'a str> for GlobalConfig {
    type Error = ParseError<'a>;
    fn try_from(globals: &'a str) -> Result<Self, Self::Error> {
        let mut global_config = Self::default();
        for opt in globals.split('\n').filter(|s| !s.trim().is_empty()) {
            let (key, value) = opt.split_at(opt.find(':').ok_or((opt, "missing :"))?);
            let value = value[1..].trim_matches('\'');
            let color = || Color::from_str(value).map_err(|e| (opt, e.to_string()));
            match key
                .trim()
                .trim_start_matches('*')
                .trim_start_matches('-')
                .trim()
            {
                "background" | "bg" | "B" => global_config.background = Some(color()?),
                "foreground" | "fg" | "F" => global_config.foreground = Some(color()?),
                "underline" | "un" | "U" => global_config.underline = Some(color()?),
                "font" | "f" => global_config.font = Some(value.into()),
                "bottom" | "b" => {
                    global_config.bottom = value
                        .trim()
                        .parse()
                        .map_err(|_| (opt, "Not a valid boolean"))?
                }
                "n_clickables" | "a" => {
                    global_config.n_clickbles = Some(
                        value
                            .trim()
                            .parse()
                            .map_err(|_| (opt, "Not a valid number"))?,
                    )
                }
                "underline_width" | "u" => {
                    global_config.underline_width = Some(
                        value
                            .trim()
                            .parse()
                            .map_err(|_| (opt, "Not a valid number"))?,
                    )
                }
                "separator" => global_config.separator = Some(value.into()),
                "geometry" | "g" => global_config.base_geometry = Some(value.into()),
                "name" | "n" => global_config.name = Some(value.into()),
                s => {
                    eprintln!("Warning: unrecognised option '{}', skipping", s);
                }
            }
        }
        Ok(global_config)
    }
}
