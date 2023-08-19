mod draw;

use x11rb::{
	connection::Connection,
	protocol::{Event, xproto::*},
};
use std::{
	fs,
	io,
	thread,
	time::Duration,
	sync::mpsc,
	process::{self, Command, Child},
};
use draw::*;
use msg::ffi::*;


x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
        _NET_WM_NAME,
        UTF8_STRING,
    }
}


const TEXT_PLACE: (i16, i16) = (100_i16, 100_i16);
const TIMEOUT: u64 = 3_u64;
const ERROR: i64 = -1_i64;
const FTOK_PATH: &str = "/etc/xffplay/token.txt";
const DEV_PATH: &str = "/dev/";


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


fn get_video_stream() -> Result<String, Box<dyn std::error::Error>> {
	let dev = fs::read_dir(DEV_PATH)?;
	let mut video_streams: Vec<String> = Vec::new();

	for file in dev {
		if file.as_ref().unwrap().file_name().to_str().unwrap().to_string().contains("video") {
			video_streams.push(file?.file_name().to_str().unwrap().to_string());
		}
	}

	println!("Какой видеопоток Вы хотите использовать? (Обычно это /dev/video0):");

	let mut counter = 0_usize;

	for stream in &video_streams {
		counter += 1;
		println!("{}. {}{}", counter, DEV_PATH, stream);
	}

	print!(">> ");
	
	let mut stream: String = String::new();

	io::stdin().read_line(&mut stream)?;

	stream = stream.replace("\n", "");
	
	if let Ok(number) = stream.parse::<usize>() {
		let mut full_path = DEV_PATH.to_string();
		full_path.push_str(&video_streams[number - 1]);
		
		return Ok(full_path);
	} else {
		panic!("Выбран неверный поток");
	}
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
	let stream = get_video_stream()?;
	
	let command: Child = Command::new("ffplay")
		.args(&["-video_size", "1920x1080", "-framerate", "10", &stream])
		.stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
		.spawn()
		.expect("Failure to make ffplay process");
	let child_pid = command.id();

	let (sender, reciever) = mpsc::channel::<isize>();
	sender.send(0)?;

	thread::spawn(move || {
		let mut msg = MsgBuf {
			mtype: 0,
			mtext: [0; MSG_BUFF],
		};
		
		let key = unsafe { ftok(FTOK_PATH.as_ptr() as *mut i8, 1) };

		if key == ERROR {
			panic!("Problems with ftok");
		}

		let msgid = unsafe { msgget(key as i32, 0666 | 01000) };

		if msgid as i64 == ERROR {
			panic!("Problems with msgget");
		}

		let mut counter = 0_usize;
		
		loop {
			unsafe { msgrcv(msgid + 1, &mut msg, MSG_BUFF as u64, 1, 0); };

			let mut u8_vec: Vec<u8> = Vec::new();
			
			while counter < 3 {
				if msg.mtext[counter] == 0 {
					counter += 1;
					
					continue;
				}
				
				u8_vec.push(msg.mtext[counter] as u8);
				
				counter += 1;
			}

			counter = 0;

			let message = String::from_utf8_lossy(&u8_vec).to_string();

			if let Ok(number) = message.parse() {
				sender.send(number).unwrap();
			}
		}
	});

	thread::sleep(Duration::from_secs(5));
	
    let (conn, screen_num) = x11rb::connect(None)?;

	let conn1 = std::rc::Rc::new(conn);
    let conn = &*conn1;
	
	let screen = &conn.setup().roots[screen_num];

	let atoms = Atoms::new(conn)?.reply()?;

	let win_aux = ChangeWindowAttributesAux::new()
		.event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::NO_EVENT);

	let (mut width, mut height) = (1920_u16, 1080_u16);
	
    for window in find_windows(conn, screen.root)? {
        let pid = get_window_pid(conn, window)?;

		if child_pid == pid {
			conn.change_window_attributes(window, &win_aux)?;

			conn.map_window(window)?;
			
			let mut old_message = reciever.recv()?;

			conn.flush()?;
			
			loop {
				let event = conn.wait_for_event()?;
				
				match event {
					Event::Expose(event) => {
						if event.count == 0 {
							let (width, height): (u16, u16) = (width as _, height as _);

							draw_line(conn, window, (width, height))?;

							draw_text(conn, window, TEXT_PLACE, 5)?;

							conn.flush()?;
						}
					},
					Event::ConfigureNotify(event) => {
						width = event.width;
						height = event.height;
					},
					Event::ClientMessage(event) => {
						let data = event.data.as_data32();

						if event.format == 32 && event.window == window && data[0] == atoms.WM_DELETE_WINDOW {
							println!("Окно было закрыто по вашему запросу!");

							return Ok(());
						}
					},
					Event::Error(err) => println!("Произошло неизвестное событие с окном: {:?}", err),
					event => println!("Получено неизвестное событие с окном: {:?}", event),
				}
				
				thread::sleep(Duration::from_millis(TIMEOUT));
		 	}
		}
    }

    Ok(())
}

