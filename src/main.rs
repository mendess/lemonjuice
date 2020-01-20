mod bar;
mod block;
mod text;
use text::{Attributes, Padding, Text};

fn main() {
    let (global_config, config) = match block::parse(
        &std::fs::read_to_string("/home/mendess/.config/lemonbar/lemonrc").unwrap(),
        0,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1)
        }
    };
    let bar = bar::Bar::new(global_config, config).unwrap();
    bar.render(Text {
        attr: Attributes::default()
            .with_padding(Padding::left(1000.0).with_right(5000.0))
            .with_bg_color(Some("#00FF00".parse().unwrap())),
        text: "Test string don't upvote".into(),
    });
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("Done");
    bar.render(Text {
        attr: Default::default(),
        text: "What happened!".into(),
    });
    std::thread::sleep(std::time::Duration::from_secs(10));
}
