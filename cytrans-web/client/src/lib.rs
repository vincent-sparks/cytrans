use wasm_bindgen::prelude::*;
use web_sys::{window, Document, Element};

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

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let document = window().expect("no global window")
        .document().expect("window has no document attribute");

    let container = document.get_element_by_id("file-browser").expect("No #file-browser root found in HTML");
    
    container.append_child(&construct_fb_column(&document, "Select A Folder")?.into())?;

    Ok(())
}

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a+b*2
}
