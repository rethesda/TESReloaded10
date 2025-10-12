#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
//#![allow(nonstandard_style)]

use bevy_reflect::{Enum, NamedField, PartialReflect, Reflect, Struct};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use winapi::shared::guiddef::{GUID, IID};
use winapi::shared::minwindef::{BOOL, BYTE, DWORD, FLOAT, INT, UINT};
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::shared::ntdef::{LPCWSTR,LPCSTR, HRESULT};
use winapi::shared::d3d9::LPDIRECT3DDEVICE9;
use winapi::shared::d3d9types::{D3DCOLOR, D3DCOLOR_XRGB};
use winapi::um::wingdi::{FW_NORMAL,DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, ANTIALIASED_QUALITY,FF_DONTCARE};
use winapi::shared::windef::{HDC,LPRECT,RECT};
use winapi::um::winuser::{SetRect,DT_CENTER, DT_LEFT,DT_RIGHT};

use std::any::TypeId;
use std::marker::PhantomData;
use std::ptr;
use std::ffi::CString;
use winapi::RIDL;
use winapi::DEFINE_GUID;

use once_cell::sync::Lazy;
use toml::{Table, Value};
use crate::effect_config::Effects;
use crate::main_config::Config;
use crate::shader_config::Shaders;
use crate::sys_string::SysString;
use crate::{log, CONFIG_TABLE, SHADERS_TABLE, EFFECTS_TABLE, CONFIG, SHADERS, EFFECTS, get_static_ref, static_mut_insert, get_static_ref_const};

#[link(name = "d3dx9")]
unsafe extern "system" {
	pub fn  D3DXCreateFontA(device: LPDIRECT3DDEVICE9, Height : i32, Width : i32, Weight: i32, MipLevels: UINT, Italic: bool, CharSet: DWORD ,OutputPrecision: DWORD, Quality: DWORD, PitchAndFamily: DWORD,pFacename : *const i8, ppfont : *mut *mut  ID3DXFont) -> HRESULT;
}


DEFINE_GUID!(IID_ID3DXFont, 
0xd79dbb70, 0x5f21, 0x4d36, 0xbb, 0xc2, 0xff, 0x52, 0x5c, 0x21, 0x3c, 0xdc);

RIDL!{#[uuid(0xd79dbb70, 0x5f21, 0x4d36, 0xbb, 0xc2, 0xff, 0x52, 0x5c, 0x21, 0x3c, 0xdc)]
interface ID3DXFont(ID3DXFontVtbl): IUnknown(IUnknownVtbl) {
	fn GetDevice(ppDevice : *mut LPDIRECT3DDEVICE9,) -> HRESULT,
    fn GetDescA(pDesc : *mut *mut libc::c_void,) -> HRESULT,
    fn GetDescW(pDesc : *mut *mut libc::c_void,) -> HRESULT,
    fn GetTextMetricsA(pTextMetrics : *mut *mut libc::c_void,) -> BOOL,
    fn GetTextMetricsW(pTextMetrics : *mut *mut libc::c_void,) -> BOOL,
	fn GetDC() -> HDC,
	fn GetGlyphData(/*STUB*/) -> HRESULT, // UINT Glyph, LPDIRECT3DTEXTURE9 *ppTexture, RECT *pBlackBox, POINT *pCellInc
	fn PreloadCharacters(First : UINT, Last : UINT,) -> HRESULT,
	fn PreloadGlyphs(First : UINT, Last : UINT,) -> HRESULT,
	fn PreloadTextA(pString : LPCSTR, Count :INT,) -> HRESULT,
	fn PreloadTextW(pString : LPCWSTR , Count :INT,) -> HRESULT,
	fn DrawTextA(pSprite : *const IUnknown,pString: LPCSTR, Count : INT, pRect : LPRECT, Format : DWORD, Color : D3DCOLOR,) -> HRESULT,
	fn DrawTextW(pSprite :  *const IUnknown,pString: LPCWSTR, Count : INT, pRect : LPRECT, Format : DWORD, Color : D3DCOLOR,) -> HRESULT, //TODO LPD3DXSPRITE
	fn OnLostDevice() -> HRESULT,
	fn OnResetDevice() -> HRESULT,
}}

