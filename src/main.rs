mod config;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "tilecap", about = "Minimal screenshot tool for tiling window managers")]
struct Args {
    #[arg(short = 'm', long, default_value = "full", value_enum)]
    mode: Mode,

    #[arg(short = 'o', long, default_value = "file", value_enum)]
    output: Output,

    #[arg(short = 'd', long)]
    dir: Option<String>,

    #[arg(short = 'n', long)]
    name: Option<String>,

    #[arg(short = 'r', long)]
    geometry: Option<String>,

    #[arg(short = 'c', long)]
    config: Option<String>,
}

#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
enum Mode {
    Full,
    Window,
    Region,
}

#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
enum Output {
    File,
    Clipboard,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub w: u16,
    pub h: u16,
}

pub fn parse_geometry(s: &str) -> Result<Rect, String> {
    let (wh, xy) = s.split_once('+').ok_or("bad geometry, expected WxH+X+Y")?;
    let (w, h) = wh.split_once('x').ok_or("bad geometry, expected WxH+X+Y")?;
    let (x_str, y_str) = xy.split_once('+').ok_or("bad geometry, expected WxH+X+Y")?;
    Ok(Rect {
        w: w.parse().map_err(|_| "bad width")?,
        h: h.parse().map_err(|_| "bad height")?,
        x: x_str.parse().map_err(|_| "bad x")?,
        y: y_str.parse().map_err(|_| "bad y")?,
    })
}

fn main() {
    let args: Args = Args::parse();

    let cfg = config::load_config(args.config.as_deref());
    let out_dir = args.dir.as_deref().map(std::path::Path::new).unwrap_or(&cfg.output_dir);

    let geometry = args.geometry.as_deref().map(parse_geometry).transpose().unwrap_or_else(|e| {
        eprintln!("tilecap: {e}");
        std::process::exit(1);
    });

    let img = match capture_screen(&args.mode, geometry) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("tilecap: {e}");
            std::process::exit(1);
        }
    };

    match args.output {
        Output::File => {
            let stem = args.name.unwrap_or_else(timestamp_stem);
            let path = out_dir.join(format!("{stem}.png"));
            if let Err(e) = save_png(&path, &img) {
                eprintln!("tilecap: save failed: {e}");
                std::process::exit(1);
            }
            println!("{}", path.display());
        }
        Output::Clipboard => {
            let png_data = match encode_png(&img) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("tilecap: encode failed: {e}");
                    std::process::exit(1);
                }
            };
            if let Err(e) = set_clipboard(&png_data) {
                eprintln!("tilecap: clipboard failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn capture_screen(mode: &Mode, geometry: Option<Rect>) -> Result<ImageData, Box<dyn std::error::Error>> {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

    if is_wayland {
        #[cfg(feature = "wayland")]
        {
            if *mode != Mode::Full {
                return Err("interactive modes not yet supported on Wayland".into());
            }
            if geometry.is_some() {
                return Err("geometry flag not yet supported on Wayland".into());
            }
            return wayland::capture_full();
        }
        #[cfg(not(feature = "wayland"))]
        {
            let _ = mode;
            let _ = geometry;
            return Err("tilecap was compiled without Wayland support".into());
        }
    } else if std::env::var("DISPLAY").is_ok() {
        #[cfg(feature = "x11")]
        {
            let (conn, screen) = x11::open_connection()?;
            return match mode {
                Mode::Full => x11::capture_full(&conn, &screen),
                Mode::Window => x11::capture_window(&conn, &screen),
                Mode::Region => {
                    if let Some(rect) = geometry {
                        x11::capture_rect(&conn, &screen, rect.x, rect.y, rect.w, rect.h)
                    } else {
                        x11::capture_region_interactive(&conn, &screen)
                    }
                }
            };
        }
        #[cfg(not(feature = "x11"))]
        {
            let _ = mode;
            let _ = geometry;
            return Err("tilecap was compiled without X11 support".into());
        }
    } else {
        Err("no display server detected (set DISPLAY or WAYLAND_DISPLAY)".into())
    }
}

pub fn set_clipboard(png_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut child = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-target", "image/png"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| "install xclip for clipboard support")?;
    child.stdin.take().unwrap().write_all(png_data)?;
    child.wait()?;
    Ok(())
}

pub fn timestamp_stem() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let (ymd, hms) = {
        let days = secs / 86400;
        let time = secs % 86400;
        let h = time / 3600;
        let s = time % 60;
        let mut y = 1970i64;
        let mut remaining = days as i64;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if remaining < days_in_year {
                break;
            }
            remaining -= days_in_year;
            y += 1;
        }
        let month_days = if is_leap(y) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut m = 0usize;
        for (i, &md) in month_days.iter().enumerate() {
            if remaining < md {
                m = i + 1;
                break;
            }
            remaining -= md;
        }
        if m == 0 {
            m = 12;
        }
        let d = remaining + 1;
        (format!("{y}{:02}{:02}", m, d), format!("{h:02}{m:02}{s:02}"))
    };
    format!("Screenshot_{ymd}_{hms}")
}

