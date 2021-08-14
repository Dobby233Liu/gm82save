#![allow(dead_code)]

use crate::{
    delphi::{DynArraySetLength, TBitmap, TMemoryStream, UStr},
    delphi_call,
};
use std::slice;

#[repr(C)]
pub struct Trigger {
    vmt: u32,
    pub name: UStr,
    pub condition: UStr,
    pub constant_name: UStr,
    pub kind: u32,
}

impl Trigger {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x62d200, 0x62cf48, 1)
    }
}

#[repr(C)]
pub struct Sound {
    vmt: u32,
    pub kind: u32,
    pub extension: UStr,
    pub effects: u32,
    pub source: UStr,
    pub padding: u32, // align f64 to 8 bytes
    pub volume: f64,
    pub pan: f64,
    pub preload: bool,
    pub data: *const TMemoryStream,
}

impl Sound {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x64fb70, 0x64f674, 1)
    }
}

unsafe impl Sync for Sound {}

#[repr(C)]
pub struct Frame {
    vmt: u32,
    pub width: u32,
    pub height: u32,
    pub data: *const u8,
}

impl Frame {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x701bf8, 0x700d94, 1)
    }

    pub unsafe fn load_from_file(&mut self, file: &UStr) {
        let _: u32 = delphi_call!(0x7026A4, self, file);
    }

    pub unsafe fn thumb(&self, out: &mut [u8]) {
        // note: this assumes output format == input format
        // these are stored as BGRA8 so make sure to double check what the output should be
        use itertools::Itertools;
        let data = slice::from_raw_parts(self.data, (self.width * self.height * 4) as usize);
        let (width, height) = if self.width > self.height {
            (16, (self.height * 16 / self.width) as usize)
        } else {
            ((self.width * 16 / self.height) as usize, 16)
        };
        let (hoffset, voffset) = (8 - width / 2, 8 - height / 2);
        for (y, row) in out.chunks_exact_mut(16 * 4).enumerate() {
            if y < voffset || y >= voffset + height {
                row.fill(0);
                continue
            }
            let y = 15 - y; // vertical flip for BMP
            let y = y - voffset;
            for (x, px) in row.chunks_exact_mut(4).enumerate() {
                if x < hoffset || x >= hoffset + width {
                    px.fill(0);
                    continue
                }
                let x = x - hoffset;
                // get sample points
                let ox = (x as f64) / (width as f64) * f64::from(self.width);
                let ox2 = ox + 0.5 / (width as f64) * f64::from(self.width);
                let oy = (y as f64) / (height as f64) * f64::from(self.height);
                let oy2 = oy + 0.5 / (height as f64) * f64::from(self.height);
                // sum all pixels
                let mut px_count = 0.0;
                let sum_px = [ox, ox2]
                    .iter()
                    .map(|x| x.floor() as usize)
                    .cartesian_product([oy, oy2].iter().map(|x| x.floor() as usize))
                    .fold([0.0f64; 4], |mut px, (ox, oy)| {
                        let offset = (oy * self.width as usize + ox) * 4;
                        let in_px = &data[offset..offset + 4];
                        if in_px[3] != 0 {
                            px_count += 1.0;
                            px.iter_mut().zip(in_px).for_each(|(o, i)| *o += f64::from(*i));
                        }
                        px
                    });
                if px_count != 0.0 {
                    // average and place into output
                    px[..3].iter_mut().zip(sum_px).for_each(|(o, i)| *o = (i / px_count).floor() as u8);
                    // blend semi-transparent to white
                    let alpha = sum_px[3] / px_count / 255.0;
                    if alpha != 1.0 {
                        px[..3].iter_mut().for_each(|c| *c += (f64::from(255 - *c) * (1.0 - alpha)) as u8);
                    }
                    px[3] = 255;
                } else {
                    // fully transparent
                    px.fill(0);
                }
            }
        }
    }
}

#[repr(C)]
pub struct Sprite {
    vmt: u32,
    pub frame_count: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub collision_shape: u32,
    pub alpha_tolerance: u32,
    pub per_frame_colliders: bool,
    pub bbox_type: u32,
    pub bbox_left: i32,
    pub bbox_top: i32,
    pub bbox_right: i32,
    pub bbox_bottom: i32,
    pub frames: *mut *mut Frame,
}

impl Sprite {
    pub unsafe fn new() -> *mut Sprite {
        delphi_call!(0x5b325c, 0x5b27dc, 1)
    }

    pub unsafe fn get_icon(&self) -> *const TBitmap {
        delphi_call!(0x5b401c, self)
    }
}

unsafe impl Sync for Sprite {}

#[repr(C)]
pub struct Background {
    vmt: u32,
    pub frame: *mut Frame,
    pub is_tileset: bool,
    pub tile_width: u32,
    pub tile_height: u32,
    pub h_offset: u32,
    pub v_offset: u32,
    pub h_sep: u32,
    pub v_sep: u32,
}