pub static mut FontRenderer : *mut ID3DXFont = ptr::null_mut();


pub fn CreateFontRender(device: LPDIRECT3DDEVICE9) {
	let mut font : *mut ID3DXFont = ptr::null_mut();
	let font_src = CString::new("Calibri").unwrap();
	let res = unsafe{
		D3DXCreateFontA(device, 22 , 0 , FW_NORMAL , 1 , false , DEFAULT_CHARSET , OUT_DEFAULT_PRECIS , ANTIALIASED_QUALITY , FF_DONTCARE , font_src.as_ptr() , &mut font as *mut *mut ID3DXFont)
	};
	
	log(format!("Create font renderer {}  {:?}", res, font));
	unsafe {
		FontRenderer  = font;
	}	
} 

enum Align{
	Left,Center,Right
}

fn DrawText(renderer : *mut ID3DXFont, rect: &mut RECT, text : *const i8, color : (u32,u32,u32), align : Align){
	let xrgb = D3DCOLOR_XRGB(color.0, color.1, color.2);
	let align = match align {
	    Align::Left => DT_LEFT,
	    Align::Center => DT_CENTER,
	    Align::Right => DT_RIGHT,
	};
	unsafe{
		(renderer.as_ref().unwrap().lpVtbl.as_ref().unwrap().DrawTextA)(renderer, ptr::null(), text, -1, rect, align, xrgb);
	}
}

fn NewRect(left : i32, top : i32, right : i32, bottom : i32) -> RECT {
	let mut rect = RECT {left : 0, top : 0, right: 0, bottom: 0};
	unsafe{
		SetRect(&mut rect, left, top, right, bottom);
	}
	rect
}

fn UpdateRect(rect : &mut RECT,left : i32, top : i32, right : i32, bottom : i32 ){
	unsafe{
		SetRect(rect, left, top, right, bottom);
	}
}


const PositionX : i32 = 60;
const PositionY : i32 = 120;
const TitleColumnSize :i32 = 450;
const TextSize : i32 = 22;
const RowSpace : i32 = 0; //TODO

enum RenderingZone {
	ActiveConfig, ActiveFirst, ActiveSecond, ActiveThird,Version
}

#[derive(PartialEq, Eq)]
pub enum MenuMove {
	Up, Down, Left, Right
}

#[derive(PartialEq, Eq)]
enum MenuSelected {
	Main,Shaders,Effects
}

pub struct MenuState {
	active_config : MenuSelected,
	active_firstnode : String,
	active_secondnode : String,
	active_field : String,
	terminal : bool,
}

impl MenuState {
    pub fn new() -> MenuState {
		MenuState { active_config: MenuSelected::Main , active_firstnode: "".into() , active_secondnode: "".into(), active_field: "".into(), terminal : false }
	} 
	pub fn get_active_config(&self, zone : RenderingZone)-> &str{
		match zone {
			RenderingZone::ActiveConfig => match self.active_config  {
			    MenuSelected::Main => "Main",
				MenuSelected::Shaders => "Shaders",
				MenuSelected::Effects => "Effects"
			}
			RenderingZone::ActiveFirst => &self.active_firstnode,
			RenderingZone::ActiveSecond => &self.active_secondnode,
			RenderingZone::ActiveThird => &self.active_field,
			_ => ""
		}
	}
	
	pub fn get_active_mainconf(&self) -> &MenuSelected{
		&self.active_config
	}
	
	pub fn get_active_table(&self) -> &Table {
		match self.active_config {
			MenuSelected::Main => get_static_ref_const(&raw const CONFIG_TABLE),
			MenuSelected::Shaders => get_static_ref_const(&raw const SHADERS_TABLE),
			MenuSelected::Effects => get_static_ref_const(&raw const EFFECTS_TABLE)
		}
	}
	
