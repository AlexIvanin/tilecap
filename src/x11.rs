use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ConnectionExt};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

use crate::ImageData;

pub fn open_connection(
) -> Result<(RustConnection, xproto::Screen), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = conn.setup().roots[screen_num].clone();
    Ok((conn, screen))
}

pub fn capture_full(
    conn: &RustConnection,
    screen: &xproto::Screen,
) -> Result<ImageData, Box<dyn std::error::Error>> {
    get_root_image(
        conn,
        screen.root,
        0,
        0,
        screen.width_in_pixels,
        screen.height_in_pixels,
        screen.root_depth,
    )
}

pub fn capture_window(
    conn: &RustConnection,
    screen: &xproto::Screen,
) -> Result<ImageData, Box<dyn std::error::Error>> {
    let win = pick_window(conn, screen.root)?;
    let geom = conn.get_geometry(win)?.reply()?;
    let root_xy =
        conn.translate_coordinates(win, screen.root, 0, 0)?
            .reply()?;
    get_root_image(
        conn,
        screen.root,
        root_xy.dst_x,
        root_xy.dst_y,
        geom.width,
        geom.height,
        screen.root_depth,
    )
}

pub fn capture_rect(
    conn: &RustConnection,
    screen: &xproto::Screen,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
) -> Result<ImageData, Box<dyn std::error::Error>> {
    get_root_image(conn, screen.root, x, y, w, h, screen.root_depth)
}

pub fn capture_region_interactive(
    conn: &RustConnection,
    screen: &xproto::Screen,
) -> Result<ImageData, Box<dyn std::error::Error>> {
    let rect = select_region(conn, screen)?;
    get_root_image(
        conn,
        screen.root,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        screen.root_depth,
    )
}

/// Fetch a screen region as RGB.
fn get_root_image(
    conn: &RustConnection,
    root: xproto::Window,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    _depth: u8,
) -> Result<ImageData, Box<dyn std::error::Error>> {
    let img = conn
        .get_image(xproto::ImageFormat::Z_PIXMAP, root, x, y, w, h, !0)?
        .reply()?;

    let h = h as usize;
    let w = w as usize;
    let total = img.data.len();
    let bpp = if w > 0 && h > 0 {
        total / (w * h)
    } else {
        4
    };

    let mut rgb = Vec::with_capacity(w * h * 3);
    let row_bytes = w * bpp;
    for row in 0..h {
        let row_start = row * row_bytes;
        for col in 0..w {
            let offset = row_start + col * bpp;
            if offset + 3 <= total {
                let b = img.data[offset];
                let g = img.data[offset + 1];
                let r = img.data[offset + 2];
                rgb.push(r);
                rgb.push(g);
                rgb.push(b);
            }
        }
    }

    Ok(ImageData {
        data: rgb,
        width: w as u32,
        height: h as u32,
    })
}

/// Raw GetImage data (server-native format).
fn get_root_image_raw(
    conn: &RustConnection,
    root: xproto::Window,
    w: u16,
    h: u16,
) -> Result<(Vec<u8>, u8), Box<dyn std::error::Error>> {
    let img = conn
        .get_image(xproto::ImageFormat::Z_PIXMAP, root, 0, 0, w, h, !0)?
        .reply()?;
    Ok((img.data, img.depth))
}

fn pick_window(
    conn: &RustConnection,
    root: xproto::Window,
) -> Result<xproto::Window, Box<dyn std::error::Error>> {
    let cursor = make_crosshair(conn)?;
    conn.grab_pointer(
        false,
        root,
        xproto::EventMask::BUTTON_PRESS,
        xproto::GrabMode::ASYNC,
        xproto::GrabMode::ASYNC,
        root,
        cursor,
        xproto::Time::CURRENT_TIME,
    )?;
    conn.flush()?;

    loop {
        match conn.wait_for_event()? {
            Event::ButtonPress(ev) => {
                conn.ungrab_pointer(xproto::Time::CURRENT_TIME)?;
                conn.flush()?;
                if ev.child == 0 {
                    return Err("no window under pointer".into());
                }
                // Walk parent chain looking for the top-level window
                let win = find_toplevel(conn, ev.child)?;
                return Ok(win);
            }
            _ => {}
        }
    }
}

