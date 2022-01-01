#![feature(path_try_exists)]
use std::{
    fmt::Write,
    fs::File,
    io::{self, Read},
    path::PathBuf,
    str::FromStr,
};

use anyhow::Context;
use clap::{App, Arg};
use ttf_parser as ttf;
use xmlwriter::*;

const TEST_STRING: &str = "the quick brown fox jumps over the lazy dog";

struct Builder {
    pub buffer: String,
    pub offset: f32,
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(&mut self.buffer, "M {} {} ", x + self.offset, -y).unwrap();
    }

    fn line_to(&mut self, x: f32, y: f32) {
        write!(&mut self.buffer, "L {} {} ", x + self.offset, -y).unwrap();
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        write!(
            &mut self.buffer,
            "Q {} {} {} {} ",
            x1 + self.offset,
            -y1,
            x + self.offset,
            -y
        )
        .unwrap();
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        write!(
            &mut self.buffer,
            "C {} {} {} {} {} {} ",
            x1 + self.offset,
            -y1,
            x2 + self.offset,
            -y2,
            x + self.offset,
            -y
        )
        .unwrap();
    }

    fn close(&mut self) {
        write!(&mut self.buffer, "Z ").unwrap();
    }
}

const RED_ERROR: &str = "\u{001b}[31;1merror: \u{001b}[0m";

fn parse_file(file: &str) -> nom::IResult<&str, (&str, &str)> {
    use nom::{
        bytes::complete::{tag, take_till},
        sequence::terminated,
    };
    let (rest, left) = take_till(|c| c == '{')(file)?;
    let (rest, _) = tag("{}")(rest)?;
    let (rest, right) = terminated(take_till(|c| c == '.'), tag(".svg"))(rest)?;
    Ok((rest, (left, right)))
}

fn format_error<E: std::fmt::Display>(
    message: String,
) -> impl FnOnce(E) -> anyhow::Error {
    move |e: E| anyhow::anyhow!("{}{}\n{}", RED_ERROR, message, e)
}

fn format_message_no_error(message: String) -> anyhow::Error {
    anyhow::anyhow!("{}{}", RED_ERROR, message)
}

fn format_error_no_message<E: std::fmt::Display>(err: E) -> anyhow::Error {
    anyhow::anyhow!("{}{}", RED_ERROR, err)
}

#[derive(Default)]
struct FormatString {
    left: String,
    right: String,
}

impl FormatString {
    pub fn index(&self, index: usize) -> String {
        format!("{}{}{}.svg", self.left, index, self.right)
    }
}

struct Output {
    format: FormatString,
    directory: PathBuf,
}

impl Output {
    pub fn write_file(
        &self,
        index: usize,
        svg: XmlWriter,
    ) -> anyhow::Result<()> {
        use std::io::Write;
        let mut path = self.directory.clone();
        path.push(self.format.index(index));
        let mut file = File::create(&path).map_err(format_error(format!(
            "Could not create {}",
            path.as_os_str().to_str().unwrap()
        )))?;
        let text = svg.end_document().into_bytes();
        file.write_all(&text).map_err(format_error_no_message)?;

        Ok(())
    }
}

struct Args {
    pub face: ttf::Face<'static>,
    pub text: String,
    pub output: Output,
}

