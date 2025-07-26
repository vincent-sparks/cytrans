use std::collections::HashMap;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Document, Element, Request, RequestInit};
use web_sys::js_sys::{try_iter, Object, Reflect};

fn document() -> Result<Document, JsValue> {
    window().ok_or(JsValue::from_str("window is null"))?.document().ok_or(JsValue::from_str("window has no document attribute"))
}

fn construct_fb_column(document: &Document, title: &str) -> Result<Element, JsValue> {
    let elem = document.create_element("div")?;
    elem.set_class_name("file-list");
    let header = document.create_element("div")?;
    header.set_class_name("fb-header");
    header.set_text_content(Some(title));
    elem.append_child(&header)?;

    Ok(elem)
}

fn get(val: &JsValue, key: impl Into<JsValue>) -> Result<JsValue,JsValue> {
    Reflect::get(val, &key.into())
}

fn set(val: &JsValue, key: impl Into<JsValue>, value: impl Into<JsValue>) -> Result<bool,JsValue> {
    Reflect::set(val, &key.into(), &value.into())
}

static mut BROWSE_CACHE: Option<HashMap<String, Vec<String>>> = None;

async fn get_browse(path: &str) -> Result<&'static [String], JsValue> {
    let browse_cache = unsafe { BROWSE_CACHE.get_or_insert_with(HashMap::new) };
    if let Some(res) = browse_cache.get(path)  {
        return Ok(res);
    }
    let req = Request::new_with_str(&format!("/api/browse?path={path}"))?;

    todo!()
}

async fn draw_browse(level: u32, path: &str) -> Result<(), JsValue> {
    let window = window().expect("no global window");
    let document = window.document().expect("window has no document attribute");
    let (_, s) = path.rsplit_once('/').unwrap_or(("",""));
    let toplevel = Some(s).filter(|s| !s.is_empty());

    let container = document.get_element_by_id("file-browser").expect("No #file-browser root found in HTML");
    let children = container.children();
    while let Some(elem) = children.item(level) {
        elem.remove();
    }
    // set the previous level's highlighted item to the one the user clicked on
    if level != 0 {
        if let Some(elem) = children.item(level - 1) {
            if let Some(s) = toplevel {
                let s = format!("{s}/");
                for child in try_iter(&elem.children())?.expect("HTMLCollection should be iterable") {
                    let child = Element::unchecked_from_js(child?);
                    child.class_list().toggle_with_force("fb-selected", child.text_content().is_some_and(|content| content == s))?;
                }
            }
        }
    }
    container.append_child(&construct_fb_column(&document, "Select A Folder")?.into())?;

    let headers = Object::new();
    set(&headers, "Accept", "text/xff-delimited")?;
    let request_init = RequestInit::new();
    request_init.set_headers(&headers);
    //let response = window.fetch_with_str_and_init(&format!("/api/browse?path={}", path), &request_init);
    //let response = JsFuture::from(response).await?;




    Ok(())
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    draw_browse(0,"/").await
}

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a+b*2
}//-*-*<kOrigin>