pub fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[derive(Debug)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn save_png(path: &std::path::Path, img: &ImageData) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, img.width, img.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&img.data)?;
    Ok(())
}

fn encode_png(img: &ImageData) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut data = Vec::new();
    let mut encoder = png::Encoder::new(&mut data, img.width, img.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&img.data)?;
    writer.finish()?;
    Ok(data)
}

#[cfg(feature = "x11")]
mod x11;
#[cfg(feature = "wayland")]
mod wayland;

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_geometry_full() {
        let r = parse_geometry("1920x1080+0+0").unwrap();
        assert_eq!(r, Rect { w: 1920, h: 1080, x: 0, y: 0 });
    }

    #[test]
    fn parse_geometry_offset() {
        let r = parse_geometry("800x600+100+50").unwrap();
        assert_eq!(r, Rect { w: 800, h: 600, x: 100, y: 50 });
    }

    #[test]
    fn parse_geometry_bad_format() {
        assert!(parse_geometry("1920x1080").is_err());
        assert!(parse_geometry("abc").is_err());
        assert!(parse_geometry("+0+0").is_err());
        assert!(parse_geometry("x+0+0").is_err());
    }

    #[test]
    fn parse_geometry_negative_coords() {
        let r = parse_geometry("100x50+-10+-20").unwrap();
        assert_eq!(r, Rect { w: 100, h: 50, x: -10, y: -20 });
    }

    #[test]
    fn is_leap_known() {
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(2024));
        assert!(!is_leap(2023));
        assert!(is_leap(2400));
        assert!(!is_leap(2100));
    }

    #[test]
    fn timestamp_stem_length() {
        let stem = timestamp_stem();
        assert!(stem.starts_with("Screenshot_"));
        assert_eq!(stem.len(), 26);
    }

    #[test]
    fn timestamp_stem_contains_digits() {
        let stem = timestamp_stem();
        let suffix = stem.strip_prefix("Screenshot_").unwrap();
        assert_eq!(suffix.len(), 15);
        assert!(suffix.chars().all(|c| c.is_ascii_digit() || c == '_'));
    }

    #[test]
    fn parse_cli_defaults() {
        let args = Args::try_parse_from(["tilecap"]).unwrap();
        assert!(matches!(args.mode, Mode::Full));
        assert!(matches!(args.output, Output::File));
        assert!(args.dir.is_none());
        assert!(args.name.is_none());
        assert!(args.geometry.is_none());
    }

    #[test]
    fn parse_cli_mode() {
        let a = Args::try_parse_from(["tilecap", "-m", "window"]).unwrap();
        assert!(matches!(a.mode, Mode::Window));
        let a = Args::try_parse_from(["tilecap", "--mode", "region"]).unwrap();
        assert!(matches!(a.mode, Mode::Region));
    }

    #[test]
    fn parse_cli_output() {
        let a = Args::try_parse_from(["tilecap", "-o", "clipboard"]).unwrap();
        assert!(matches!(a.output, Output::Clipboard));
    }

    #[test]
    fn parse_cli_geometry() {
        let a = Args::try_parse_from(["tilecap", "-r", "800x600+100+50"]).unwrap();
        assert_eq!(a.geometry.as_deref(), Some("800x600+100+50"));
    }

    #[test]
    fn parse_cli_dir() {
        let a = Args::try_parse_from(["tilecap", "-d", "/tmp"]).unwrap();
        assert_eq!(a.dir.as_deref(), Some("/tmp"));
    }

    #[test]
    fn parse_cli_name() {
        let a = Args::try_parse_from(["tilecap", "-n", "myshot"]).unwrap();
        assert_eq!(a.name.as_deref(), Some("myshot"));
    }

    #[test]
    fn parse_cli_config() {
        let a = Args::try_parse_from(["tilecap", "-c", "/cfg/tilecap.toml"]).unwrap();
        assert_eq!(a.config.as_deref(), Some("/cfg/tilecap.toml"));
    }

    #[test]
    fn parse_cli_unknown_option_fails() {
        assert!(Args::try_parse_from(["tilecap", "--bogus"]).is_err());
    }

    #[test]
    fn parse_cli_missing_value_fails() {
        assert!(Args::try_parse_from(["tilecap", "-m"]).is_err());
        assert!(Args::try_parse_from(["tilecap", "-o"]).is_err());
    }
}