impl Background {
    pub unsafe fn new() -> *mut Background {
        delphi_call!(0x062dba4, 0x62d408, 1)
    }

    pub unsafe fn get_icon(&self) -> *const TBitmap {
        delphi_call!(0x62e5e8, self)
    }
}

unsafe impl Sync for Background {}

#[repr(C)]
pub struct PathPoint {
    pub x: f64,
    pub y: f64,
    pub speed: f64,
}

#[repr(C)]
pub struct Path {
    vmt: u32,
    pub points: *mut PathPoint,
    pub point_count: usize,
    pub connection: u32,
    pub closed: bool,
    pub precision: u32,
    padding: u128,
    pub path_editor_room_background: i32,
    pub snap_x: u32,
    pub snap_y: u32,
}

impl Path {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x5357b0, 0x534924, 1)
    }

    pub unsafe fn alloc_points(&mut self, count: usize) -> &mut [PathPoint] {
        self.point_count = count;
        DynArraySetLength(&mut self.points, 0x534870 as _, 1, count);
        slice::from_raw_parts_mut(self.points, count)
    }

    pub unsafe fn commit(&mut self) {
        let _: u32 = delphi_call!(0x53578c, self);
    }
}

unsafe impl Sync for Path {}

#[repr(C)]
pub struct Script {
    vmt: u32,
    pub source: UStr,
}

impl Script {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x652860, 0x65267c, 1)
    }
}

#[repr(C)]
pub struct Font {
    vmt: u32,
    pub sys_name: UStr,
    pub size: u32,
    pub bold: bool,
    pub italic: bool,
    pub range_start: u32,
    pub range_end: u32,
    pub charset: u32,
    /// This is 1 less than what you'll see saved in .gmk or .exe or .gm81 or whatever
    pub aa_level: u32,
}

impl Font {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x5a8760, 0x5a6628, 1)
    }
}

#[repr(C)]
pub struct Action {
    vmt: u32,
    pub lib_id: u32,
    pub id: u32,
    pub action_kind: u32,
    pub can_be_relative: bool,
    pub is_condition: bool,
    pub applies_to_something: bool,
    pub execution_type: u32,
    pub fn_name: UStr,
    pub fn_code: UStr,
    pub param_count: u32,
    pub param_types: [u32; 8],
    pub applies_to: i32,
    pub is_relative: bool,
    pub param_strings: [UStr; 8],
    pub invert_condition: bool,
}

impl Action {
    pub unsafe fn fill_in(&mut self, lib_id: u32, act_id: u32) {
        let _: u32 = delphi_call!(0x710544, self, lib_id, act_id);
    }
}

#[repr(C)]
pub struct Event {
    vmt: u32,
    pub actions: *const *const Action,
    pub action_count: u32,
}

impl Event {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x5a5048, 0x5a4c6c, 1)
    }

    pub unsafe fn add_action(&mut self, libid: u32, actid: u32) -> *mut Action {
        delphi_call!(0x5a51d4, self, libid, actid)
    }
}

#[repr(C)]
pub struct Timeline {
    vmt: u32,
    pub moment_events: *mut *mut Event,
    pub moment_times: *mut u32,
    pub moment_count: usize,
}

impl Timeline {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x5adf3c, 0x5ad98c, 1)
    }

    pub unsafe fn alloc(&mut self, count: usize) {
        self.moment_count = count;
        DynArraySetLength(&mut self.moment_times, 0x5ad900 as _, 1, count);
        DynArraySetLength(&mut self.moment_events, 0x5ad8c8 as _, 1, count);
    }
}

unsafe impl Sync for Timeline {}

#[repr(C)]
pub struct Object {
    vmt: u32,
    pub sprite_index: i32,
    pub solid: bool,
    pub visible: bool,
    pub depth: i32,
    pub persistent: bool,
    pub parent_index: i32,
    pub mask_index: i32,
    pub events: [*mut *mut Event; 12],
}

impl Object {
    pub unsafe fn new() -> *mut Object {
        delphi_call!(0x7049a8, 0x704428, 1)
    }

    pub unsafe fn get_event(&mut self, ev_type: usize, ev_numb: usize) -> *mut Event {
        delphi_call!(0x704d74, self, ev_type, ev_numb)
    }
}

unsafe impl Sync for Object {}

#[repr(C)]
pub struct RoomBackground {
    pub visible_on_start: bool,
    pub is_foreground: bool,
    pub source_bg: i32,
    pub xoffset: i32,
    pub yoffset: i32,
    pub tile_horz: bool,
    pub tile_vert: bool,
    pub hspeed: i32,
    pub vspeed: i32,
    pub stretch: bool,
}

