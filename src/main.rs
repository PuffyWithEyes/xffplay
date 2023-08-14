extern crate x11rb;
mod intergration_test_util;

use x11rb::{connection::Connection, protocol::Event};
use x11rb::errors::{ReplyOrIdError, ConnectionError};
use x11rb::protocol::xproto::*;
use std::{
	thread,
	time::Duration,
	process::{self, Command, Child},
};

use intergration_test_util::util;


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


fn draw_line<C>(
    conn: &C,
    win_id: Window,
    _: Gcontext,
    white: Gcontext,
    window_size: (u16, u16),
) -> Result<(), ConnectionError>
where C: Connection {
    // Draw the black outlines
	let point1 = Point {
		x: 0,
		y: (window_size.1 / 2) as i16,
	};
	let point2 = Point {
		x: window_size.0 as i16,
		y: (window_size.1 / 2) as i16,
	};
	
	conn.poly_line(CoordMode::ORIGIN, win_id, white, &[point1, point2])?;

    Ok(())
}


fn shape_window<C>(
    conn: &C,
    win_id: Window,
    window_size: (u16, u16),
) -> Result<(), ReplyOrIdError>
where C: Connection {
    // Create a pixmap for the shape
    let pixmap = PixmapWrapper::create_pixmap(conn, 1, win_id, window_size.0, window_size.1)?;

    // Fill the pixmap with what will indicate "transparent"
    let gc = create_gc_with_foreground(conn, pixmap.pixmap(), 0)?;

    let rect = Rectangle {
        x: 0,
        y: 0,
        width: window_size.0,
        height: window_size.1,
    };
    conn.poly_fill_rectangle(pixmap.pixmap(), gc.gcontext(), &[rect])?;

    // Draw the eyes as "not transparent"
    let values = ChangeGCAux::new().foreground(1);
    conn.change_gc(gc.gcontext(), &values)?;
    draw_line(
        conn,
        pixmap.pixmap(),
        gc.gcontext(),
        gc.gcontext(),
        window_size,
    )?;

    // Set the shape of the window
    //conn.shape_mask(shape::SO::SET, shape::SK::BOUNDING, win_id, 0, 0, &pixmap)?;
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
			let white_gc = create_gc_with_foreground(conn, window, screen.white_pixel)?;
			let black_gc = create_gc_with_foreground(conn, window, screen.black_pixel)?;
			
			util::start_timeout_thread(conn1.clone(), window);

			let mut need_repaint = false;
			let mut need_reshape = false;

			let mut window_size = (0_u16, 0_u16);

			let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?;
			let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW")?;
			let (_, wm_delete_window) = (wm_protocols.reply()?.atom, wm_delete_window.reply()?.atom);

			util::start_timeout_thread(conn1.clone(), window);

			loop {
				println!("Size: {} {}", window_size.0, window_size.1);
				
				let event = conn.wait_for_event()?;
				let mut event_option = Some(event);
				while let Some(event) = event_option {
					match event {
						Event::Expose(event) => {
							window_size = (event.width, event.height);
							
							if event.count == 0 {
								need_repaint = true;
							}
						},
						Event::ConfigureNotify(event) => {
							window_size = (event.width, event.height);
							
							need_repaint = true;
						},
						Event::MapNotify(_) => need_reshape = true,
						Event::ClientMessage(event) => {
							let data = event.data.as_data32();
							if event.format == 32 && event.window == window && data[0] == wm_delete_window {
								println!("Window was asked to close");
								
								return Ok(());
							}
						},
						Event::Error(error) => println!("Unknown error {:?}", error),
						event => println!("Unknown event {:?}", event),
					}

					event_option = conn.poll_for_event()?;
				}

				if need_reshape {
					shape_window(conn, window, window_size)?;

					need_reshape = false;
				}

				if need_repaint {
					let pixmap = PixmapWrapper::create_pixmap(
						conn,
						screen.root_depth,
						window,
						window_size.0,
						window_size.1,
					)?;
					draw_line(conn, window, black_gc.gcontext(), white_gc.gcontext(), window_size)?;

					conn.copy_area(
						pixmap.pixmap(),
						window,
						white_gc.gcontext(),
						0,
						0,
						0,
						0,
						window_size.0,
						window_size.1,
					)?;

					conn.flush()?;
					
					need_repaint = false;
				}
		 	}
		}
    }

    Ok(())
}