/// Walk up the parent chain looking for a top-level window.
fn find_toplevel(
    conn: &RustConnection,
    mut win: xproto::Window,
) -> Result<xproto::Window, Box<dyn std::error::Error>> {
    let root = conn.setup().roots[0].root;
    let mut candidate = win;

    loop {
        let has_state = has_wm_state(conn, win);
        let has_type = has_net_wm_window_type(conn, win);

        if has_state || has_type {
            candidate = win;
            break;
        }

        let tree = conn.query_tree(win)?.reply()?;
        if tree.parent == root || tree.parent == tree.root {
            break;
        }
        win = tree.parent;
    }

    Ok(candidate)
}

fn has_wm_state(conn: &RustConnection, win: xproto::Window) -> bool {
    let wm_state = conn
        .intern_atom(false, b"WM_STATE")
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom);

    let atom = match wm_state {
        Some(a) => a,
        None => return false,
    };

    conn.get_property(false, win, atom, atom, 0, 1)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.length > 0)
        .unwrap_or(false)
}

fn has_net_wm_window_type(conn: &RustConnection, win: xproto::Window) -> bool {
    let net_type = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE")
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom);

    let atom = match net_type {
        Some(a) => a,
        None => return false,
    };

    conn.get_property(false, win, atom, xproto::AtomEnum::ATOM, 0, 1024)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.length > 0)
        .unwrap_or(false)
}

fn select_region(
    conn: &RustConnection,
    screen: &xproto::Screen,
) -> Result<crate::Rect, Box<dyn std::error::Error>> {
    let w = screen.width_in_pixels;
    let h = screen.height_in_pixels;

    let (raw_data, raw_depth) = get_root_image_raw(conn, screen.root, w, h)?;


    let overlay = conn.generate_id()?;
    let cursor = make_crosshair(conn)?;
    let win_aux = xproto::CreateWindowAux::new()
        .override_redirect(Some(xproto::Bool32::from(true)))
        .event_mask(
            xproto::EventMask::BUTTON_PRESS
                | xproto::EventMask::BUTTON_RELEASE
                | xproto::EventMask::POINTER_MOTION
                | xproto::EventMask::KEY_PRESS
                | xproto::EventMask::EXPOSURE,
        )
        .cursor(cursor);
    conn.create_window(
        screen.root_depth,
        overlay,
        screen.root,
        0,
        0,
        w,
        h,
        0,
        xproto::WindowClass::COPY_FROM_PARENT,
        0,
        &win_aux,
    )?;
    conn.map_window(overlay)?;

    let gc = conn.generate_id()?;
    conn.create_gc(gc, overlay, &xproto::CreateGCAux::new())?;

    conn.put_image(
        xproto::ImageFormat::Z_PIXMAP,
        overlay,
        gc,
        w,
        h,
        0,
        0,
        0,
        raw_depth,
        &raw_data,
    )?;
    conn.flush()?;

    let escape_kc = lookup_escape_keycode(conn);


    conn.grab_pointer(
        false,
        overlay,
        xproto::EventMask::BUTTON_PRESS
            | xproto::EventMask::BUTTON_RELEASE
            | xproto::EventMask::POINTER_MOTION,
        xproto::GrabMode::ASYNC,
        xproto::GrabMode::ASYNC,
        overlay,
        0u32,
        xproto::Time::CURRENT_TIME,
    )?;
    conn.grab_keyboard(
        false,
        overlay,
        xproto::Time::CURRENT_TIME,
        xproto::GrabMode::ASYNC,
        xproto::GrabMode::ASYNC,
    )?;
    conn.flush()?;

    let mut start: Option<(i16, i16)> = None;
    let mut rubber: Option<crate::Rect> = None;
    let mut rect_cache: Option<crate::Rect> = None;

    loop {
        match conn.wait_for_event()? {
            Event::ButtonPress(ev) => {
                start = Some((ev.event_x, ev.event_y));
                rubber = None;
            }
            Event::MotionNotify(ev) => {
                if let Some(p) = start {
                    if let Some(r) = rubber {
                        draw_rubber(conn, overlay, gc, r)?;
                    }
                    let x = p.0.min(ev.event_x);
                    let y = p.1.min(ev.event_y);
                    let rw = (p.0 - ev.event_x).unsigned_abs() as u16;
                    let rh = (p.1 - ev.event_y).unsigned_abs() as u16;
                    let r = crate::Rect {
                        x,
                        y,
                        w: rw.max(1),
                        h: rh.max(1),
                    };
                    draw_rubber(conn, overlay, gc, r)?;
                    rubber = Some(r);
                    rect_cache = Some(r);
                }
            }
            Event::ButtonRelease(_) => {
                if let Some(r) = rubber {
                    draw_rubber(conn, overlay, gc, r)?;
                }
                conn.flush()?;
                let result = rect_cache.ok_or("no selection")?;
                if result.w < 3 || result.h < 3 {
                    cleanup_grab(conn, overlay, gc);
                    return Err("selection too small".into());
                }
                cleanup_grab(conn, overlay, gc);
                return Ok(result);
            }
            Event::KeyPress(ev) => {
                if Some(ev.detail) == escape_kc {
                    cleanup_grab(conn, overlay, gc);
                    return Err("cancelled".into());
                }
            }
            _ => {}
        }
    }
}