	fn move_menu_config(&mut self, mov : MenuMove){
		let repl = if mov == MenuMove::Right {
			match self.active_config {
				MenuSelected::Main => MenuSelected::Shaders,
				MenuSelected::Shaders => MenuSelected::Effects,
				MenuSelected::Effects => MenuSelected::Effects
			}
		}
		else {
			match self.active_config {
				MenuSelected::Main => MenuSelected::Main,
				MenuSelected::Shaders => MenuSelected::Main,
				MenuSelected::Effects => MenuSelected::Shaders
			}
		};
		self.active_config = repl;
	}
	
	fn get_next_state_key<'a>(&'a self, mov : &MenuMove, table : &'a Table, active_node : &str) -> Option<&String>{
		if *mov == MenuMove::Down{
			let mut found = false;
			let mut el : Option<&String> = None;
			for (key,val) in table {
				if found{
					el = Some(key);
				}
				found = active_node == key;
			}
			el
		}
		else if *mov == MenuMove::Up {
			let mut found = false;
			let mut el : Option<&String> = None;
			for (key,val) in table.iter().rev() {
				if found{
					el = Some(key);
				}
				found = active_node == key;
			}
			el
		}
		else {
			None
		}
	}
	pub fn is_terminal(&self) -> bool {
		self.terminal
	}
	
	pub fn move_menu_active_field(&mut self, mov : MenuMove) {
		let term = self.terminal;
		let table = self.get_active_table();
		if self.active_firstnode.is_empty(){
			if mov == MenuMove::Left || mov == MenuMove::Right{
				self.move_menu_config(mov);
			}
			else if mov == MenuMove::Down {
				let item = table.iter().next();
				self.active_firstnode = item.unwrap().0.into();
			}
		}
		else if self.active_secondnode.is_empty(){
			if mov == MenuMove::Up || mov == MenuMove::Down {
				match self.get_next_state_key(&mov, table , &self.active_firstnode ){
					None => {
						if mov == MenuMove::Up {
							self.active_firstnode = "".to_owned();
						}
					},
					Some(el) => {
						self.active_firstnode = el.to_owned();
					}
				}
			}
			else if mov == MenuMove::Right {
				let t = table.get(&self.active_firstnode).unwrap().as_table().unwrap();
				let activ = t.iter().next().unwrap();
				let act_key = activ.0.to_owned();
				let termin = !activ.1.is_table();
				self.active_secondnode = act_key;
				self.terminal = termin;
			}
		}
		else if self.active_field.is_empty(){
			let tab = table.get(&self.active_firstnode).unwrap().as_table().unwrap();
			if mov == MenuMove::Up || mov == MenuMove::Down {
				match self.get_next_state_key(&mov, tab , &self.active_secondnode ){
					None => {},
					Some(el) => {
						let b = !tab.get(&self.active_secondnode).unwrap().is_table();
						self.active_secondnode = el.to_owned();
						self.terminal = b;
					}
				}
			}
			else if mov == MenuMove::Right && !self.terminal{
				let t = tab.get(&self.active_secondnode).unwrap();
				if t.is_table() {
					self.active_field = t.as_table().unwrap().iter().next().unwrap().0.to_owned();
					self.terminal = false;
				}
				else{
					self.terminal = true;
				}
			}
			else if mov == MenuMove::Left{
				self.active_secondnode = "".to_owned();
				self.terminal = false;
			}
		}
		else {
			let tabl = table.get(&self.active_firstnode).unwrap().as_table().unwrap().get(&self.active_secondnode).unwrap();
			if tabl.is_table(){
				let tab = tabl.as_table().unwrap();
				
				if mov == MenuMove::Up || mov == MenuMove::Down {
					match self.get_next_state_key(&mov, tab , &self.active_field ){
						None => {},
						Some(el) => {
							self.active_field = el.to_owned();
							self.terminal = true;
						}
					}
				}
				else if mov == MenuMove::Left{
					self.active_field = "".to_owned();
					self.terminal = false;
				}
			}
		}
	}
}

pub static mut MENU_STATE : Lazy<MenuState> = Lazy::new(|| MenuState::new());

pub fn get_active_config_from_global_state(zone : RenderingZone) -> &'static str{
	unsafe{
		(& *(&raw const  MENU_STATE)).get_active_config(zone)
	}
}