#[repr(C)]
pub struct View {
    pub visible: bool,
    pub source_x: i32,
    pub source_y: i32,
    pub source_w: u32,
    pub source_h: u32,
    pub port_x: i32,
    pub port_y: i32,
    pub port_w: u32,
    pub port_h: u32,
    pub following_hborder: i32,
    pub following_vborder: i32,
    pub following_hspeed: i32,
    pub following_vspeed: i32,
    pub following_target: i32,
}

#[repr(C)]
pub struct Instance {
    pub x: i32,
    pub y: i32,
    pub object: i32,
    pub id: u32,
    pub creation_code: UStr,
    pub locked: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub source_bg: i32,
    pub u: i32,
    pub v: i32,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub id: u32,
    pub locked: bool,
}

#[repr(C)]
pub struct Room {
    vmt: u32,
    pub caption: UStr,
    pub speed: u32,
    pub width: u32,
    pub height: u32,
    pub snap_x: u32,
    pub snap_y: u32,
    pub isometric: bool,
    pub persistent: bool,
    pub bg_colour: i32,
    pub clear_screen: bool,
    pub backgrounds: [RoomBackground; 8],
    pub views_enabled: bool,
    pub clear_view: bool,
    pub views: [View; 8],
    pub creation_code: UStr,
    pub instance_count: usize,
    pub instances: *mut Instance,
    pub tile_count: usize,
    pub tiles: *mut Tile,
    pub remember_room_editor_info: bool,
    pub editor_width: u32,
    pub editor_height: u32,
    pub show_grid: bool,
    pub show_objects: bool,
    pub show_tiles: bool,
    pub show_backgrounds: bool,
    pub show_foregrounds: bool,
    pub show_views: bool,
    pub delete_underlying_objects: bool,
    pub delete_underlying_tiles: bool,
    pub tab: u32,
    pub x_position_scroll: u32,
    pub y_position_scroll: u32,
}

impl Room {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x6577b8, 0x6564cc, 1)
    }

    pub unsafe fn alloc_instances(&mut self, count: usize) -> &mut [Instance] {
        self.instance_count = count;
        DynArraySetLength(&mut self.instances, 0x656418 as _, 1, count);
        slice::from_raw_parts_mut(self.instances, count)
    }

    pub unsafe fn get_instances(&self) -> &[Instance] {
        slice::from_raw_parts(self.instances, self.instance_count)
    }

    pub unsafe fn put_tiles(&mut self, tiles: Vec<Tile>) {
        self.tile_count = tiles.len();
        DynArraySetLength(&mut self.tiles, 0x656448 as _, 1, tiles.len());
        tiles.as_ptr().copy_to_nonoverlapping(self.tiles, tiles.len());
    }

    pub unsafe fn get_tiles(&self) -> &[Tile] {
        slice::from_raw_parts(self.tiles, self.tile_count)
    }

    pub unsafe fn calc_extents(&mut self) {
        let _: u32 = delphi_call!(0x657b48, self);
    }
}

unsafe impl Sync for Room {}

#[repr(C)]
pub struct IncludedFile {
    vmt: u32,
    pub file_name: UStr,
    pub source_path: UStr,
    pub data_exists: bool,
    pub source_length: u32,
    pub stored_in_gmk: bool,
    pub data: *const TMemoryStream,
    pub export_setting: u32,
    pub export_custom_folder: UStr,
    pub overwrite_file: bool,
    pub free_memory: bool,
    pub remove_at_end: bool,
}

impl IncludedFile {
    pub unsafe fn new() -> *mut Self {
        delphi_call!(0x6ca800, 0x6ca360, 1)
    }
}

#[repr(C)]
pub struct Extension {
    vmt: u32,
    pub name: UStr,
    // other stuff that doesn't get written to the gmk
}

#[repr(C)]
pub struct ActionDefinition {
    vmt: u32,
    name: UStr,
    pub id: u32,
    image: u32,      // pointer
    image_list: u32, // also pointer
    image_index: u32,
    hidden: bool,
    advanced: bool,
    pro_only: bool,
    short_desc: UStr,
    list_text: UStr,
    hint_text: UStr,
    kind: u32,
    interface: u32,
    question: bool,
    apply_to: bool,
    relative: bool,
    arg_count: u32,
    arg_captions: [UStr; 8],
    arg_types: [u32; 8],
    arg_defaults: [UStr; 8],
    arg_menu_lens: [UStr; 8],
    execution_type: u32,
    function_name: UStr,
    code_string: UStr,
}

#[repr(C)]
pub struct ActionLibrary {
    vmt: u32,
    caption: UStr,
    pub id: u32,
    author: UStr,
    version: u32,
    padding: u32,
    last_changed: f64,
    information: UStr,
    pub init_code: UStr,
    advanced: bool,
    pub action_count: usize,
    pub actions: *const &'static ActionDefinition,
    max_id: u32,
    // also an image list but who cares
}

pub type Form = u32; // pointer but eh