fn lookup_escape_keycode(conn: &RustConnection) -> Option<xproto::Keycode> {
    let setup = conn.setup();
    let first = setup.min_keycode;
    let count = setup.max_keycode - first + 1;
    let reply = conn
        .get_keyboard_mapping(first, count)
        .ok()?
        .reply()
        .ok()?;

    let per = reply.keysyms_per_keycode as usize;
    let escape_keysym: xproto::Keysym = 0xFF1B;

    for (i, kc) in (first..=setup.max_keycode).enumerate() {
        let base = i * per;
        for j in 0..per {
            if base + j < reply.keysyms.len() && reply.keysyms[base + j] == escape_keysym {
                return Some(kc);
            }
        }
    }
    None
}

fn cleanup_grab(conn: &RustConnection, overlay: xproto::Window, gc: xproto::Gcontext) {
    let _ = conn.ungrab_pointer(xproto::Time::CURRENT_TIME);
    let _ = conn.ungrab_keyboard(xproto::Time::CURRENT_TIME);
    let _ = conn.destroy_window(overlay);
    let _ = conn.free_gc(gc);
    let _ = conn.flush();
}

fn make_crosshair(
    conn: &RustConnection,
) -> Result<xproto::Cursor, Box<dyn std::error::Error>> {
    let font = conn.generate_id()?;
    conn.open_font(font, b"cursor")?;
    let cursor = conn.generate_id()?;
    conn.create_glyph_cursor(
        cursor,
        font,
        font,
        68,
        68,
        0xffff,
        0xffff,
        0xffff,
        0x0000,
        0x0000,
        0x0000,
    )?;
    conn.close_font(font)?;
    Ok(cursor)
}

fn draw_rubber(
    conn: &RustConnection,
    overlay: xproto::Window,
    gc: xproto::Gcontext,
    rect: crate::Rect,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.change_gc(
        gc,
        &xproto::ChangeGCAux::new()
            .function(xproto::GX::INVERT)
            .line_width(2)
            .subwindow_mode(xproto::SubwindowMode::INCLUDE_INFERIORS),
    )?;
    conn.poly_rectangle(
        overlay,
        gc,
        &[xproto::Rectangle {
            x: rect.x,
            y: rect.y,
            width: rect.w,
            height: rect.h,
        }],
    )?;
    conn.change_gc(
        gc,
        &xproto::ChangeGCAux::new().function(xproto::GX::COPY),
    )?;
    conn.flush()?;
    Ok(())
}