pub fn get_global_menu_state() -> &'static MenuState{
	unsafe{
		& *(&raw const MENU_STATE)
	}
}

pub fn get_global_menu_state_mut() -> &'static mut MenuState{
	unsafe{
		&mut *(&raw mut MENU_STATE)
	}
}

struct MenuRect {
	rect : RECT,
	save_rect : RECT,
	renderer : *mut ID3DXFont
}

impl MenuRect {
	pub fn new(renderer : *mut ID3DXFont) -> Self{
		MenuRect { rect: RECT {left : 0, top : 0, right: 0, bottom: 0},save_rect : RECT {left : 0, top : 0, right: 0, bottom: 0}, renderer }
	}
	
	pub fn new_with_coords(left : i32, top : i32, right : i32, bottom : i32, renderer : *mut ID3DXFont) -> Self{
		MenuRect { rect: NewRect(left, top , right , bottom ), save_rect: NewRect(left, top , right , bottom ), renderer }
	}
	
	pub fn next_row(&mut self) {
		let rect_bor = &mut self.rect;
		UpdateRect(rect_bor, rect_bor.left , rect_bor.bottom , rect_bor.right , rect_bor.bottom + TextSize );
	}
	
	pub fn next_column(&mut self){
		let rect_bor = &mut self.rect;
		UpdateRect(rect_bor, rect_bor.right , rect_bor.top , rect_bor.right + TitleColumnSize , rect_bor.bottom);

	}
	pub fn save(&mut self){
		self.save_rect = self.rect;
	}
	
	pub fn restore(&mut self){
		self.rect = self.save_rect;
	}
	
	fn is_active_zone(&self, rendering : &RenderingZone) -> bool {
		match rendering {
		    RenderingZone::ActiveConfig => {
				get_global_menu_state().get_active_config(RenderingZone::ActiveFirst).is_empty()
			},
		    RenderingZone::ActiveFirst => {
				get_global_menu_state().get_active_config(RenderingZone::ActiveSecond).is_empty()
			},
		    RenderingZone::ActiveSecond =>  {
				get_global_menu_state().get_active_config(RenderingZone::ActiveThird).is_empty()
			},
		    RenderingZone::ActiveThird =>  {
				true
			},
		    RenderingZone::Version => {false},
		}
	}
	
	pub fn draw<'a, S : Into<&'a str>>(&mut self, text : S, align : Align, rendering : RenderingZone) {
		let nulled = CString::new(text.into()).unwrap();
		let active_node =  self.is_active_zone(&rendering) && 	get_global_menu_state().get_active_config(rendering).eq_ignore_ascii_case(nulled.to_str().unwrap());
		let color = if active_node {(10,240,180)} else {(250,240,180)};
		DrawText(self.renderer, &mut self.rect, nulled.as_ptr() as *const i8, color , align);
	}

	
	pub fn draw_opt<'a, S : Into<&'a str>>(&mut self, arg : S, opt : S,  align : Align, rendering : RenderingZone) {
		let it = arg.into().to_string();
		let nulled = CString::new(it.clone() + " = " + opt.into()).unwrap();
		let active_node = self.is_active_zone(&rendering) && get_global_menu_state().get_active_config(rendering).eq_ignore_ascii_case(it.as_str());
		let color = if active_node {(10,240,180)} else {(250,240,180)};
		DrawText(self.renderer, &mut self.rect, nulled.as_ptr() as *const i8, color , align);
	}
}

pub fn WriteVersionString(width: i32, height : i32, string : *const i8){
	let font_render = unsafe {FontRenderer};
	let mut rect = NewRect(0, height - TextSize - 10, width, height + TextSize);
	
	DrawText(font_render , &mut rect, string ,(250,240,180), Align::Center);
}

