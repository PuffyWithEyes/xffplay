use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::errors::ReplyOrIdError;



const RED_COLOR: u32 = 0xFF0000;
const GREEN_COLOR: u32 = 0x00FF00;
const BLUE_COLOR: u32 = 0x0000FF;
const YELLOW_COLOR: u32 = 0xFFFF00;


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


pub fn draw_line<C>(
    conn: &C,
    win_id: Window,
    window_size: (u16, u16),
) -> Result<(), Box<dyn std::error::Error>>
where C: Connection {
	let point1 = Point {
		x: 0,
		y: (window_size.1 / 2) as i16,
	};
	let point2 = Point {
		x: window_size.0 as i16,
		y: (window_size.1 / 2) as i16,
	};

	let red_gc = create_gc_with_foreground(conn, win_id, RED_COLOR)?;
	
	conn.poly_line(CoordMode::ORIGIN, win_id, red_gc.gcontext(), &[point1, point2])?;

    Ok(())
}


pub fn draw_text<C>(
    conn: &C,
    window: Window,
    coord: (i16, i16),
    number: isize,
) -> Result<(), Box<dyn std::error::Error>>
where C: Connection {
    let gc = gc_font_get(conn, window, "6x13".to_string(), number)?;

    conn.image_text8(window, gc, coord.0, coord.1, number.to_string().as_bytes())?;
    conn.free_gc(gc)?;

    Ok(())
}


fn gc_font_get<C>(
    conn: &C,
    window: Window,
    font_name: String,
	number: isize,
) -> Result<Gcontext, ReplyOrIdError>
where C: Connection {
    let font = conn.generate_id()?;

    conn.open_font(font, font_name.as_bytes())?;

    let gc = conn.generate_id()?;

	let mut color = BLUE_COLOR;

	if number > 80 {
		color = RED_COLOR;
	} else if number > 50 {
		color = YELLOW_COLOR;
	} else if number > 35 {
		color = GREEN_COLOR;
	}
	
    let values = CreateGCAux::new()
        .foreground(color)
        .font(font);
    conn.create_gc(gc, window, &values)?;

    conn.close_font(font)?;

    Ok(gc)
}

