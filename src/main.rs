extern crate x11rb;

use x11rb::connection::{Connection, RequestConnection};
use x11rb::errors::{ReplyOrIdError, ConnectionError};
use x11rb::protocol::xproto::*;
use std::{
	thread,
	time::Duration,
	process::{self, Command, Child},
};


const RED_COLOR: u32 = 0xFF0000;
const GREEN_COLOR: u32 = 0x00FF00;
// const BLUE_COLOR: u32 = 0x0000FF;
// const YELLOW_COLOR: u32 = 0xFFFF00;
const DEFAULT_SIZE: (u16, u16) = (1920_u16, 1080_u16);
//const ERROR: isize = -1_isize;
const TIMEOUT: u64 = 3_u64;
const FONT: &str = "Cantarell:size=32";


fn find_windows<C>(
    conn: &C,
    root: Window,
) -> Result<Vec<Window>, Box<dyn std::error::Error>>
where C: Connection {
    let client_list = conn.intern_atom(false, b"_NET_CLIENT_LIST")?.reply()?.atom;
    let reply = conn
        .get_property(false, root, client_list, AtomEnum::WINDOW, 0, !0)?
        .reply()?;
    let windows = reply.value32().ok_or("Wrong property type")?.collect();
    Ok(windows)
}


fn get_window_pid<C>(
    conn: &C,
    window: Window,
) -> Result<u32, Box<dyn std::error::Error>>
where C: Connection {
    let wm_pid = conn.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
    let reply = conn
        .get_property(false, window, wm_pid, AtomEnum::CARDINAL, 0, 1)?
        .reply()?;
    let pid = reply
        .value32()
        .and_then(|mut iter| iter.next())
        .ok_or("Failed to get pid")?;
    Ok(pid)
}


fn create_gc_with_foreground<C>(
	conn: &C,
	win_id: Window,
	foreground: u32,
) -> Result<GcontextWrapper<'_, C>, ReplyOrIdError>
where C: Connection {
	GcontextWrapper::create_gc(
        conn,
        win_id,
        &CreateGCAux::new()
            .graphics_exposures(0)
            .foreground(foreground),
    )
}


fn create_gc_with_foreground_font<C>(
	conn: &C,
	win_id: Window,
	foreground: u32,
	font: Font,
) -> Result<GcontextWrapper<'_, C>, ReplyOrIdError>
where C: Connection {
	GcontextWrapper::create_gc(
		conn,
		win_id,
		&CreateGCAux::new()
			.graphics_exposures(0)
			.foreground(foreground)
			.font(font),
	)
}


fn draw_line<C>(
    conn: &C,
    win_id: Window,
    gc: Gcontext,
    window_size: (u16, u16),
) -> Result<(), ConnectionError>
where C: Connection {
	let point1 = Point {
		x: 0,
		y: (window_size.1 / 2) as i16,
	};
	let point2 = Point {
		x: window_size.0 as i16,
		y: (window_size.1 / 2) as i16,
	};
	
	conn.poly_line(CoordMode::ORIGIN, win_id, gc, &[point1, point2])?;

    Ok(())
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
	let command: Child = Command::new("ffplay")
		.args(&["-video_size", "1920x1080", "-framerate", "30", "/dev/video0"])
		.stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
		.spawn()
		.expect("Failure to make ffplay process");
	let child_pid = command.id();

	thread::sleep(Duration::from_secs(5));
	
    let (conn, screen_num) = x11rb::connect(None)?;

	let conn1 = std::sync::Arc::new(conn);
    let conn = &*conn1;

	let screen = &conn.setup().roots[screen_num];
	
    for window in find_windows(conn, screen.root)? {
        let pid = get_window_pid(conn, window)?;

		if child_pid == pid {
			let font = conn.generate_id()?;
			conn.open_font(font, b"7x15")?;
			
			let red_gc = create_gc_with_foreground(conn, window, RED_COLOR)?;
			//let green_gc = create_gc_with_foreground(conn, window, GREEN_COLOR)?;
			let green_font_gc = create_gc_with_foreground_font(conn, window, GREEN_COLOR, font)?;

			conn.close_font(font)?;
			
			loop {
				draw_line(conn, window, red_gc.gcontext(), DEFAULT_SIZE)?;

				conn.image_text8(window, green_font_gc.gcontext(), 100, 100, b"Hello World!")?;
				
				conn.flush()?;

				thread::sleep(Duration::from_millis(TIMEOUT));
		 	}
		}
    }

    Ok(())
}