pub fn RenderHeader() -> MenuRect{

	let mut rect = MenuRect::new_with_coords(PositionX, PositionY, PositionX + TitleColumnSize, PositionY + TextSize, unsafe{ FontRenderer} );
	rect.draw("Oblivion Reloaded - Settings", Align::Center, RenderingZone::Version);
	rect.next_row();
	rect.save();
	rect.draw("Main", Align::Left, RenderingZone::ActiveConfig);	
	rect.next_column();
	rect.draw("Shaders", Align::Left,RenderingZone::ActiveConfig);
	rect.next_column();
	rect.draw("Effects", Align::Left,RenderingZone::ActiveConfig);
	rect.restore();
	rect
}

fn render_reflected_struct(namedField: &NamedField, field: &dyn PartialReflect, zone : RenderingZone, rect : &mut MenuRect){
	match namedField.type_info().unwrap(){
		bevy_reflect::TypeInfo::Struct(_struct_info) => {
			rect.draw(namedField.name(), Align::Left ,zone);
		}
		bevy_reflect::TypeInfo::TupleStruct(tuple_struct_info) => todo!(),
		bevy_reflect::TypeInfo::Tuple(tuple_info) => todo!(),
		bevy_reflect::TypeInfo::List(list_info) => todo!(),
		bevy_reflect::TypeInfo::Array(array_info) => {
			rect.draw_opt(namedField.name(), "<ARRAY>",  Align::Left ,zone);
		}
		bevy_reflect::TypeInfo::Map(map_info) => todo!(),
		bevy_reflect::TypeInfo::Set(set_info) => todo!(),
		bevy_reflect::TypeInfo::Enum(enum_info) => {
			rect.draw_opt(namedField.name(), "<ENUM>",  Align::Left ,zone);
		},
		bevy_reflect::TypeInfo::Opaque(opaque_info) => {
			let id = opaque_info.type_id();
			log(format!("{:?}", namedField));
			let v : String = if id  == TypeId::of::<u32>() {
				field.try_downcast_ref::<u32>().unwrap().to_string()
			}
			else if id  == TypeId::of::<u8>() {
				field.try_downcast_ref::<u8>().unwrap().to_string()
			}
			else if id  == TypeId::of::<u16>() {
				field.try_downcast_ref::<u16>().unwrap().to_string()
			}
			else if id  == TypeId::of::<u64>() {
				field.try_downcast_ref::<u64>().unwrap().to_string()
			}
			else if id  == TypeId::of::<bool>() {
				field.try_downcast_ref::<bool>().unwrap().to_string()
			}
			else if id  == TypeId::of::<f32>() {
				field.try_downcast_ref::<f32>().unwrap().to_string()
			}
			else if id  == TypeId::of::<f64>() {
				field.try_downcast_ref::<f64>().unwrap().to_string()
			}
			else if id  == TypeId::of::<SysString>() {
				field.try_downcast_ref::<SysString>().unwrap().to_string()
			}
			else {
				"<opaque>".to_owned()
			};
			rect.draw_opt(namedField.name(), &v,  Align::Left ,zone);
		}
	}
}

pub fn RenderMenu(width: i32, height : i32){
	let mut rect = RenderHeader();
	let configtable : &dyn Struct  = match get_global_menu_state().get_active_mainconf() {
		MenuSelected::Main => get_static_ref_const::<Config>(&raw const CONFIG) as &dyn Struct,
		MenuSelected::Shaders => get_static_ref_const::<Shaders>(&raw const SHADERS) as &dyn Struct,
		MenuSelected::Effects => get_static_ref_const::<Effects>(&raw const EFFECTS) as &dyn Struct,
	};
	
	rect.next_row();
	rect.save();
	let type_info = configtable.get_represented_type_info().unwrap().as_struct().unwrap();
	for namedField in  type_info.iter() {
		rect.draw(namedField.name(), Align::Left, RenderingZone::ActiveFirst);
		rect.next_row();
	}
	rect.restore();
	rect.next_column();
	rect.save();
	let first = 	get_global_menu_state().get_active_config(RenderingZone::ActiveFirst);
	if !first.is_empty(){
		match configtable.field(first){
			None => {
				log(format!("[ERROR] Configuration Key {} not found", first));
			}
			Some(val) => {
				let stru = val.reflect_ref().as_struct().unwrap();
				let type_first_selected = stru.get_represented_type_info().unwrap().as_struct().unwrap();
				for  (field, namedField) in stru.iter_fields().zip(type_first_selected.iter()) {
					render_reflected_struct(	namedField, field, RenderingZone::ActiveSecond, &mut rect );
					rect.next_row();
				}
			}
		}
	}
	rect.restore();
	rect.next_column();
	rect.save();
	let second = get_global_menu_state().get_active_config(RenderingZone::ActiveSecond);
	if !second.is_empty(){
		match configtable.field(first).unwrap().reflect_ref().as_struct().unwrap().field(second) {
			None => {
				log(format!("[ERROR] Configuration Key {} not found", second));
			}
			Some(val) => {
				match  val.reflect_ref().as_struct() {
					Ok(structure) =>{
						let type_first_selected = structure.get_represented_type_info().unwrap().as_struct().unwrap();
						for  (field, namedField) in structure.iter_fields().zip(type_first_selected.iter()) {
							render_reflected_struct(	namedField, field, RenderingZone::ActiveThird, &mut rect );
							rect.next_row();
						}
					},
					Err(_) => {},
				}
			}
		}
	}
}