fn parse_arguments() -> anyhow::Result<Args> {
    let app = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Turns lines of text into SVG files.")
        .arg(
            Arg::with_name("font")
                .short("f")
                .long("font")
                .value_name("FILE")
                .help("Path to font for conversion.")
                .required(true),
        )
        .arg(
            Arg::with_name("text")
                .short("i")
                .long("input")
                .value_name("FILE")
                .required_unless("text")
                .default_value("./lines.txt")
                .help(
                    "Path to the text file that'll be turned into an SVG. If \
                     specified with -- then it'll use stdin.",
                ),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .required(true)
                .help("Specifies the output folder where the svgs are saved.")
                .default_value("./output")
                .long_help(
                    "Specifies both the output folder and the format of the \
                     output file.
By default how SVGs are saved is using a simple numbering scheme.
EX: 1.svg, 2.svg ...
However you can change how it looks by specifying the output directory
then using {} to specify where the numbers fit in.
For example, if i wanted to have the files look like line_1.svg, line_2.svg
inside of the 'output' folder, then it'll look like this.
ourput/line_{}.svg",
                ),
        );
    let matches = app.get_matches();
    let font: ttf::Face<'static> = {
        let font = matches.value_of("font").unwrap();
        let mut file = File::open(font)
            .map_err(format_error(format!("Could not open {}", font)))?;
        let mut buffer = Box::new(vec![]);
        file.read_to_end(&mut buffer)
            .map_err(format_error_no_message)?;
        // We're leaking here as one and only one font will
        // ever be used within the lifetime of this program.
        // so leaking here is an act of convenience to keep
        // all initilization code in parse_arguments.
        let buffer = buffer.leak();
        ttf::Face::from_slice(buffer, 0)
            .map_err(format_error("Error when parsing font.".to_string()))?
    };

    let text: String = {
        let input = matches.value_of("text").unwrap();
        if input == "--" {
            let mut stdin = io::stdin();
            let mut buffer = vec![];
            stdin
                .read_to_end(&mut buffer)
                .map_err(format_error_no_message)?;
            String::from_utf8(buffer).map_err(format_error(
                "stdin is not formatted with utf8".to_string(),
            ))?
        } else {
            let mut file = File::open(input)
                .map_err(format_error(format!("Could not open {}", input)))?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer)
                .map_err(format_error_no_message)?;
            String::from_utf8(buffer).map_err(format_error(format!(
                "{} is not formatted with utf8",
                input
            )))?
        }
    };

    let output: Output = {
        let output = matches.value_of("output").unwrap();
        let mut path =
            PathBuf::from_str(output).map_err(format_error_no_message)?;

        match path.extension().map(|ext| ext.to_str()) {
            Some(Some("svg")) => {
                let format = {
                    let file = path
                        .file_name()
                        .ok_or_else(|| {
                            format_message_no_error(
                                "Path has no name.".to_string(),
                            )
                        })?
                        .to_str()
                        .unwrap();
                    let (_, (left, right)) =
                        parse_file(file).map_err(|_| {
                            format_error_no_message(
                                "output not formatted correctly.",
                            )
                        })?;
                    FormatString {
                        left: left.to_string(),
                        right: right.to_string(),
                    }
                };

                path.pop();
                std::fs::create_dir_all(&path).unwrap();
                Output {
                    format,
                    directory: path,
                }
            }
            Some(Some(ext)) => {
                return Err(format_message_no_error(format!(
                    "{} is not a valid output type.",
                    ext
                )))
            }
            Some(None) => {
                return Err(format_message_no_error(
                    "extention is not utf8 formatted.".to_string(),
                ))
            }
            None => {
                std::fs::create_dir_all(&path).unwrap();
                Output {
                    format: FormatString::default(),
                    directory: path,
                }
            }
        }
    };
    Ok(Args {
        face: font,
        text,
        output,
    })
}

fn main() {
    let Args { face, text, output } = match parse_arguments() {
        Ok(args) => args,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };

    for (i, line) in text.lines().enumerate() {
        let chars = line
            .chars()
            .filter_map(|c| face.glyph_index(c))
            .collect::<Vec<_>>();
        let mut w = XmlWriter::new(Options {
            use_single_quote: true,
            ..Default::default()
        });
        w.start_element("svg");
        w.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        let font_data = std::fs::read("./fonts/Roboto-Regular.ttf").unwrap();
        let face = ttf::Face::from_slice(&font_data, 0).unwrap();
        let height = chars
            .iter()
            .filter_map(|id| face.glyph_bounding_box(*id))
            .map(|bounding_box| bounding_box.height())
            .max()
            .unwrap_or_default();
        let width: u16 = chars
            .iter()
            .filter_map(|id| face.glyph_hor_advance(*id))
            .sum();
        w.write_attribute("width", &width);
        w.write_attribute("height", &height);
        let _ = chars.iter().copied().fold(0, |offset, glyph_id| {
            let mut builder = Builder {
                buffer: String::new(),
                offset: offset as f32,
            };
            let advance = face.glyph_hor_advance(glyph_id).unwrap_or_default();
            if face.outline_glyph(glyph_id, &mut builder).is_some() {
                let path: &str = &builder.buffer;
                w.start_element("path");
                w.write_attribute("d", path);
                w.end_element();
            }
            offset + advance
        });
        w.end_element();
        if let Err(e) = output
            .write_file(i, w)
            .map_err(format_error(format!("could not write file {}", i)))
        {
            println!("{}", e);
            std::process::exit(1);
        }
    }
}
