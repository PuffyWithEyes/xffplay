mod draw;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use std::{
	thread,
	time::Duration,
	process::{self, Command, Child},
};

use draw::*;


const DEFAULT_SIZE: (u16, u16) = (1920_u16, 1080_u16);
const TIMEOUT: u64 = 3_u64;
const ERROR: i64 = -1_i64;


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


fn main() -> Result<(), Box<dyn std::error::Error>> {
	let command: Child = Command::new("ffplay")
		.args(&["-video_size", "1920x1080", "-framerate", "30", "/dev/video0"])
		.stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
		.spawn()
		.expect("Failure to make ffplay process");
	let child_pid = command.id();

	// let mut message = String::new();
	
	// thread::spawn(move || {
	// 	let mut msg = MsgBuf {
	// 		mtype: 0,
	// 		mtext: [0; MSG_BUFF],
	// 	};
		
	// 	let key = unsafe { ftok("/etc/qtmpv/token.txt".as_ptr() as *mut i8, 1) };

	// 	if key == ERROR {
	// 		panic!("Problems with ftok");
	// 	}

	// 	let msgid = unsafe { msgget(key as i32, 0666 | 01000) };

	// 	if msgid as i64 == ERROR {
	// 		panic!("Problems with msgget");
	// 	}
		
	// 	loop {
	// 		unsafe { msgrcv(msgid + 1, &mut msg, MSG_BUFF as u64, 1, 0); };
			
	// 		println!("New message: {:?}", msg.mtext);
	// 	}
	// });
	
	thread::sleep(Duration::from_secs(5));
	
    let (conn, screen_num) = x11rb::connect(None)?;

	let conn1 = std::sync::Arc::new(conn);
    let conn = &*conn1;

	let screen = &conn.setup().roots[screen_num];
	
    for window in find_windows(conn, screen.root)? {
        let pid = get_window_pid(conn, window)?;

		if child_pid == pid {
			let screen = &conn.setup().roots[screen_num];
			
			loop {
				draw_text(conn, screen, window, 100, 100, "HELLO WORLD!")?;

				draw_line(conn, window, DEFAULT_SIZE)?;

				conn.flush()?;

				thread::sleep(Duration::from_millis(TIMEOUT));
		 	}
		}
    }

    Ok(())
}