#[repr(C)]
#[derive(PartialEq)]
pub enum OperationSetting {
	Add, Sub
}

pub fn ChangeCurrentSetting(op : OperationSetting) -> Option<String> {
	if get_global_menu_state().is_terminal() {
		let conf = get_global_menu_state().get_active_mainconf();
		let configtable  : &mut dyn Struct =  {
			match conf {
				MenuSelected::Main => get_static_ref(&raw mut CONFIG_TABLE),
				MenuSelected::Shaders => get_static_ref(&raw mut SHADERS_TABLE),
				MenuSelected::Effects => get_static_ref(&raw mut EFFECTS_TABLE)
			}
		};
		let first = get_active_config_from_global_state(RenderingZone::ActiveFirst);
		let second = get_active_config_from_global_state(RenderingZone::ActiveSecond);
		let third = get_active_config_from_global_state(RenderingZone::ActiveThird);
		let tabled = configtable.get_mut(first).unwrap().as_table_mut().unwrap().get_mut(second).unwrap();
		let tab : &mut Value = if tabled.is_table(){ tabled.as_table_mut().unwrap().get_mut(third).unwrap() } else {tabled};
		let modified = match tab {
			//TODO, this rely on a custom version of the toml crate with custom Value discriminants. We can implement it directly as it's only a serialization between struct and table with no TOML accessor
			// But for now it seems good enough, not that it break actual TOML serialization and deserialization
		    Value::Integer(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::UInteger(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::Int(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::UInt(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::Short(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::UShort(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::Byte(cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::UByte( cont) => { if op == OperationSetting::Add { *cont = cont.saturating_add(1) } else { *cont = cont.saturating_sub(1) }; true},
		    Value::Float( cont) => {  if op == OperationSetting::Add { *cont += 0.1 } else { *cont -= 0.1 };  true},
		    Value::Float32( cont) => {if op == OperationSetting::Add { *cont += 0.1f32 } else { *cont -= 0.1f32 }; true},
		    Value::Boolean( cont) => { *cont = !*cont; true },
		    _ => {log(format!("{:?}", tab)); false},
		};
		if modified {
			match conf {
				MenuSelected::Main => {
					static_mut_insert(&raw mut CONFIG  ,crate::main_config::Config::deserialize(configtable.clone()).unwrap());
				},
				MenuSelected::Shaders => {
					static_mut_insert(&raw mut SHADERS  ,crate::shader_config::Shaders::deserialize(configtable.clone()).unwrap());
				},
				MenuSelected::Effects => {
					static_mut_insert(&raw mut EFFECTS  ,crate::effect_config::Effects::deserialize(configtable.clone()).unwrap());
				}
			}
			if *conf == MenuSelected::Main {
				if first.eq_ignore_ascii_case("Shaders") || first.eq_ignore_ascii_case("Effects"){
					return Some(second.to_owned());
				}
			}
			return None;
		}
		return None;
	}
	return None;
}
