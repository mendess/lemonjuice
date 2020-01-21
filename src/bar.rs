use crate::block::{Alignment, Config, GlobalConfig};
use crate::text::ComputedText;
use xcb::base::Connection;
use xcb_util::ewmh;

pub struct Bar {
    conn: ewmh::Connection,
    window_id: u32,
    screen_idx: usize,
    surface: cairo::XCBSurface,
    width: u16,
    height: u16,
    pub contents: Config,
    global_config: GlobalConfig,
    contents_cache: Vec<ComputedText>,
}

impl Bar {
    pub fn new(
        global_config: GlobalConfig,
        config: Config,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, screen_idx) = Connection::connect(None)?;
        let screen = conn
            .get_setup()
            .roots()
            .nth(screen_idx as usize)
            .ok_or("invalid screen_idx")?;
        let width = screen.width_in_pixels();
        let height = 22;
        let id = conn.generate_id();
        let values = [
            (xcb::CW_BACK_PIXEL, screen.black_pixel()),
            (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
        ];
        xcb::create_window(
            &conn,
            xcb::COPY_FROM_PARENT as u8,
            id,
            screen.root(),
            0,
            0,
            width,
            height,
            0,
            xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
            screen.root_visual(),
            &values,
        );
        let surface =
            cairo_surface_for_xcb_window(&conn, &screen, id, i32::from(width), i32::from(height))
                .map_err(|e| format!("Couldn't create cairo surface: {}", e))?;
        let ewmh_conn = ewmh::Connection::connect(conn).map_err(|(e, _)| e)?;
        ewmh::set_wm_window_type(&ewmh_conn, id, &[ewmh_conn.WM_WINDOW_TYPE_DOCK()]);
        let strut_partial = ewmh::StrutPartial {
            left: 0,
            right: 0,
            top: u32::from(height),
            bottom: 0,
            left_start_y: 0,
            left_end_y: 0,
            right_start_y: 0,
            right_end_y: 0,
            top_start_x: 0,
            top_end_x: 0,
            bottom_start_x: 0,
            bottom_end_x: 0,
        };
        ewmh::set_wm_strut_partial(&ewmh_conn, id, strut_partial);
        xcb::map_window(&ewmh_conn, id);
        ewmh_conn.flush();
        Ok(Bar {
            conn: ewmh_conn,
            window_id: id,
            screen_idx: screen_idx as usize,
            surface,
            width,
            height,
            contents: config,
            global_config: global_config,
            contents_cache: vec![],
        })
    }

    pub fn render(&self, text: crate::text::Text) {
        text.compute(&self.surface)
            .unwrap()
            .render(&self.surface)
            .unwrap();
        self.conn.flush();
    }

    pub fn render_contents(&mut self, monitor: usize) {
        self.contents_cache.clear();
        if let Some(blocks) = self.contents.get_mut(&Alignment::Right) {
            let surface = &self.surface;
            let contents_cache = &mut self.contents_cache;
            blocks
                .iter_mut()
                .map(|b| b.to_text(monitor))
                .filter_map(|x| x)
                .map(|t| t.compute(surface))
                .try_for_each(|maybe_t| maybe_t.map(|t| contents_cache.push(t)))
                .expect("Failed to render the right side");
        }
        if let Some(blocks) = self.contents.get_mut(&Alignment::Left) {
            let surface = &self.surface;
            let contents_cache = &mut self.contents_cache;
            blocks
                .iter_mut()
                .map(|b| b.to_text(monitor))
                .filter_map(|x| x)
                .map(|t| t.compute(surface))
                .try_for_each(|maybe_t| maybe_t.map(|t| contents_cache.push(t)))
                .expect("Failed to render the left side");
        }
        self.contents_cache
            .iter()
            .try_for_each(|t| t.render(&self.surface))
            .expect("Render failed");
        self.conn.flush();
    }
}

fn get_root_visual_type(conn: &xcb::Connection, screen: &xcb::Screen<'_>) -> xcb::Visualtype {
    for root in conn.get_setup().roots() {
        for allowed_depth in root.allowed_depths() {
            for visual in allowed_depth.visuals() {
                if visual.visual_id() == screen.root_visual() {
                    return visual;
                }
            }
        }
    }
    panic!("No visual type found");
}

/// Creates a `cairo::Surface` for the XCB window with the given `id`.
fn cairo_surface_for_xcb_window(
    conn: &xcb::Connection,
    screen: &xcb::Screen<'_>,
    id: u32,
    width: i32,
    height: i32,
) -> Result<cairo::XCBSurface, cairo::Status> {
    // TODO: Breaks ownership rules
    // conn is passed to cairo as mutable and returned
    let cairo_conn = unsafe {
        cairo::XCBConnection::from_raw_none(conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t)
    };
    let visual = unsafe {
        cairo::XCBVisualType::from_raw_none(
            &mut get_root_visual_type(conn, screen).base as *mut xcb::ffi::xcb_visualtype_t
                as *mut cairo_sys::xcb_visualtype_t,
        )
    };
    let drawable = cairo::XCBDrawable(id);
    cairo::XCBSurface::create(&cairo_conn, &drawable, &visual, width, height)
}
